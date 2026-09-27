//! The fixed 32-byte header that precedes the rkyv archive.

use crate::validate::PageError;

/// File magic.
pub const MAGIC: [u8; 8] = *b"LEANPG\0\0";
/// Size of the header in bytes; the archive starts at this offset.
pub const HEADER_LEN: usize = 32;

/// Format version written by this crate. Bump on any incompatible change.
pub const FORMAT_VERSION: u16 = 1;
/// Oldest reader that can still read files written by this crate.
pub const MIN_READER_VERSION: u16 = 1;
/// Version of the reader in this crate (compared against a file's
/// `min_reader_version`).
pub const READER_VERSION: u16 = 1;

/// Fixed header, serialized little-endian field by field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Header {
    /// [`MAGIC`].
    pub magic: [u8; 8],
    /// Format version of the archive that follows.
    pub format_version: u16,
    /// Oldest reader version that may read this file.
    pub min_reader_version: u16,
    /// [`Header::FLAG_JS_RAN`], [`Header::FLAG_TRUNCATED`].
    pub flags: u32,
    /// Length in bytes of the rkyv archive following the header.
    pub archive_len: u64,
    /// CRC-32 of the archive bytes.
    pub archive_crc32: u32,
    /// Reserved; written as zero, ignored on read.
    pub reserved: [u8; 4],
}

impl Header {
    /// JavaScript ran in the loader before the page was frozen.
    pub const FLAG_JS_RAN: u32 = 1 << 0;
    /// A loader limit was hit and the page is incomplete.
    pub const FLAG_TRUNCATED: u32 = 1 << 1;
    /// Mask of all flags this version knows.
    pub const KNOWN_FLAGS: u32 = Self::FLAG_JS_RAN | Self::FLAG_TRUNCATED;

    /// A header for an archive of `archive_len` bytes with the given CRC.
    pub fn new(flags: u32, archive_len: u64, archive_crc32: u32) -> Header {
        Header {
            magic: MAGIC,
            format_version: FORMAT_VERSION,
            min_reader_version: MIN_READER_VERSION,
            flags,
            archive_len,
            archive_crc32,
            reserved: [0; 4],
        }
    }

    /// Serializes the header into its on-disk form.
    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        out[0..8].copy_from_slice(&self.magic);
        out[8..10].copy_from_slice(&self.format_version.to_le_bytes());
        out[10..12].copy_from_slice(&self.min_reader_version.to_le_bytes());
        out[12..16].copy_from_slice(&self.flags.to_le_bytes());
        out[16..24].copy_from_slice(&self.archive_len.to_le_bytes());
        out[24..28].copy_from_slice(&self.archive_crc32.to_le_bytes());
        out[28..32].copy_from_slice(&self.reserved);
        out
    }

    /// Parses a header without interpreting it (no version checks).
    pub fn from_bytes(bytes: &[u8]) -> Result<Header, PageError> {
        if bytes.len() < HEADER_LEN {
            return Err(PageError::TooShort);
        }
        let u16_at = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
        let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[0..8]);
        let mut reserved = [0u8; 4];
        reserved.copy_from_slice(&bytes[28..32]);
        Ok(Header {
            magic,
            format_version: u16_at(8),
            min_reader_version: u16_at(10),
            flags: u32_at(12),
            archive_len: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            archive_crc32: u32_at(24),
            reserved,
        })
    }

    /// Validation step 1: magic and versions.
    pub fn check(&self) -> Result<(), PageError> {
        if self.magic != MAGIC {
            return Err(PageError::BadMagic);
        }
        if self.format_version != FORMAT_VERSION {
            return Err(PageError::UnsupportedVersion {
                found: self.format_version,
                supported: FORMAT_VERSION,
            });
        }
        if self.min_reader_version > READER_VERSION {
            return Err(PageError::ReaderTooOld {
                required: self.min_reader_version,
                reader: READER_VERSION,
            });
        }
        Ok(())
    }
}

const _: () = assert!(size_of::<Header>() == HEADER_LEN);
// The archive must start 16-byte aligned for rkyv's access checks.
const _: () = assert!(HEADER_LEN % 16 == 0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let h = Header::new(Header::FLAG_TRUNCATED, 1234, 0xDEADBEEF);
        let bytes = h.to_bytes();
        assert_eq!(&bytes[..8], b"LEANPG\0\0");
        assert_eq!(Header::from_bytes(&bytes).unwrap(), h);
        assert!(h.check().is_ok());
    }

    #[test]
    fn rejects_bad_versions() {
        let mut h = Header::new(0, 0, 0);
        h.format_version = FORMAT_VERSION + 1;
        assert!(matches!(
            h.check(),
            Err(PageError::UnsupportedVersion { .. })
        ));
        let mut h = Header::new(0, 0, 0);
        h.min_reader_version = READER_VERSION + 1;
        assert!(matches!(h.check(), Err(PageError::ReaderTooOld { .. })));
        let mut h = Header::new(0, 0, 0);
        h.magic[0] = b'X';
        assert_eq!(h.check(), Err(PageError::BadMagic));
        assert_eq!(Header::from_bytes(&[0; 31]), Err(PageError::TooShort));
    }
}
