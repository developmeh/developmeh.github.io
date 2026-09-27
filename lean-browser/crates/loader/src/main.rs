//! `lean-loader`: per-navigation process that fetches, parses, cascades and
//! freezes a page into a page file, then exits (plan §2).
//!
//! M0 skeleton: only `--write-empty` is implemented. The broker/worker
//! split, HTML parsing, cascade and compression land in M1–M3.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use page_format::Page;

const USAGE: &str = "\
usage: lean-loader --write-empty <page.lpg> [--url <url>] [--viewport-width <px>]

  --write-empty <path>     write a minimal valid page file (root node only)
  --url <url>              final_url recorded in the page (default about:blank)
  --viewport-width <px>    cascade bucket recorded in the page (default 1280)
";

fn main() -> ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut url = String::from("about:blank");
    let mut viewport_width: u16 = 1280;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--write-empty" => out = args.next().map(PathBuf::from),
            "--url" => url = args.next().unwrap_or_default(),
            "--viewport-width" => {
                viewport_width = match args.next().and_then(|v| v.parse().ok()) {
                    Some(v) => v,
                    None => return usage_error("--viewport-width needs a number"),
                }
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => return usage_error(&format!("unknown argument {other}")),
        }
    }

    let Some(out) = out else {
        return usage_error("--write-empty is required");
    };
    let page = Page::empty(url, viewport_width);
    match page_format::write_to_path(&page, 0, &out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lean-loader: {}: {e}", out.display());
            ExitCode::FAILURE
        }
    }
}

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("lean-loader: {msg}\n{USAGE}");
    ExitCode::from(2)
}
