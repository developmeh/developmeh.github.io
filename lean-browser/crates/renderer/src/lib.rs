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
#[doc(hidden)]
pub mod testing;
pub mod text;
#[cfg(feature = "window")]
pub mod window;

pub use document::Document;

/// Installs plan §7's per-component `lean-alloc` budgets (strip, glyph
/// cache, layout tree, image decode). In debug builds an overrun aborts;
/// in release it is logged once per tag. The layout and image figures are
/// deliberate deviations from the plan's 256 KB / 512 KB (see
/// `STATUS.md`): the measured blog-page layout peak is ~290 KB and JPEG /
/// interlaced PNG decode whole images into a 2 MB scratch.
pub fn install_budgets() {
    use lean_alloc::{set_budget, Tag};
    set_budget(Tag::Strip, paint::STRIP_BYTES_CAP);
    set_budget(Tag::GlyphCache, text::GLYPH_CACHE_BUDGET);
    set_budget(Tag::Layout, LAYOUT_BUDGET);
    set_budget(Tag::Image, IMAGE_BUDGET);
}

/// Budget for the transient viewport layout tree (plan: 256 KB).
pub const LAYOUT_BUDGET: usize = 512 * 1024;

/// Budget for image decoding: the display buffer plus the whole-image
/// scratch plus codec state (plan: 512 KB transient).
pub const IMAGE_BUDGET: usize =
    paint::image::DISPLAY_CAP + paint::image::WHOLE_IMAGE_SCRATCH_CAP + 128 * 1024;
pub use fonts::{FontSet, FontSource};
pub use layout::LayoutTree;
pub use paint::{paint_viewport, PaintParams, StripBuffer};
pub use text::TextEngine;
