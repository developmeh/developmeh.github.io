//! Running the pipeline on the corpus (plan §9): loader, then the renderer
//! in `--serve` mode painting top / 50 % / bottom, then `smaps_rollup`
//! samples while the renderer is still alive, then the `lean-alloc` tags.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::diff::diff_files;
use crate::report::{
    median, AllocTag, LoaderStats, PageResult, PaintResult, PassStat, RendererStats, Report,
    TARGET_PRIVATE_DIRTY_KB,
};
use crate::{read_smaps_rollup, SmapsRollup};

/// The three scroll positions of plan §9.
pub const POSITIONS: [(&str, f32); 3] = [("top", 0.0), ("mid", 0.5), ("bottom", 1.0)];

/// `corpus/manifest.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    /// Pages in measurement order.
    pub pages: Vec<ManifestPage>,
}

/// One corpus entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestPage {
    /// Short name; also the stem of the PNGs and references.
    pub name: String,
    /// HTML file relative to the corpus directory.
    pub file: String,
    /// What the page exercises.
    #[serde(default)]
    pub description: String,
    /// Gate for the renderer's `Private_Dirty` median in kB (plan §8).
    #[serde(default)]
    pub target_private_dirty_kb: Option<u64>,
}

impl Manifest {
    /// Reads and parses a manifest file.
    pub fn read(path: &Path) -> Result<Manifest, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// How to run the measurement.
#[derive(Clone, Debug)]
pub struct Config {
    /// `lean-loader` binary.
    pub loader: PathBuf,
    /// `lean-browser` binary.
    pub renderer: PathBuf,
    /// Corpus directory containing `manifest.json`.
    pub corpus: PathBuf,
    /// Where page files, PNGs and the report go.
    pub out: PathBuf,
    /// Directory of Chromium reference PNGs (`<name>-<position>.png`).
    pub refs: Option<PathBuf>,
    /// Viewport in CSS px.
    pub viewport: (u32, u32),
    /// `smaps_rollup` samples after the last paint.
    pub samples: usize,
    /// Window over which the samples are spread.
    pub settle_ms: u64,
    /// `--font-dir` for the renderer (`LEAN_FONT_DIR` otherwise).
    pub font_dir: Option<PathBuf>,
    /// Only measure the pages with these names (all when empty).
    pub only: Vec<String>,
    /// Also measure the empty-page baseline.
    pub baseline: bool,
}

impl Config {
    /// `WxH`.
    pub fn viewport_arg(&self) -> String {
        format!("{}x{}", self.viewport.0, self.viewport.1)
    }
}

/// Measures the whole corpus.
pub fn measure_corpus(cfg: &Config) -> Result<Report, String> {
    let manifest = Manifest::read(&cfg.corpus.join("manifest.json"))?;
    std::fs::create_dir_all(&cfg.out).map_err(|e| format!("{}: {e}", cfg.out.display()))?;
    let baseline = if cfg.baseline {
        Some(measure_baseline(cfg))
    } else {
        None
    };
    let mut pages = Vec::new();
    for entry in &manifest.pages {
        if !cfg.only.is_empty() && !cfg.only.iter().any(|n| n == &entry.name) {
            continue;
        }
        eprintln!("lean-measure: {}", entry.name);
        pages.push(measure_page(cfg, entry));
    }
    let profile = if cfg.renderer.to_string_lossy().contains("release") {
        "release"
    } else {
        "debug"
    };
    Ok(Report {
        harness_version: env!("CARGO_PKG_VERSION").to_string(),
        corpus: cfg.corpus.display().to_string(),
        viewport: cfg.viewport_arg(),
        samples: cfg.samples,
        settle_ms: cfg.settle_ms,
        profile: profile.to_string(),
        loader_bin: cfg.loader.display().to_string(),
        renderer_bin: cfg.renderer.display().to_string(),
        baseline,
        pages,
    })
}

/// The empty renderer (plan M0 exit criterion): `lean-loader --write-empty`
/// then the renderer painting it.
pub fn measure_baseline(cfg: &Config) -> PageResult {
    eprintln!("lean-measure: baseline (empty page)");
    let lpg = cfg.out.join("empty.lpg");
    let mut errors = Vec::new();
    let mut loader = LoaderStats::default();
    let started = Instant::now();
    match Command::new(&cfg.loader)
        .arg("--write-empty")
        .arg(&lpg)
        .arg("--viewport-width")
        .arg(cfg.viewport.0.to_string())
        .status()
    {
        Ok(st) => loader.exit_code = st.code().unwrap_or(-1),
        Err(e) => {
            loader.exit_code = -1;
            errors.push(format!("spawn loader: {e}"));
        }
    }
    loader.wall_ms = started.elapsed().as_millis() as u64;
    loader.page_file_bytes = std::fs::metadata(&lpg).ok().map(|m| m.len());
    let renderer = if loader.exit_code == 0 {
        run_renderer(cfg, "empty", &lpg, &mut errors)
    } else {
        RendererStats::default()
    };
    PageResult {
        name: "empty".into(),
        url: "about:blank".into(),
        target_private_dirty_kb: 1024,
        loader,
        renderer,
        errors,
    }
}

/// Measures one corpus page.
pub fn measure_page(cfg: &Config, entry: &ManifestPage) -> PageResult {
    let mut errors = Vec::new();
    let html = cfg.corpus.join(&entry.file);
    let url = match std::fs::canonicalize(&html) {
        Ok(p) => format!("file://{}", p.display()),
        Err(e) => {
            errors.push(format!("{}: {e}", html.display()));
            format!("file://{}", html.display())
        }
    };
    let lpg = cfg.out.join(format!("{}.lpg", entry.name));
    let loader = run_loader(cfg, &url, &lpg, &mut errors);
    let renderer = if loader.exit_code == 0 {
        run_renderer(cfg, &entry.name, &lpg, &mut errors)
    } else {
        RendererStats::default()
    };
    PageResult {
        name: entry.name.clone(),
        url,
        target_private_dirty_kb: entry
            .target_private_dirty_kb
            .unwrap_or(TARGET_PRIVATE_DIRTY_KB),
        loader,
        renderer,
        errors,
    }
}

/// Runs the loader and parses its `--stats` output. Peak RSS comes from
/// the loader's own `VmHWM` line; `/proc/<pid>/status` is also polled while
/// it runs as an independent lower bound.
pub fn run_loader(cfg: &Config, url: &str, lpg: &Path, errors: &mut Vec<String>) -> LoaderStats {
    let mut stats = LoaderStats::default();
    let started = Instant::now();
    let child = Command::new(&cfg.loader)
        .arg(url)
        .arg("--out")
        .arg(lpg)
        .arg("--viewport")
        .arg(cfg.viewport_arg())
        .arg("--stats")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            stats.exit_code = -1;
            errors.push(format!("spawn loader: {e}"));
            return stats;
        }
    };
    let stdout = child
        .stdout
        .take()
        .map(|s| std::thread::spawn(move || read_all(s)));
    let stderr = child
        .stderr
        .take()
        .map(|s| std::thread::spawn(move || read_all(s)));
    let mut polled_peak = 0u64;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Ok(st),
            Ok(None) => {
                if let Some(kb) = vm_hwm_kb(child.id()) {
                    polled_peak = polled_peak.max(kb);
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(e) => break Err(e),
        }
    };
    stats.wall_ms = started.elapsed().as_millis() as u64;
    stats.polled_peak_rss_kb = (polled_peak > 0).then_some(polled_peak);
    let out = stdout.and_then(|t| t.join().ok()).unwrap_or_default();
    let err = stderr.and_then(|t| t.join().ok()).unwrap_or_default();
    match status {
        Ok(st) => stats.exit_code = st.code().unwrap_or(-1),
        Err(e) => {
            stats.exit_code = -1;
            errors.push(format!("wait loader: {e}"));
        }
    }
    if stats.exit_code != 0 {
        errors.push(format!(
            "loader exited with {}: {}",
            stats.exit_code,
            err.trim()
        ));
    }
    parse_loader_stats(&out, &mut stats);
    stats.page_file_bytes = std::fs::metadata(lpg).ok().map(|m| m.len());
    stats
}

