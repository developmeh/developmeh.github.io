//! `lean-measure`: runs the loader and the headless renderer on every corpus
//! page, samples `smaps_rollup` while the renderer is alive and writes a
//! JSON report plus a Markdown table (plan §9).

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use harness::measure::{measure_corpus, Config};

const USAGE: &str = "\
usage: lean-measure --corpus <dir> [options]
       lean-measure --self | --pid <pid>

  --corpus <dir>        directory with manifest.json and the HTML pages
  --out <dir>           output directory (default: <corpus>/../target/measure)
  --loader <bin>        lean-loader binary (default: next to this executable)
  --renderer <bin>      lean-browser binary (default: next to this executable)
  --refs <dir>          Chromium reference PNGs, <name>-{top,mid,bottom}.png
                        (default: <corpus>/../refs when it exists)
  --viewport <WxH>      CSS px viewport (default 1280x800)
  --samples <n>         smaps_rollup samples after the last paint (default 10)
  --settle-ms <ms>      window over which the samples are spread (default 5000)
  --font-dir <dir>      passed to the renderer as --font-dir
  --only <name>         measure only this page (repeatable)
  --no-baseline         skip the empty-page baseline
  --gate                exit 1 when a plan §8 gate fails
  --self / --pid <pid>  print one process's smaps_rollup figures and exit
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [a] if a == "--self" => return print_pid(std::process::id()),
        [a, p] if a == "--pid" => {
            return match p.parse() {
                Ok(pid) => print_pid(pid),
                Err(_) => usage("--pid needs a number"),
            }
        }
        _ => {}
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let mut corpus: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut loader = exe_dir.join("lean-loader");
    let mut renderer = exe_dir.join("lean-browser");
    let mut refs: Option<PathBuf> = None;
    let mut viewport = (1280u32, 800u32);
    let mut samples = 10usize;
    let mut settle_ms = 5000u64;
    let mut font_dir = None;
    let mut only = Vec::new();
    let mut baseline = true;
    let mut gate = false;

    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or(format!("{arg} needs a value"));
        let r: Result<(), String> = (|| {
            match arg.as_str() {
                "--corpus" => corpus = Some(PathBuf::from(value()?)),
                "--out" => out = Some(PathBuf::from(value()?)),
                "--loader" => loader = PathBuf::from(value()?),
                "--renderer" => renderer = PathBuf::from(value()?),
                "--refs" => refs = Some(PathBuf::from(value()?)),
                "--font-dir" => font_dir = Some(PathBuf::from(value()?)),
                "--only" => only.push(value()?),
                "--no-baseline" => baseline = false,
                "--gate" => gate = true,
                "--samples" => {
                    samples = value()?.parse().map_err(|_| "--samples needs a number")?
                }
                "--settle-ms" => {
                    settle_ms = value()?.parse().map_err(|_| "--settle-ms needs a number")?
                }
                "--viewport" => {
                    let v = value()?;
                    let (w, h) = v.split_once('x').ok_or("--viewport needs WxH")?;
                    viewport = (
                        w.parse().map_err(|_| "bad viewport width")?,
                        h.parse().map_err(|_| "bad viewport height")?,
                    );
                }
                "-h" | "--help" => {
                    print!("{USAGE}");
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument {other}")),
            }
            Ok(())
        })();
        if let Err(e) = r {
            return usage(&e);
        }
    }
    let Some(corpus) = corpus else {
        return usage("--corpus is required");
    };
    let root = corpus.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let refs = refs.or_else(|| {
        let r = root.join("refs");
        r.join("VERSION").is_file().then_some(r)
    });
    let cfg = Config {
        loader,
        renderer,
        out: out.unwrap_or_else(|| root.join("target").join("measure")),
        corpus,
        refs,
        viewport,
        samples: samples.max(1),
        settle_ms,
        font_dir,
        only,
        baseline,
    };
    let report = match measure_corpus(&cfg) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("lean-measure: {e}");
            return ExitCode::FAILURE;
        }
    };
    let json = match serde_json::to_string_pretty(&report) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("lean-measure: {e}");
            return ExitCode::FAILURE;
        }
    };
    let md = report.markdown();
    for (name, text) in [("report.json", json), ("report.md", md.clone())] {
        let path = cfg.out.join(name);
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("lean-measure: {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    }
    print!("{md}");
    eprintln!(
        "lean-measure: wrote {}",
        cfg.out.join("report.{json,md}").display()
    );
    if gate && report.gates().iter().any(|g| !g.pass) {
        eprintln!("lean-measure: a gate failed");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn print_pid(pid: u32) -> ExitCode {
    match harness::read_smaps_rollup(pid) {
        Ok(s) => {
            println!(
                "pid {pid}: rss {} kB, private_dirty {} kB, shared_dirty {} kB, private_clean {} kB",
                s.rss_kb, s.private_dirty_kb, s.shared_dirty_kb, s.private_clean_kb
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("lean-measure: pid {pid}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("lean-measure: {msg}\n{USAGE}");
    ExitCode::from(2)
}
