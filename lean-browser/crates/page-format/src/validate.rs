//! The four validation steps run once at map time (plan §4):
//!
//! 1. header magic / version / `min_reader_version`;
//! 2. CRC-32 of the archive region;
//! 3. rkyv structural check (`rkyv::access` with bytecheck);
//! 4. one linear semantic pass over every table.
//!
//! Any failure rejects the whole page; the renderer never paints a partial
//! page.

use std::fmt;

use css_subset::{NodeFlags, STYLE_NONE};

use crate::crc32::crc32;
use crate::header::{Header, HEADER_LEN};
use crate::page::{ArchivedPage, NONE};

/// Why a page file was rejected.
#[derive(Debug)]
pub enum PageError {
    /// Shorter than the fixed header.
    TooShort,
    /// Wrong magic bytes.
    BadMagic,
    /// `format_version` is not one this crate reads.
    UnsupportedVersion {
        /// Version in the file.
        found: u16,
        /// Version this crate supports.
        supported: u16,
    },
    /// The file demands a newer reader.
    ReaderTooOld {
        /// `min_reader_version` in the file.
        required: u16,
        /// This crate's reader version.
        reader: u16,
    },
    /// `archive_len` does not match the bytes present.
    LengthMismatch {
        /// Length claimed by the header.
        declared: u64,
        /// Bytes actually following the header.
        actual: u64,
    },
    /// The archive's CRC-32 does not match the header.
    CrcMismatch {
        /// CRC in the header.
        expected: u32,
        /// CRC of the bytes.
        actual: u32,
    },
    /// rkyv/bytecheck rejected the archive (or serialization failed).
    Structural(String),
    /// The archive is well-formed but violates a page invariant.
    Semantic(String),
    /// Reading or mapping the file failed.
    Io(std::io::Error),
}

impl PartialEq for PageError {
    fn eq(&self, other: &Self) -> bool {
        use PageError::*;
        match (self, other) {
            (TooShort, TooShort) | (BadMagic, BadMagic) => true,
            (
                UnsupportedVersion {
                    found: a,
                    supported: b,
                },
                UnsupportedVersion {
                    found: c,
                    supported: d,
                },
            ) => a == c && b == d,
            (
                ReaderTooOld {
                    required: a,
                    reader: b,
                },
                ReaderTooOld {
                    required: c,
                    reader: d,
                },
            ) => a == c && b == d,
            (
                LengthMismatch {
                    declared: a,
                    actual: b,
                },
                LengthMismatch {
                    declared: c,
                    actual: d,
                },
            ) => a == c && b == d,
            (
                CrcMismatch {
                    expected: a,
                    actual: b,
                },
                CrcMismatch {
                    expected: c,
                    actual: d,
                },
            ) => a == c && b == d,
            (Structural(a), Structural(b)) | (Semantic(a), Semantic(b)) => a == b,
            (Io(a), Io(b)) => a.kind() == b.kind(),
            _ => false,
        }
    }
}

impl fmt::Display for PageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PageError::TooShort => write!(f, "file is shorter than the page header"),
            PageError::BadMagic => write!(f, "not a page file (bad magic)"),
            PageError::UnsupportedVersion { found, supported } => {
                write!(
                    f,
                    "page format version {found} is not supported (reader supports {supported})"
                )
            }
            PageError::ReaderTooOld { required, reader } => {
                write!(
                    f,
                    "page requires reader version {required}, this reader is {reader}"
                )
            }
            PageError::LengthMismatch { declared, actual } => {
                write!(
                    f,
                    "header declares {declared} archive bytes but {actual} are present"
                )
            }
            PageError::CrcMismatch { expected, actual } => {
                write!(
                    f,
                    "archive CRC mismatch: header {expected:#010x}, computed {actual:#010x}"
                )
            }
            PageError::Structural(msg) => write!(f, "archive failed structural check: {msg}"),
            PageError::Semantic(msg) => write!(f, "page violates an invariant: {msg}"),
            PageError::Io(e) => write!(f, "i/o error: {e}"),
        }
    }
}

impl std::error::Error for PageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PageError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PageError {
    fn from(e: std::io::Error) -> Self {
        PageError::Io(e)
    }
}