/// Parses the text `lean-loader --stats` prints.
pub fn parse_loader_stats(text: &str, stats: &mut LoaderStats) {
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("frozen nodes:") {
            stats.frozen_nodes = v.trim().parse().ok();
        } else if let Some(v) = line.strip_prefix("peak_rss_kb:") {
            stats.peak_rss_kb = v.trim().parse().ok();
        } else if let Some(v) = line.strip_prefix("fetched:") {
            stats.fetched_bytes = v.split_whitespace().next().and_then(|n| n.parse().ok());
        } else if let Some(v) = line.strip_prefix("page:") {
            // "page: 126 nodes, 69 styles, ..."
            let mut it = v.split(',');
            stats.page_nodes = it.next().and_then(first_number);
            stats.page_styles = it.next().and_then(first_number);
        } else if line.starts_with("warning: page truncated") {
            stats.truncated = true;
        } else if let Some(v) = line.strip_prefix("pass ") {
            // "pass 7 trim attributes      37 ->      15 attrs (59.5% removed)"
            let Some((left, right)) = v.split_once("->") else {
                continue;
            };
            let mut left: Vec<&str> = left.split_whitespace().collect();
            let mut right = right.split_whitespace();
            let (Some(number), Some(before)) = (left.first(), left.last()) else {
                continue;
            };
            let (Ok(number), Ok(before)) = (number.parse::<u8>(), before.parse::<usize>()) else {
                continue;
            };
            let (Some(after), Some(unit)) = (right.next(), right.next()) else {
                continue;
            };
            let Ok(after) = after.parse::<usize>() else {
                continue;
            };
            left.remove(0);
            left.pop();
            stats.passes.push(PassStat {
                number,
                name: left.join(" "),
                unit: unit.to_string(),
                before,
                after,
            });
        }
    }
}

