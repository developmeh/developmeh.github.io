//! Read-only access to a validated page file.

use std::fs::File;
use std::path::Path;

use memmap2::Mmap;
use rkyv::util::AlignedVec;

use crate::header::Header;
use crate::page::ArchivedPage;
use crate::validate::{validate, PageError};

enum Backing {
    /// `MAP_PRIVATE`, `PROT_READ`: file-backed clean pages, never dirty.
    Mapped(Mmap),
    /// In-memory bytes (tests, or a page received without a file).
    Owned(AlignedVec),
}

impl Backing {
    fn bytes(&self) -> &[u8] {
        match self {
            Backing::Mapped(m) => m,
            Backing::Owned(v) => v,
        }
    }
}

/// A page file that passed all four validation steps.
///
/// Dropping it unmaps the file, which is how the renderer releases the
/// previous page on navigation.
pub struct PageFile {
    header: Header,
    backing: Backing,
}

impl PageFile {
    /// Memory-maps and validates the file at `path`.
    pub fn open(path: &Path) -> Result<PageFile, PageError> {
        let file = File::open(path)?;
        // SAFETY: the map is read-only and private. The file could in
        // principle be truncated by another process while mapped, which
        // would raise SIGBUS on access; page files live in the session
        // directory that only the loader writes (via rename), so no writer
        // ever touches a file the renderer has mapped.
        let map = unsafe { Mmap::map(&file)? };
        Self::from_backing(Backing::Mapped(map))
    }

    /// Validates page-file bytes already in memory.
    pub fn from_bytes(bytes: AlignedVec) -> Result<PageFile, PageError> {
        Self::from_backing(Backing::Owned(bytes))
    }

    fn from_backing(backing: Backing) -> Result<PageFile, PageError> {
        let (header, _) = validate(backing.bytes())?;
        Ok(PageFile { header, backing })
    }

    /// The parsed header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// The validated archive root.
    pub fn page(&self) -> &ArchivedPage {
        let archive = &self.backing.bytes()[crate::header::HEADER_LEN..];
        // SAFETY: `validate` ran `rkyv::access` (structural check) on
        // exactly these bytes when the file was opened, the backing is
        // immutable, and the archive offset keeps rkyv's 16-byte alignment.
        // Re-checking on every call would walk the whole file each time.
        unsafe { rkyv::access_unchecked::<ArchivedPage>(archive) }
    }

    /// Total size of the file in bytes.
    pub fn len(&self) -> usize {
        self.backing.bytes().len()
    }

    /// Whether the file is empty (never true for a validated page).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
