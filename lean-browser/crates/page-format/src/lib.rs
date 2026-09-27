//! The Lean Browser page file (`page.lpg`): the contract between the loader
//! (which writes it) and the renderer (which memory-maps it read-only).
//!
//! Layout on disk (plan §4):
//!
//! ```text
//! +----------------------+------------------------------------------+
//! | Header (32 bytes)    | rkyv archive of `Page` (`archive_len`)   |
//! +----------------------+------------------------------------------+
//! ```
//!
//! The header lives outside the archive so version checks happen before
//! rkyv touches anything. Opening a file runs the four validation steps in
//! [`validate`] exactly once; afterwards the renderer indexes the archive
//! without further checks.
//!
//! This crate is one of the two places `unsafe` is allowed (the other is
//! `lean-alloc`), and only for the memory map behind [`PageFile`].

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

mod crc32;
mod header;
mod page;
mod reader;
mod validate;
mod writer;

pub use crc32::crc32;
pub use header::{Header, FORMAT_VERSION, HEADER_LEN, MAGIC, MIN_READER_VERSION, READER_VERSION};
pub use page::{
    ArchivedAttr, ArchivedForm, ArchivedImageRef, ArchivedNode, ArchivedPage, Attr, AttrKey, Form,
    FormMethod, ImageFormat, ImageRef, Node, NodeKind, Page, Role, NONE,
};
pub use reader::PageFile;
pub use validate::{validate, PageError};
pub use writer::{encode, write_to_path};

// Re-export the shared style model so consumers need only one import.
pub use css_subset;
pub use rkyv::util::AlignedVec;