/// Runs all four validation steps on a whole page file (header + archive)
/// and returns the parsed header and the validated archive root.
///
/// `bytes` must be 16-byte aligned (a memory map or an
/// [`rkyv::util::AlignedVec`]); rkyv reports misalignment as a structural
/// error.
pub fn validate(bytes: &[u8]) -> Result<(Header, &ArchivedPage), PageError> {
    // Step 1: header.
    let header = Header::from_bytes(bytes)?;
    header.check()?;
    let archive = &bytes[HEADER_LEN..];
    if header.archive_len != archive.len() as u64 {
        return Err(PageError::LengthMismatch {
            declared: header.archive_len,
            actual: archive.len() as u64,
        });
    }

    // Step 2: CRC.
    let actual = crc32(archive);
    if actual != header.archive_crc32 {
        return Err(PageError::CrcMismatch {
            expected: header.archive_crc32,
            actual,
        });
    }

    // Step 3: rkyv structural check.
    let page = rkyv::access::<ArchivedPage, rkyv::rancor::Error>(archive)
        .map_err(|e| PageError::Structural(e.to_string()))?;

    // Step 4: semantic pass.
    check_semantics(page)?;
    Ok((header, page))
}

fn sem<T>(msg: impl Into<String>) -> Result<T, PageError> {
    Err(PageError::Semantic(msg.into()))
}

/// Checks `off + len` lies within a blob of `total` bytes.
fn in_range(off: u32, len: u32, total: usize, what: &str) -> Result<(), PageError> {
    match (off as u64).checked_add(len as u64) {
        Some(end) if end <= total as u64 => Ok(()),
        _ => sem(format!(
            "{what} range {off}+{len} exceeds blob of {total} bytes"
        )),
    }
}

