//! `lean-loader`: per-navigation process that fetches, parses, cascades,
//! compresses and freezes a page into a page file, then exits (plan §2).
//!
//! M0–M2 scope: document mode over `file://`, `http(s)://` and `data:`;
//! no JS, no broker/worker sandbox, no IPC yet (those are M3/M5).

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use loader::fetch::{url_from_arg, Limits};
use loader::{LoadOptions, LoadResult};
use page_format::Page;

const USAGE: &str = "\
usage: lean-loader <url-or-path> --out <page.lpg> [--viewport WxH] [--stats] [--check]
       lean-loader --write-empty <page.lpg> [--url <url>] [--viewport-width <px>]

  <url-or-path>            http(s):// or file:// URL, or a filesystem path
  --out <path>             page file to write (required)
  --viewport WxH           cascade bucket and viewport height (default 1280x800)
  --stats                  print node counts before/after each compression pass
  --check                  verify compression is style-preserving (always on in debug builds)
  --write-empty <path>     write a minimal valid page file (root node only)
";

fn main() -> ExitCode {
    let mut input: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut write_empty: Option<PathBuf> = None;
    let mut url = String::from("about:blank");
    let mut viewport: (u16, u16) = (1280, 800);
    let mut stats = false;
    let mut check = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = args.next().map(PathBuf::from),
            "--write-empty" => write_empty = args.next().map(PathBuf::from),
            "--url" => url = args.next().unwrap_or_default(),
            "--viewport" => {
                viewport = match args.next().and_then(|v| parse_viewport(&v)) {
                    Some(v) => v,
                    None => return usage_error("--viewport needs WxH, e.g. 1280x800"),
                }
            }
            "--viewport-width" => {
                viewport.0 = match args.next().and_then(|v| v.parse().ok()) {
                    Some(v) => v,
                    None => return usage_error("--viewport-width needs a number"),
                }
            }
            "--stats" => stats = true,
            "--check" => check = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                return usage_error(&format!("unknown argument {other}"))
            }
            other => {
                if input.is_some() {
                    return usage_error("only one input is accepted");
                }
                input = Some(other.to_string());
            }
        }
    }

    if let Some(path) = write_empty {
        let page = Page::empty(url, viewport.0);
        return write(&page, 0, &path);
    }

    let Some(input) = input else {
        return usage_error("an input URL or path is required");
    };
    let Some(out) = out else {
        return usage_error("--out is required");
    };
    let url = match url_from_arg(&input) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("lean-loader: {e}");
            return ExitCode::from(2);
        }
    };
    let opts = LoadOptions {
        viewport,
        limits: Limits::default(),
        check,
    };
    let result = match loader::load(&url, &opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("lean-loader: {e}");
            return ExitCode::FAILURE;
        }
    };
    if stats {
        print_stats(&result);
    }
    write(&result.page, result.flags, &out)
}

fn print_stats(r: &LoadResult) {
    println!("frozen nodes: {}", r.frozen_nodes);
    for s in &r.stats {
        let pct = if s.before > 0 {
            100.0 * (s.before as f64 - s.after as f64) / s.before as f64
        } else {
            0.0
        };
        println!(
            "pass {} {:<30} {:>7} -> {:>7} {} ({pct:.1}% removed)",
            s.number, s.name, s.before, s.after, s.unit
        );
    }
    let p = &r.page;
    println!(
        "page: {} nodes, {} styles, {} attrs, {} images ({} blob bytes), {} forms, {} text bytes, {} top-level blocks, breakpoints {:?}",
        p.nodes.len(),
        p.styles.len(),
        p.attrs.len(),
        p.images.len(),
        p.blobs.len(),
        p.forms.len(),
        p.text.len(),
        p.top_level.len(),
        p.breakpoints
    );
    println!("fetched: {} bytes", r.fetched_bytes);
    // Peak RSS of this process (plan §9 wants the loader's peak via wait4
    // rusage; rustix 1.x has no getrusage and libc is not a direct
    // dependency, so the loader reports its own high-water mark instead).
    if let Some(kb) = peak_rss_kb() {
        println!("peak_rss_kb: {kb}");
    }
    if r.flags & loader::FLAG_TRUNCATED != 0 {
        println!("warning: page truncated (limits hit)");
    }
}

/// `VmHWM` from `/proc/self/status`, in kB.
fn peak_rss_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok())
}

fn write(page: &Page, flags: u32, path: &std::path::Path) -> ExitCode {
    match page_format::write_to_path(page, flags, path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lean-loader: {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}

fn parse_viewport(s: &str) -> Option<(u16, u16)> {
    let (w, h) = s.split_once(['x', 'X'])?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("lean-loader: {msg}\n{USAGE}");
    ExitCode::from(2)
}