fn first_number(s: &str) -> Option<usize> {
    s.split_whitespace().next().and_then(|n| n.parse().ok())
}

/// Drives the renderer's `--serve` loop: three paints, then the samples,
/// then the allocator stats, then `quit`.
pub fn run_renderer(
    cfg: &Config,
    name: &str,
    lpg: &Path,
    errors: &mut Vec<String>,
) -> RendererStats {
    let mut stats = RendererStats::default();
    let mut cmd = Command::new(&cfg.renderer);
    cmd.arg(lpg)
        .arg("--serve")
        .arg("--viewport")
        .arg(cfg.viewport_arg())
        .arg("--dpr")
        .arg("1");
    if let Some(d) = &cfg.font_dir {
        cmd.arg("--font-dir").arg(d);
    }
    let started = Instant::now();
    let mut child = match cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            stats.exit_code = -1;
            errors.push(format!("spawn renderer: {e}"));
            return stats;
        }
    };
    let stderr = child
        .stderr
        .take()
        .map(|s| std::thread::spawn(move || read_all(s)));
    let outcome = drive(cfg, name, &mut child, &mut stats, errors);
    let _ = child.stdin.take();
    let status = child.wait();
    let err = stderr.and_then(|t| t.join().ok()).unwrap_or_default();
    match status {
        Ok(st) => stats.exit_code = st.code().unwrap_or(-1),
        Err(e) => {
            stats.exit_code = -1;
            errors.push(format!("wait renderer: {e}"));
        }
    }
    if let Err(e) = outcome {
        errors.push(format!("renderer: {e}; stderr: {}", err.trim()));
    } else if stats.exit_code != 0 {
        errors.push(format!(
            "renderer exited with {}: {}",
            stats.exit_code,
            err.trim()
        ));
    }
    if stats.paint_ms == 0 {
        stats.paint_ms = started.elapsed().as_millis() as u64;
    }
    stats
}

fn drive(
    cfg: &Config,
    name: &str,
    child: &mut Child,
    stats: &mut RendererStats,
    errors: &mut Vec<String>,
) -> Result<(), String> {
    let started = Instant::now();
    let pid = child.id();
    let mut stdin = child.stdin.take().ok_or("renderer stdin")?;
    let mut stdout = BufReader::new(child.stdout.take().ok_or("renderer stdout")?);
    let mut line = String::new();
    let expect = |stdout: &mut BufReader<_>, line: &mut String| -> Result<String, String> {
        line.clear();
        if stdout.read_line(line).map_err(|e| e.to_string())? == 0 {
            return Err("renderer closed its stdout".into());
        }
        let l = line.trim().to_string();
        if let Some(e) = l.strip_prefix("err ") {
            return Err(e.to_string());
        }
        Ok(l)
    };
    expect(&mut stdout, &mut line)?;
    for (pos, frac) in POSITIONS {
        let png = cfg.out.join(format!("{name}-{pos}.png"));
        writeln!(stdin, "paint {frac} {}", png.display()).map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;
        let reply = expect(&mut stdout, &mut line);
        let mut paint = PaintResult {
            position: pos.to_string(),
            fraction: frac,
            png: png.display().to_string(),
            reference: None,
            diff: None,
        };
        match reply {
            Ok(_) => {
                if let Some(refs) = &cfg.refs {
                    let r = refs.join(format!("{name}-{pos}.png"));
                    if r.is_file() {
                        paint.reference = Some(r.display().to_string());
                        match diff_files(&png, &r) {
                            Ok(d) => paint.diff = Some(d),
                            Err(e) => errors.push(format!("diff {pos}: {e}")),
                        }
                    }
                }
            }
            Err(e) => errors.push(format!("paint {pos}: {e}")),
        }
        stats.paints.push(paint);
    }
    stats.paint_ms = started.elapsed().as_millis() as u64;

    // Plan §9: N samples spread over the settle window after the last paint.
    let gap = Duration::from_millis(cfg.settle_ms / cfg.samples.max(1) as u64);
    for i in 0..cfg.samples {
        if i > 0 {
            std::thread::sleep(gap);
        }
        match read_smaps_rollup(pid) {
            Ok(s) => stats.samples.push(s),
            Err(e) => errors.push(format!("smaps_rollup: {e}")),
        }
    }
    summarize(stats);

    writeln!(stdin, "stats").map_err(|e| e.to_string())?;
    stdin.flush().map_err(|e| e.to_string())?;
    loop {
        let l = expect(&mut stdout, &mut line)?;
        if l == "ok stats" {
            break;
        }
        let parts: Vec<&str> = l.split_whitespace().collect();
        if let ["stat", tag, live, peak] = parts.as_slice() {
            stats.alloc.push(AllocTag {
                tag: (*tag).to_string(),
                live: live.parse().unwrap_or(0),
                peak: peak.parse().unwrap_or(0),
            });
        }
    }
    writeln!(stdin, "quit").map_err(|e| e.to_string())?;
    stdin.flush().map_err(|e| e.to_string())?;
    Ok(())
}

