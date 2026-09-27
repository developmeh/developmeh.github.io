//! Serializing a [`Page`] into a page file.

use std::io::Write;
use std::path::Path;

use rkyv::util::AlignedVec;

use crate::crc32::crc32;
use crate::header::{Header, HEADER_LEN};
use crate::page::Page;
use crate::validate::PageError;

/// Encodes `page` as header + archive. The result is 16-byte aligned so it
/// can be validated in memory without touching disk.
pub fn encode(page: &Page, flags: u32) -> Result<AlignedVec, PageError> {
    let archive = rkyv::to_bytes::<rkyv::rancor::Error>(page)
        .map_err(|e| PageError::Structural(e.to_string()))?;
    let header = Header::new(flags, archive.len() as u64, crc32(&archive));
    let mut out = AlignedVec::with_capacity(HEADER_LEN + archive.len());
    out.extend_from_slice(&header.to_bytes());
    out.extend_from_slice(&archive);
    Ok(out)
}

/// Encodes `page` and writes it atomically-ish (write to `path.tmp`, then
/// rename) so a reader never maps a half-written file.
pub fn write_to_path(page: &Page, flags: u32, path: &Path) -> Result<(), PageError> {
    let bytes = encode(page, flags)?;
    let tmp = path.with_extension("lpg.tmp");
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(PageError::Io)
}
