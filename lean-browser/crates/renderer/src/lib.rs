//! The Lean Browser renderer (plan §2, §7): maps a validated page file,
//! lays out only the scrollbar units intersecting the viewport, and paints
//! through a small strip buffer.
//!
//! The binary `lean-browser` (see `main.rs`) drives this library either
//! headless (`--paint-png`) or, with the `window` feature, in a
//! winit/softbuffer window.
//!
//! `unsafe` is denied crate-wide; the single exception is the read-only
//! font mapping in [`fonts`], documented there and in `STATUS.md`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod document;
pub mod fonts;
pub mod layout;
pub mod paint;
pub mod text;
#[cfg(feature = "window")]
pub mod window;

pub use document::Document;
pub use fonts::{FontSet, FontSource};
pub use layout::LayoutTree;
pub use paint::{paint_viewport, PaintParams, StripBuffer};
pub use text::TextEngine;
