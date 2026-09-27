//! The measurement report (plan §9): one JSON document for CI and a
//! Markdown table for humans, plus the §8 gates.

use serde::{Deserialize, Serialize};

use crate::diff::DiffResult;
use crate::SmapsRollup;

/// Plan §8 M2 gate for a blog-like page, in kB.
pub const TARGET_PRIVATE_DIRTY_KB: u64 = 2048;
/// Plan §8 M2 gate: compression must remove at least this share of nodes
/// (corpus median).
pub const TARGET_NODE_REDUCTION_PCT: f64 = 40.0;
/// Plan §8 M2 gate: median SSIM against Chromium.
pub const TARGET_SSIM_MEDIAN: f64 = 0.85;

/// One compression pass as the loader printed it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PassStat {
    /// Pass number (1..=7).
    pub number: u8,
    /// Pass name.
    pub name: String,
    /// What is counted.
    pub unit: String,
    /// Count before the pass.
    pub before: usize,
    /// Count after the pass.
    pub after: usize,
}

/// What the loader did for one page.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LoaderStats {
    /// Wall-clock time of the loader process.
    pub wall_ms: u64,
    /// Loader exit code.
    pub exit_code: i32,
    /// Peak RSS (`VmHWM`) reported by the loader itself, kB.
    pub peak_rss_kb: Option<u64>,
    /// Peak RSS observed by polling `/proc/<pid>/status` while it ran, kB
    /// (a lower bound: the last stretch before exit may be missed).
    pub polled_peak_rss_kb: Option<u64>,
    /// Nodes in the frozen tree before compression.
    pub frozen_nodes: Option<usize>,
    /// Nodes in the written page.
    pub page_nodes: Option<usize>,
    /// Interned styles in the written page.
    pub page_styles: Option<usize>,
    /// Bytes of source fetched (HTML, CSS, images).
    pub fetched_bytes: Option<usize>,
    /// Size of the page file on disk.
    pub page_file_bytes: Option<u64>,
    /// Per-pass counts.
    pub passes: Vec<PassStat>,
    /// Whether the page was truncated by the §11 limits.
    pub truncated: bool,
}

impl LoaderStats {
    /// Percentage of frozen nodes removed by compression.
    pub fn node_reduction_pct(&self) -> Option<f64> {
        match (self.frozen_nodes, self.page_nodes) {
            (Some(before), Some(after)) if before > 0 => {
                Some(100.0 * (before as f64 - after as f64) / before as f64)
            }
            _ => None,
        }
    }
}

/// A `lean-alloc` tag as the renderer reported it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllocTag {
    /// Tag name.
    pub tag: String,
    /// Bytes live when sampled.
    pub live: usize,
    /// High-water mark in bytes.
    pub peak: usize,
}

/// One paint of the viewport and, when a reference exists, its comparison.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaintResult {
    /// `top`, `mid` or `bottom`.
    pub position: String,
    /// Scroll fraction requested.
    pub fraction: f32,
    /// Path of the painted PNG.
    pub png: String,
    /// Path of the Chromium reference, if one was found.
    pub reference: Option<String>,
    /// Comparison against the reference.
    pub diff: Option<DiffResult>,
}

/// What the renderer did for one page.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RendererStats {
    /// Renderer exit code (0 unless it crashed).
    pub exit_code: i32,
    /// Wall-clock time from spawn to the last paint's reply.
    pub paint_ms: u64,
    /// Every `smaps_rollup` sample after the last paint.
    pub samples: Vec<SmapsRollup>,
    /// Max `Private_Dirty` over the samples, kB.
    pub private_dirty_max_kb: u64,
    /// Median `Private_Dirty`, kB.
    pub private_dirty_median_kb: u64,
    /// Max `Shared_Dirty`, kB.
    pub shared_dirty_max_kb: u64,
    /// Median `Shared_Dirty`, kB.
    pub shared_dirty_median_kb: u64,
    /// Max RSS, kB.
    pub rss_max_kb: u64,
    /// Median RSS, kB.
    pub rss_median_kb: u64,
    /// Max `AnonHugePages`, kB (should be 0: the renderer opts out of THP).
    pub anon_huge_max_kb: u64,
    /// `lean-alloc` per-tag figures after the last paint.
    pub alloc: Vec<AllocTag>,
    /// The paints in order.
    pub paints: Vec<PaintResult>,
}

/// Everything measured for one corpus page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageResult {
    /// Manifest name.
    pub name: String,
    /// Source URL given to the loader.
    pub url: String,
    /// Gate for `Private_Dirty` median, kB.
    pub target_private_dirty_kb: u64,
    /// Loader figures.
    pub loader: LoaderStats,
    /// Renderer figures.
    pub renderer: RendererStats,
    /// Problems that did not stop the measurement.
    pub errors: Vec<String>,
}