/// Fills the max/median fields from the samples.
pub fn summarize(stats: &mut RendererStats) {
    let pick = |f: fn(&SmapsRollup) -> u64| -> (u64, u64) {
        let v: Vec<u64> = stats.samples.iter().map(f).collect();
        (
            v.iter().copied().max().unwrap_or(0),
            median(&v).unwrap_or(0),
        )
    };
    (stats.private_dirty_max_kb, stats.private_dirty_median_kb) = pick(|s| s.private_dirty_kb);
    (stats.shared_dirty_max_kb, stats.shared_dirty_median_kb) = pick(|s| s.shared_dirty_kb);
    (stats.rss_max_kb, stats.rss_median_kb) = pick(|s| s.rss_kb);
    stats.anon_huge_max_kb = pick(|s| s.anon_huge_kb).0;
}

/// `VmHWM` of a running process, kB.
pub fn vm_hwm_kb(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok())
}

fn read_all<R: std::io::Read>(mut r: R) -> String {
    let mut s = String::new();
    let _ = r.read_to_string(&mut s);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_loader_stats_text() {
        let text = "\
frozen nodes: 179
pass 1 drop never-rendered             179 ->     150 nodes (16.2% removed)
pass 5 intern styles                   126 ->      69 styles (45.2% removed)
pass 7 trim attributes                  37 ->      15 attrs (59.5% removed)
page: 126 nodes, 69 styles, 15 attrs, 2 images (78 blob bytes), 0 forms, 890 text bytes, 7 top-level blocks, breakpoints [720]
fetched: 4659 bytes
peak_rss_kb: 14608
";
        let mut s = LoaderStats::default();
        parse_loader_stats(text, &mut s);
        assert_eq!(s.frozen_nodes, Some(179));
        assert_eq!(s.page_nodes, Some(126));
        assert_eq!(s.page_styles, Some(69));
        assert_eq!(s.fetched_bytes, Some(4659));
        assert_eq!(s.peak_rss_kb, Some(14608));
        assert_eq!(s.passes.len(), 3);
        assert_eq!(s.passes[0].name, "drop never-rendered");
        assert_eq!(s.passes[1].unit, "styles");
        assert_eq!(s.passes[2].before, 37);
        assert_eq!(s.passes[2].after, 15);
        assert!((s.node_reduction_pct().unwrap() - 29.6).abs() < 0.1);
    }

    #[test]
    fn summarizes_samples() {
        let mut r = RendererStats::default();
        for kb in [500u64, 700, 600] {
            r.samples.push(SmapsRollup {
                rss_kb: kb * 2,
                private_dirty_kb: kb,
                shared_dirty_kb: 1,
                private_clean_kb: 0,
                anon_huge_kb: 0,
            });
        }
        summarize(&mut r);
        assert_eq!(r.private_dirty_max_kb, 700);
        assert_eq!(r.private_dirty_median_kb, 600);
        assert_eq!(r.rss_max_kb, 1400);
    }

    #[test]
    fn manifest_round_trip() {
        let m: Manifest = serde_json::from_str(
            r#"{"pages":[{"name":"a","file":"a.html"},{"name":"b","file":"b.html","target_private_dirty_kb":4096,"description":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(m.pages.len(), 2);
        assert_eq!(m.pages[0].target_private_dirty_kb, None);
        assert_eq!(m.pages[1].target_private_dirty_kb, Some(4096));
    }
}
