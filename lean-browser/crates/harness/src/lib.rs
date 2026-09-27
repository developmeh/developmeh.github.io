//! Measurement helpers shared by `lean-measure` and `lean-diff` (plan §9).
//!
//! M0 skeleton: the `smaps_rollup` parser. Sampling, SSIM and the CI
//! report follow in M0's harness work.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The memory figures the harness gates on, in kilobytes as the kernel
/// reports them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SmapsRollup {
    /// Resident set size.
    pub rss_kb: u64,
    /// Anonymous or copy-on-write pages this process alone dirtied: the
    /// single optimizing target.
    pub private_dirty_kb: u64,
    /// Dirty pages shared with another mapping (e.g. the wl_shm buffer).
    pub shared_dirty_kb: u64,
    /// Clean private pages (e.g. the page file mapping after a read).
    pub private_clean_kb: u64,
}

/// Parses the text of `/proc/<pid>/smaps_rollup`.
pub fn parse_smaps_rollup(text: &str) -> SmapsRollup {
    let mut out = SmapsRollup::default();
    for line in text.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let kb = rest
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        match key {
            "Rss" => out.rss_kb = kb,
            "Private_Dirty" => out.private_dirty_kb = kb,
            "Shared_Dirty" => out.shared_dirty_kb = kb,
            "Private_Clean" => out.private_clean_kb = kb,
            _ => {}
        }
    }
    out
}

/// Reads and parses `/proc/<pid>/smaps_rollup`.
pub fn read_smaps_rollup(pid: u32) -> std::io::Result<SmapsRollup> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup"))?;
    Ok(parse_smaps_rollup(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kernel_format() {
        let text = "\
00400000-7fff9c9d2000 ---p 00000000 00:00 0                              [rollup]
Rss:                1234 kB
Pss:                 900 kB
Shared_Clean:        300 kB
Shared_Dirty:         64 kB
Private_Clean:       200 kB
Private_Dirty:       670 kB
Referenced:         1234 kB
Anonymous:           700 kB
";
        assert_eq!(
            parse_smaps_rollup(text),
            SmapsRollup {
                rss_kb: 1234,
                private_dirty_kb: 670,
                shared_dirty_kb: 64,
                private_clean_kb: 200,
            }
        );
    }

    #[test]
    fn reads_own_process() {
        let s = read_smaps_rollup(std::process::id()).unwrap();
        assert!(s.rss_kb > 0);
        assert!(s.private_dirty_kb > 0);
    }
}