/// The whole run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    /// Harness version.
    pub harness_version: String,
    /// Corpus directory measured.
    pub corpus: String,
    /// Viewport `WxH`.
    pub viewport: String,
    /// Samples taken after the last paint.
    pub samples: usize,
    /// Window over which the samples were spread, ms.
    pub settle_ms: u64,
    /// Build profile of the binaries (`release` or `debug`), by path.
    pub profile: String,
    /// Loader binary used.
    pub loader_bin: String,
    /// Renderer binary used.
    pub renderer_bin: String,
    /// The empty-page baseline (plan M0 exit).
    pub baseline: Option<PageResult>,
    /// Per-page results in corpus order.
    pub pages: Vec<PageResult>,
}

/// Median of a sample set (upper median for even counts).
pub fn median<T: Copy + Ord>(values: &[T]) -> Option<T> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort();
    Some(v[v.len() / 2])
}

/// Median of floating-point samples.
pub fn median_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(v[v.len() / 2])
}

/// A §8 gate evaluated over the report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gate {
    /// What is gated.
    pub name: String,
    /// Measured value.
    pub value: Option<f64>,
    /// Required value.
    pub target: f64,
    /// Whether the measured value satisfies the gate (`false` when unmeasured).
    pub pass: bool,
}

impl Report {
    /// The plan's M2 exit gates, corpus-wide.
    pub fn gates(&self) -> Vec<Gate> {
        let mut out = Vec::new();
        let dirty: Vec<u64> = self
            .pages
            .iter()
            .filter(|p| p.renderer.exit_code == 0 && !p.renderer.samples.is_empty())
            .map(|p| p.renderer.private_dirty_median_kb)
            .collect();
        let dirty_median = median(&dirty).map(|v| v as f64);
        out.push(Gate {
            name: "renderer Private_Dirty median (kB)".into(),
            value: dirty_median,
            target: TARGET_PRIVATE_DIRTY_KB as f64,
            pass: dirty_median.is_some_and(|v| v <= TARGET_PRIVATE_DIRTY_KB as f64),
        });
        let worst = self
            .pages
            .iter()
            .filter(|p| p.renderer.exit_code == 0 && !p.renderer.samples.is_empty())
            .map(|p| p.renderer.private_dirty_max_kb as f64 - p.target_private_dirty_kb as f64)
            .fold(None, |acc: Option<f64>, v| {
                Some(acc.map_or(v, |a| a.max(v)))
            });
        out.push(Gate {
            name: "worst page: Private_Dirty max minus its target (kB)".into(),
            value: worst,
            target: 0.0,
            pass: worst.is_some_and(|v| v <= 0.0),
        });
        let reductions: Vec<f64> = self
            .pages
            .iter()
            .filter_map(|p| p.loader.node_reduction_pct())
            .collect();
        let red = median_f64(&reductions);
        out.push(Gate {
            name: "compression node reduction median (%)".into(),
            value: red,
            target: TARGET_NODE_REDUCTION_PCT,
            pass: red.is_some_and(|v| v >= TARGET_NODE_REDUCTION_PCT),
        });
        let ssims: Vec<f64> = self
            .pages
            .iter()
            .flat_map(|p| p.renderer.paints.iter())
            .filter_map(|p| p.diff.as_ref().map(|d| d.ssim))
            .collect();
        let ssim = median_f64(&ssims);
        out.push(Gate {
            name: "SSIM vs Chromium median".into(),
            value: ssim,
            target: TARGET_SSIM_MEDIAN,
            pass: ssim.is_some_and(|v| v >= TARGET_SSIM_MEDIAN),
        });
        out
    }

    /// The Markdown summary (one row per page, then the gates).
    pub fn markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!(
            "## lean-measure report\n\nviewport {}, {} samples over {} ms after the last paint, {} binaries.\n\n",
            self.viewport, self.samples, self.settle_ms, self.profile
        ));
        md.push_str("| page | loader ms | loader peak RSS | nodes before → after | reduction | styles | page file | Private_Dirty median / max | Shared_Dirty max | RSS max | THP max | SSIM top/mid/bottom | mismatch % | status |\n");
        md.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
        for p in self.baseline.iter().chain(self.pages.iter()) {
            md.push_str(&page_row(p));
        }
        md.push_str("\n### Gates (plan §8, M2 exit)\n\n| gate | measured | target | result |\n|---|---:|---:|---|\n");
        for g in self.gates() {
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                g.name,
                g.value.map_or("n/a".to_string(), |v| format!("{v:.2}")),
                g.target,
                if g.pass { "pass" } else { "FAIL" }
            ));
        }
        md.push_str(
            "\n### Renderer allocations after the last paint (lean-alloc peaks, bytes)\n\n",
        );
        let tags: Vec<String> = {
            let mut t: Vec<String> = self
                .baseline
                .iter()
                .chain(self.pages.iter())
                .flat_map(|p| p.renderer.alloc.iter().map(|a| a.tag.clone()))
                .collect();
            t.sort();
            t.dedup();
            t
        };
        md.push_str(&format!(
            "| page | {} |\n|---|{}\n",
            tags.join(" | "),
            "---:|".repeat(tags.len())
        ));
        for p in self.baseline.iter().chain(self.pages.iter()) {
            let cells: Vec<String> = tags
                .iter()
                .map(|t| {
                    p.renderer
                        .alloc
                        .iter()
                        .find(|a| &a.tag == t)
                        .map_or("-".to_string(), |a| a.peak.to_string())
                })
                .collect();
            md.push_str(&format!("| {} | {} |\n", p.name, cells.join(" | ")));
        }
        md.push_str("\n### Compression passes (count before → after)\n\n");
        for p in &self.pages {
            if p.loader.passes.is_empty() {
                continue;
            }
            let cells: Vec<String> = p
                .loader
                .passes
                .iter()
                .map(|s| format!("{}: {}→{} {}", s.number, s.before, s.after, s.unit))
                .collect();
            md.push_str(&format!("- **{}**: {}\n", p.name, cells.join("; ")));
        }
        let errors: Vec<String> = self
            .baseline
            .iter()
            .chain(self.pages.iter())
            .flat_map(|p| p.errors.iter().map(move |e| format!("- {}: {e}", p.name)))
            .collect();
        if !errors.is_empty() {
            md.push_str("\n### Errors\n\n");
            md.push_str(&errors.join("\n"));
            md.push('\n');
        }
        md
    }
}

