//! Shared cascade data model for Lean Browser.
//!
//! The loader *writes* these types (after running the lightningcss cascade)
//! and the renderer *reads* them straight out of the memory-mapped page file,
//! so everything here is an rkyv archive type. The model is deliberately a
//! subset of CSS (plan §5): enough for consistent, not complete, layout.
//!
//! Fieldless enums are archived "as themselves" (`#[rkyv(as = Self)]`): the
//! archived representation is the very same `#[repr(u8)]` enum, so the
//! renderer never needs to convert. Structs that carry `f32` values get a
//! generated `Archived*` twin with little-endian fields.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod enums;
mod intern;
mod node;
mod style;

pub use enums::*;
pub use intern::{StyleId, StyleTable, StyleTableFull, MAX_STYLES, STYLE_NONE};
pub use node::NodeFlags;
pub use style::{
    ArchivedComputedStyle, ArchivedGridLine, ArchivedGridPlacement, ArchivedGridTrack,
    ArchivedLength, ComputedStyle, GridLine, GridPlacement, GridTrack, Length, Rgba, Side,
    TextDecoration, TrackListRef, Z_INDEX_AUTO,
};
