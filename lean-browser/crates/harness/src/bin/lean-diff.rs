//! `lean-diff`: SSIM and mismatched-pixel percentage between a renderer
//! PNG and a Chromium reference (plan §9).
//!
//! M0 skeleton: only the pixel-mismatch metric; SSIM follows.

#![forbid(unsafe_code)]

use std::process::ExitCode;

/// Per-channel tolerance from plan §9.
const TOLERANCE: u8 = 12;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [a, b] = args.as_slice() else {
        eprintln!("usage: lean-diff <candidate.png> <reference.png>");
        return ExitCode::from(2);
    };
    let load = |p: &str| {
        image::open(p)
            .map(|i| i.into_rgba8())
            .map_err(|e| format!("{p}: {e}"))
    };
    let (ca, cb) = match (load(a), load(b)) {
        (Ok(x), Ok(y)) => (x, y),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("lean-diff: {e}");
            return ExitCode::FAILURE;
        }
    };
    if ca.dimensions() != cb.dimensions() {
        eprintln!(
            "lean-diff: size mismatch {:?} vs {:?}",
            ca.dimensions(),
            cb.dimensions()
        );
        return ExitCode::FAILURE;
    }
    let total = ca.pixels().len();
    let mismatched = ca
        .pixels()
        .zip(cb.pixels())
        .filter(|(p, q)| {
            p.0.iter()
                .zip(q.0.iter())
                .any(|(x, y)| x.abs_diff(*y) > TOLERANCE)
        })
        .count();
    println!(
        "mismatched_pixels_pct {:.3}",
        100.0 * mismatched as f64 / total.max(1) as f64
    );
    ExitCode::SUCCESS
}
