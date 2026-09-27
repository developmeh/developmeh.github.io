//! `lean-measure`: launches the renderer headless, samples `smaps_rollup`
//! and reports `Private_Dirty` (plan §9).
//!
//! M0 skeleton: `--self` demonstrates the sampler on this process; the
//! renderer launch, settle window and JSON report are M0 harness work.

#![forbid(unsafe_code)]

use std::process::ExitCode;

const USAGE: &str = "\
usage: lean-measure --self
       lean-measure --pid <pid>

  --self        print this process's smaps_rollup figures
  --pid <pid>   print another process's figures
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pid = match args.as_slice() {
        [a] if a == "--self" => std::process::id(),
        [a, p] if a == "--pid" => match p.parse() {
            Ok(pid) => pid,
            Err(_) => return usage(),
        },
        _ => return usage(),
    };
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

fn usage() -> ExitCode {
    eprint!("{USAGE}");
    ExitCode::from(2)
}