fn kb(v: u64) -> String {
    format!("{:.0} kB", v as f64)
}

fn page_row(p: &PageResult) -> String {
    let l = &p.loader;
    let r = &p.renderer;
    let nodes = match (l.frozen_nodes, l.page_nodes) {
        (Some(a), Some(b)) => format!("{a} → {b}"),
        _ => "-".into(),
    };
    let reduction = l
        .node_reduction_pct()
        .map_or("-".to_string(), |v| format!("{v:.1}%"));
    let ssim: Vec<String> = r
        .paints
        .iter()
        .map(|x| {
            x.diff
                .as_ref()
                .map_or("-".to_string(), |d| format!("{:.3}", d.ssim))
        })
        .collect();
    let mismatch: Vec<String> = r
        .paints
        .iter()
        .map(|x| {
            x.diff
                .as_ref()
                .map_or("-".to_string(), |d| format!("{:.1}", d.mismatch_pct))
        })
        .collect();
    let status = if l.exit_code != 0 {
        "loader failed"
    } else if r.exit_code != 0 {
        "renderer failed"
    } else if r.samples.is_empty() {
        "not sampled"
    } else if r.private_dirty_median_kb <= p.target_private_dirty_kb {
        "ok"
    } else {
        "over target"
    };
    format!(
        "| {} | {} | {} | {} | {} | {} | {} | {} / {} | {} | {} | {} | {} | {} | {} |\n",
        p.name,
        l.wall_ms,
        l.peak_rss_kb.map_or("-".to_string(), kb),
        nodes,
        reduction,
        l.page_styles.map_or("-".to_string(), |v| v.to_string()),
        l.page_file_bytes
            .map_or("-".to_string(), |v| format!("{v} B")),
        kb(r.private_dirty_median_kb),
        kb(r.private_dirty_max_kb),
        kb(r.shared_dirty_max_kb),
        kb(r.rss_max_kb),
        kb(r.anon_huge_max_kb),
        if ssim.is_empty() {
            "-".into()
        } else {
            ssim.join(" / ")
        },
        if mismatch.is_empty() {
            "-".into()
        } else {
            mismatch.join(" / ")
        },
        status
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medians() {
        assert_eq!(median(&[3u64, 1, 2]), Some(2));
        assert_eq!(median(&[4u64, 1, 2, 3]), Some(3));
        assert_eq!(median::<u64>(&[]), None);
        assert_eq!(median_f64(&[0.5, 0.9, 0.7]), Some(0.7));
    }

    #[test]
    fn reduction_pct() {
        let l = LoaderStats {
            frozen_nodes: Some(200),
            page_nodes: Some(120),
            ..Default::default()
        };
        assert!((l.node_reduction_pct().unwrap() - 40.0).abs() < 1e-9);
        assert_eq!(LoaderStats::default().node_reduction_pct(), None);
    }

    #[test]
    fn gates_and_markdown_on_empty_report() {
        let r = Report {
            harness_version: "0".into(),
            corpus: "c".into(),
            viewport: "1280x800".into(),
            samples: 0,
            settle_ms: 0,
            profile: "debug".into(),
            loader_bin: "l".into(),
            renderer_bin: "r".into(),
            baseline: None,
            pages: vec![],
        };
        assert!(r.gates().iter().all(|g| !g.pass));
        assert!(r.markdown().contains("FAIL"));
        let json = serde_json::to_string(&r).unwrap();
        let back: Report = serde_json::from_str(&json).unwrap();
        assert_eq!(back.viewport, "1280x800");
    }
}