/// Step 4: every index and range in every table.
pub(crate) fn check_semantics(page: &ArchivedPage) -> Result<(), PageError> {
    let nodes = page.nodes.as_slice();
    let n = nodes.len();
    let text = page.text.as_slice();
    let n_styles = page.styles.len();

    if n == 0 {
        return sem("page has no nodes");
    }
    if n >= NONE as usize {
        return sem("too many nodes");
    }
    if n_styles > STYLE_NONE as usize {
        return sem("more than 65535 styles");
    }
    let text_str = std::str::from_utf8(text)
        .map_err(|e| PageError::Semantic(format!("text blob is not UTF-8: {e}")))?;
    let text_range = |off: u32, len: u32, what: &str| -> Result<(), PageError> {
        in_range(off, len, text.len(), what)?;
        let (o, l) = (off as usize, len as usize);
        if !text_str.is_char_boundary(o) || !text_str.is_char_boundary(o + l) {
            return sem(format!("{what} range {off}+{len} splits a UTF-8 sequence"));
        }
        Ok(())
    };

    // Nodes: tree links, pre-order invariant, ranges, enums, flags.
    if nodes[0].parent.to_native() != NONE {
        return sem("root node has a parent");
    }
    for (i, node) in nodes.iter().enumerate() {
        let idx = i as u32;
        let parent = node.parent.to_native();
        let first_child = node.first_child.to_native();
        let next = node.next_sibling.to_native();

        if i > 0 {
            if parent == NONE || parent as usize >= n {
                return sem(format!("node {i}: parent {parent} out of range"));
            }
            if parent >= idx {
                return sem(format!("node {i}: parent {parent} does not precede it"));
            }
        }
        if first_child != NONE {
            if first_child != idx + 1 || first_child as usize >= n {
                return sem(format!(
                    "node {i}: first_child {first_child} breaks pre-order"
                ));
            }
            if nodes[first_child as usize].parent.to_native() != idx {
                return sem(format!("node {i}: first child does not point back to it"));
            }
        }
        if next != NONE {
            if next as usize >= n {
                return sem(format!("node {i}: next_sibling {next} out of range"));
            }
            if next <= idx {
                return sem(format!("node {i}: next_sibling {next} does not follow it"));
            }
            if nodes[next as usize].parent.to_native() != parent {
                return sem(format!(
                    "node {i}: next_sibling {next} has a different parent"
                ));
            }
        }
        // A node's subtree is contiguous: the first sibling after it must
        // come after all of its descendants. Checked cheaply via the
        // parent-precedes invariant plus: node i+1 (if any) is either its
        // first child or a following sibling/ancestor's sibling, i.e. its
        // parent must be i or an ancestor of i. Verified by walking up.
        if i + 1 < n {
            let np = nodes[i + 1].parent.to_native();
            let mut a = idx;
            let mut ok = false;
            while a != NONE {
                if a == np {
                    ok = true;
                    break;
                }
                a = nodes[a as usize].parent.to_native();
            }
            if !ok {
                return sem(format!(
                    "node {}: parent {np} is not an ancestor of node {i}",
                    i + 1
                ));
            }
        }

        let style = node.style.to_native();
        if style == STYLE_NONE || style as usize >= n_styles {
            return sem(format!("node {i}: style {style} out of range"));
        }
        if NodeFlags::from_bits(node.flags).is_none() {
            return sem(format!("node {i}: unknown flag bits {:#04x}", node.flags));
        }
        if node.reserved != 0 {
            return sem(format!("node {i}: reserved byte is not zero"));
        }
        text_range(
            node.text_off.to_native(),
            node.text_len.to_native(),
            "node text",
        )?;
        text_range(
            node.name_off.to_native(),
            u32::from(node.name_len.to_native()),
            "node name",
        )?;
    }

    // Styles: grid track references.
    let n_tracks = page.tracks.len();
    for (i, style) in page.styles.iter().enumerate() {
        for list in [&style.grid_template_columns, &style.grid_template_rows] {
            let list = list.to_native();
            match list.end() {
                Some(end) if end <= n_tracks => {}
                _ => return sem(format!("style {i}: track list exceeds {n_tracks} tracks")),
            }
        }
    }

    // Attributes: sorted by node, valid node, value in text.
    let mut last_node = 0u32;
    for (i, attr) in page.attrs.iter().enumerate() {
        let node = attr.node.to_native();
        if node as usize >= n {
            return sem(format!("attr {i}: node {node} out of range"));
        }
        if node < last_node {
            return sem(format!("attr {i}: table not sorted by node"));
        }
        last_node = node;
        text_range(
            attr.val_off.to_native(),
            attr.val_len.to_native(),
            "attr value",
        )?;
    }

    // Images: node and blob range.
    for (i, img) in page.images.iter().enumerate() {
        let node = img.node.to_native();
        if node as usize >= n {
            return sem(format!("image {i}: node {node} out of range"));
        }
        in_range(
            img.blob_off.to_native(),
            img.blob_len.to_native(),
            page.blobs.len(),
            "image blob",
        )?;
    }

    // Forms and their field lists.
    let n_fields = page.fields.len();
    for (i, form) in page.forms.iter().enumerate() {
        let node = form.node.to_native();
        if node as usize >= n {
            return sem(format!("form {i}: node {node} out of range"));
        }
        text_range(
            form.action_off.to_native(),
            form.action_len.to_native(),
            "form action",
        )?;
        in_range(
            form.first_field.to_native(),
            form.field_count.to_native(),
            n_fields,
            "form fields",
        )?;
    }
    for (i, field) in page.fields.iter().enumerate() {
        if field.to_native() as usize >= n {
            return sem(format!("field {i}: node out of range"));
        }
    }

    // Top-level blocks: in range and strictly increasing.
    let mut prev: Option<u32> = None;
    for (i, tl) in page.top_level.iter().enumerate() {
        let tl = tl.to_native();
        if tl as usize >= n {
            return sem(format!("top_level {i}: node {tl} out of range"));
        }
        if prev.is_some_and(|p| tl <= p) {
            return sem(format!("top_level {i}: not strictly increasing"));
        }
        prev = Some(tl);
    }

    // Breakpoints sorted ascending.
    let bps = page.breakpoints.as_slice();
    if bps.windows(2).any(|w| w[0].to_native() > w[1].to_native()) {
        return sem("breakpoints are not sorted");
    }

    Ok(())
}
