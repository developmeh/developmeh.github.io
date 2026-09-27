//! `lean-diff`: grayscale SSIM (8×8 windows) and mismatched-pixel
//! percentage at a 12/255 tolerance between a renderer PNG and a Chromium
//! reference (plan §9).

#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;

use harness::diff::diff_files;

const USAGE: &str = "\
usage: lean-diff <candidate.png> <reference.png> [--json] [--min-ssim <f>]

  --json            print the result as JSON
  --min-ssim <f>    exit 1 when the SSIM is below f
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut files = Vec::new();
    let mut json = false;
    let mut min_ssim: Option<f64> = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--json" => json = true,
            "--min-ssim" => match it.next().and_then(|v| v.parse().ok()) {
                Some(v) => min_ssim = Some(v),
                None => return usage("--min-ssim needs a number"),
            },
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => return usage(&format!("unknown argument {other}")),
            other => files.push(other.to_string()),
        }
    }
    let [candidate, reference] = files.as_slice() else {
        return usage("two PNG paths are required");
    };
    let result = match diff_files(Path::new(candidate), Path::new(reference)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("lean-diff: {e}");
            return ExitCode::FAILURE;
        }
    };
    if json {
        match serde_json::to_string(&result) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("lean-diff: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        println!("ssim {:.4}", result.ssim);
        println!("mismatched_pixels_pct {:.3}", result.mismatch_pct);
        if !result.same_size() {
            println!(
                "size_mismatch {}x{} vs {}x{} (metrics over the common area)",
                result.candidate.0, result.candidate.1, result.reference.0, result.reference.1
            );
        }
    }
    if min_ssim.is_some_and(|m| result.ssim < m) {
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("lean-diff: {msg}\n{USAGE}");
    ExitCode::from(2)
}
