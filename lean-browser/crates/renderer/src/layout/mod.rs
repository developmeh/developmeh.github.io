//! Viewport-only block/inline layout (plan §7).
//!
//! The renderer never lays out the whole page at once. A page's scrollbar
//! units are its *top-level blocks* (`Page.top_level`); the
//! [`crate::document::Document`] keeps one `f32` per unit (its top edge)
//! and, for a paint, only the units intersecting the viewport are laid out
//! into a transient [`LayoutTree`] that is dropped afterwards.
//!
//! Coordinates are CSS px in page space (y grows downward, 0 at the top of
//! the document). Every [`LayoutBox`] is pushed in pre-order, so a subtree
//! occupies a contiguous index range and can be moved by
//! [`LayoutTree::shift_range`] once its collapsed margins are known.
//!
//! What is implemented for M2 and what is approximated is listed in
//! `STATUS.md`; in short: block and inline formatting with margin
//! collapsing, `inline-block` shrink-to-fit, replaced elements, relative
//! and absolute positioning (fixed = absolute), lists with outside markers,
//! `white-space`, `text-align`, `text-indent`, `word-break: break-all`.
//! Floats, flex, grid and tables degrade to block / inline-block flow.

mod block;
mod inline;

use css_subset::{
    ComputedStyle, Display, FontFamily, FontStyle, FontWeight, Length, Position, Rgba,
    TextDecoration,
};
use lean_alloc::{scope, Tag};
use page_format::{ArchivedPage, NodeKind};

use crate::fonts::{FaceId, FontSet};
use crate::text::{FontMetricsPx, TextEngine, MAX_FONT_PX};

pub use block::{BlockMode, BlockResult, ContainingBlock};

/// "No box".
pub const NONE: u32 = u32::MAX;

/// Maximum nesting depth of block layout before children are dropped
/// (plan §7 forbids deep recursion; the loader is expected to flatten
/// pathological documents).
pub const MAX_DEPTH: u32 = 96;

/// An axis-aligned rectangle in CSS px.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

impl Rect {
    /// Builds a rectangle.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    /// Right edge.
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// Bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Whether the two rectangles overlap (touching edges do not count).
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }

    /// Intersection, or `None` when empty.
    pub fn intersect(&self, o: &Rect) -> Option<Rect> {
        let x = self.x.max(o.x);
        let y = self.y.max(o.y);
        let r = self.right().min(o.right());
        let b = self.bottom().min(o.bottom());
        (r > x && b > y).then(|| Rect::new(x, y, r - x, b - y))
    }

    /// Shrinks by `e` on every side.
    pub fn inset(&self, e: &Edges) -> Rect {
        Rect::new(
            self.x + e.left,
            self.y + e.top,
            (self.w - e.left - e.right).max(0.0),
            (self.h - e.top - e.bottom).max(0.0),
        )
    }
}

/// Four edge widths in CSS px.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges {
    /// Top edge width.
    pub top: f32,
    /// Right edge width.
    pub right: f32,
    /// Bottom edge width.
    pub bottom: f32,
    /// Left edge width.
    pub left: f32,
}

impl Edges {
    /// `left + right`.
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// `top + bottom`.
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    /// Sum of two edge sets.
    pub fn plus(&self, o: &Edges) -> Edges {
        Edges {
            top: self.top + o.top,
            right: self.right + o.right,
            bottom: self.bottom + o.bottom,
            left: self.left + o.left,
        }
    }
}

/// A glyph placed in a [`TextRun`], relative to the run's origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    /// Glyph id in the run's face.
    pub id: u16,
    /// Horizontal offset from the run's left edge.
    pub x: f32,
    /// Vertical offset from the baseline (positive = up).
    pub y: f32,
}

/// A run of text in one face, size and colour on one line.
#[derive(Clone, Debug)]
pub struct TextRun {
    /// Face, or `None` when no font is available (nothing is painted).
    pub face: Option<FaceId>,
    /// Font size in CSS px.
    pub size: f32,
    /// Text colour.
    pub color: Rgba,
    /// Baseline offset from the top of the run's rect.
    pub baseline: f32,
    /// Decoration lines to draw.
    pub decoration: TextDecoration,
    /// Opacity inherited from inline ancestors (block ancestors apply theirs
    /// in the paint walk).
    pub opacity: f32,
    /// Metrics for decoration placement.
    pub metrics: FontMetricsPx,
    /// The glyphs, left to right.
    pub glyphs: Vec<PlacedGlyph>,
}

/// What a box paints.
#[derive(Clone, Debug)]
pub enum BoxKind {
    /// A block-level box: background and borders from its style.
    Block,
    /// One line fragment of an inline element: background and borders.
    Inline,
    /// Shaped text.
    Text(TextRun),
    /// A decoded-at-paint image; `rect` is the content box.
    Image {
        /// Index into `Page.images`.
        image: u32,
    },
    /// A replaced element the renderer cannot draw (SVG, unsupported
    /// formats, iframes, media): a grey placeholder.
    Placeholder,
    /// A form control (M3): drawn as an outlined field.
    Control,
}

/// One box of the transient layout tree.
#[derive(Clone, Debug)]
pub struct LayoutBox {
    /// Owning node.
    pub node: u32,
    /// Style id of the owning node.
    pub style: u16,
    /// Border box (content box for `Text` and `Image`).
    pub rect: Rect,
    /// What to paint.
    pub kind: BoxKind,
    /// Whether descendants are clipped to this box's padding box.
    pub clip: bool,
    /// Border + padding edges (to derive the padding/content boxes).
    pub border: Edges,
    /// Padding edges.
    pub padding: Edges,
    /// First child or [`NONE`].
    pub first_child: u32,
    /// Last child or [`NONE`] (for O(1) append).
    pub last_child: u32,
    /// Next sibling or [`NONE`].
    pub next_sibling: u32,
}

impl LayoutBox {
    /// Padding box.
    pub fn padding_box(&self) -> Rect {
        self.rect.inset(&self.border)
    }

    /// Content box.
    pub fn content_box(&self) -> Rect {
        self.rect.inset(&self.border.plus(&self.padding))
    }
}

/// The transient per-paint tree.
#[derive(Debug, Default)]
pub struct LayoutTree {
    /// Boxes in pre-order.
    pub boxes: Vec<LayoutBox>,
    /// One root per laid-out unit, in document order.
    pub roots: Vec<u32>,
}

impl LayoutTree {
    fn push(&mut self, b: LayoutBox) -> u32 {
        self.boxes.push(b);
        (self.boxes.len() - 1) as u32
    }

    fn append_child(&mut self, parent: u32, child: u32) {
        let p = &mut self.boxes[parent as usize];
        if p.first_child == NONE {
            p.first_child = child;
        } else {
            let last = p.last_child;
            self.boxes[last as usize].next_sibling = child;
        }
        self.boxes[parent as usize].last_child = child;
    }

    /// Moves every box in `start..end` by `(dx, dy)`.
    pub fn shift_range(&mut self, start: u32, end: u32, dx: f32, dy: f32) {
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        for b in &mut self.boxes[start as usize..end as usize] {
            b.rect.x += dx;
            b.rect.y += dy;
        }
    }

    /// Children of a box.
    pub fn children(&self, idx: u32) -> impl Iterator<Item = u32> + '_ {
        let mut next = self.boxes[idx as usize].first_child;
        std::iter::from_fn(move || {
            if next == NONE {
                return None;
            }
            let cur = next;
            next = self.boxes[cur as usize].next_sibling;
            Some(cur)
        })
    }

    /// Indented text dump for debugging (`--dump-boxes`).
    pub fn dump(&self) -> String {
        let mut out = String::new();
        let mut stack: Vec<(u32, usize)> = self.roots.iter().rev().map(|&r| (r, 0)).collect();
        while let Some((idx, depth)) = stack.pop() {
            let b = &self.boxes[idx as usize];
            let kind = match &b.kind {
                BoxKind::Block => "block".to_string(),
                BoxKind::Inline => "inline".to_string(),
                BoxKind::Text(t) => format!("text[{} glyphs, {:.1}px]", t.glyphs.len(), t.size),
                BoxKind::Image { image } => format!("image#{image}"),
                BoxKind::Placeholder => "placeholder".to_string(),
                BoxKind::Control => "control".to_string(),
            };
            out.push_str(&format!(
                "{:indent$}#{idx} node {} style {} {kind} @({:.1},{:.1}) {:.1}x{:.1}{}\n",
                "",
                b.node,
                b.style,
                b.rect.x,
                b.rect.y,
                b.rect.w,
                b.rect.h,
                if b.clip { " clip" } else { "" },
                indent = depth * 2
            ));
            let kids: Vec<u32> = self.children(idx).collect();
            for &k in kids.iter().rev() {
                stack.push((k, depth + 1));
            }
        }
        out
    }
}

/// How a node participates in its parent's formatting context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Block,
    Inline,
    Atomic,
    Text,
    Br,
    Wbr,
    Skip,
}

/// An absolutely positioned descendant waiting for its containing block.
struct PendingAbs {
    node: u32,
    static_x: f32,
    static_y: f32,
}

/// Lays out units of one page into a [`LayoutTree`].
pub struct Layouter<'a> {
    /// The validated page.
    pub page: &'a ArchivedPage,
    /// Available faces.
    pub fonts: &'a FontSet,
    /// Shaper.
    pub text: &'a mut TextEngine,
    /// Viewport width in CSS px (for `vw`).
    pub vw: f32,
    /// Viewport height in CSS px (for `vh`).
    pub vh: f32,
    /// The tree under construction.
    pub tree: LayoutTree,
    abs_frames: Vec<Vec<PendingAbs>>,
    depth: u32,
}

impl<'a> Layouter<'a> {
    /// A layouter with an empty tree.
    pub fn new(
        page: &'a ArchivedPage,
        fonts: &'a FontSet,
        text: &'a mut TextEngine,
        vw: f32,
        vh: f32,
    ) -> Layouter<'a> {
        Layouter {
            page,
            fonts,
            text,
            vw,
            vh,
            tree: LayoutTree::default(),
            abs_frames: Vec::new(),
            depth: 0,
        }
    }

    /// Takes the finished tree.
    pub fn finish(self) -> LayoutTree {
        self.tree
    }

    /// Computed style of a node (copied out of the page file).
    /// Style of a node. `font-size` is clamped to `0..=MAX_FONT_PX` here,
    /// once, so metrics, line heights, run rects and glyph masks stay
    /// bounded whatever a validated page asks for.
    pub fn style(&self, node: u32) -> ComputedStyle {
        let id = self.page.nodes[node as usize].style.to_native();
        let mut s: ComputedStyle = self.page.styles[id as usize].to_native();
        s.font_size = if s.font_size.is_finite() {
            s.font_size.clamp(0.0, MAX_FONT_PX)
        } else {
            0.0
        };
        s
    }

    /// Style id of a node.
    pub fn style_id(&self, node: u32) -> u16 {
        self.page.nodes[node as usize].style.to_native()
    }

    /// Resolves a length against `base` (`None` for `auto`).
    pub fn len(&self, l: Length, base: f32) -> Option<f32> {
        l.resolve(base, self.vw, self.vh)
    }

    /// Resolves a length, treating `auto` as zero.
    pub fn len0(&self, l: Length, base: f32) -> f32 {
        self.len(l, base).unwrap_or(0.0)
    }

    /// The face for a style.
    pub fn face(&self, s: &ComputedStyle) -> Option<FaceId> {
        self.face_for(s.font_family, s.font_weight, s.font_style)
    }

    /// The face for explicit font properties.
    pub fn face_for(
        &self,
        family: FontFamily,
        weight: FontWeight,
        style: FontStyle,
    ) -> Option<FaceId> {
        self.fonts.face(family, weight, style)
    }

    /// Scaled metrics for a style's font.
    pub fn metrics(&self, s: &ComputedStyle) -> FontMetricsPx {
        let face = self.face(s);
        self.text.metrics(self.fonts, face, s.font_size)
    }

    /// The used `line-height` of a style in px.
    pub fn line_height(&self, s: &ComputedStyle, m: &FontMetricsPx) -> f32 {
        self.len(s.line_height, s.font_size)
            .unwrap_or_else(|| m.natural_line_height())
            .max(0.0)
    }

    /// Lays out one top-level unit with its border box at `(x, y)` inside a
    /// containing block `cb` (the body's content box). Returns the block
    /// result; the tree gains one root.
    pub fn layout_unit(&mut self, node: u32, cb: ContainingBlock) -> BlockResult {
        let _tag = scope(Tag::Layout);
        self.abs_frames.push(Vec::new());
        let r = match self.level(node, false) {
            Level::Block => self.layout_block(node, cb, BlockMode::Flow),
            Level::Skip => BlockResult::empty(cb),
            _ => self.layout_anonymous_block(node, cb),
        };
        // The unit is the initial containing block for absolute descendants
        // (plan R2 approximation).
        let frame = self.abs_frames.pop().unwrap_or_default();
        if r.idx != NONE {
            self.tree.roots.push(r.idx);
            let pad = self.tree.boxes[r.idx as usize].padding_box();
            self.place_abs(r.idx, pad, frame);
        }
        r
    }

    /// Wraps a single inline-level top-level node in an anonymous block.
    fn layout_anonymous_block(&mut self, node: u32, cb: ContainingBlock) -> BlockResult {
        let parent = self.page.nodes[node as usize].parent.to_native();
        let parent_style = if parent == page_format::NONE {
            ComputedStyle::INITIAL
        } else {
            self.style(parent)
        };
        let idx = self.tree.push(LayoutBox {
            node,
            style: self.style_id(node),
            rect: Rect::new(cb.x, cb.y, cb.w, 0.0),
            kind: BoxKind::Block,
            clip: false,
            border: Edges::default(),
            padding: Edges::default(),
            first_child: NONE,
            last_child: NONE,
            next_sibling: NONE,
        });
        let run = [node];
        let h = self
            .layout_inline_run(idx, &run, cb.x, cb.y, cb.w, &parent_style)
            .unwrap_or(0.0);
        self.tree.boxes[idx as usize].rect.h = h;
        BlockResult {
            idx,
            x: cb.x,
            width: cb.w,
            height: h,
            margin_top: 0.0,
            margin_bottom: 0.0,
        }
    }

    /// Classifies a node for its parent's formatting context.
    pub(crate) fn level(&self, node: u32, parent_blockifies: bool) -> Level {
        let n = &self.page.nodes[node as usize];
        let has_text = n.text_len.to_native() > 0;
        match n.kind {
            NodeKind::Text => {
                if has_text {
                    Level::Text
                } else {
                    Level::Skip
                }
            }
            // The loader's marker node is consumed by `add_marker` (outside
            // position); it never takes part in the inline flow.
            NodeKind::Marker => Level::Skip,
            NodeKind::LineBreak => Level::Br,
            NodeKind::Wbr => Level::Wbr,
            _ => {
                let display = self.page.styles[n.style.to_native() as usize].display;
                let replaced = matches!(
                    n.kind,
                    NodeKind::Image | NodeKind::Svg | NodeKind::FormControl
                );
                match display {
                    Display::None => Level::Skip,
                    _ if parent_blockifies => Level::Block,
                    Display::Inline if replaced => Level::Atomic,
                    Display::Inline => Level::Inline,
                    Display::InlineBlock
                    | Display::InlineFlex
                    | Display::InlineGrid
                    | Display::TableCell => Level::Atomic,
                    _ => Level::Block,
                }
            }
        }
    }

    /// Whether `node` is out of flow (absolute or fixed).
    pub(crate) fn is_out_of_flow(&self, s: &ComputedStyle) -> bool {
        matches!(s.position, Position::Absolute | Position::Fixed)
    }

    /// Whether a node is a replaced element and, if an image, its record.
    pub(crate) fn replaced_kind(&self, node: u32) -> Option<NodeKind> {
        let k = self.page.nodes[node as usize].kind;
        matches!(k, NodeKind::Image | NodeKind::Svg | NodeKind::FormControl).then_some(k)
    }

    /// Intrinsic size of a replaced element in CSS px.
    pub(crate) fn intrinsic_size(&self, node: u32) -> (f32, f32) {
        match self.page.nodes[node as usize].kind {
            NodeKind::Image | NodeKind::Svg => match self.image_index(node) {
                Some(i) => {
                    let img = &self.page.images[i as usize];
                    let (w, h) = (
                        f32::from(img.width.to_native()),
                        f32::from(img.height.to_native()),
                    );
                    if w > 0.0 && h > 0.0 {
                        (w, h)
                    } else {
                        (300.0, 150.0)
                    }
                }
                None => (300.0, 150.0),
            },
            NodeKind::FormControl => {
                let ty = self
                    .page
                    .attr(node, page_format::AttrKey::Type)
                    .unwrap_or("text");
                match ty {
                    "checkbox" | "radio" => (13.0, 13.0),
                    "hidden" => (0.0, 0.0),
                    "submit" | "button" | "reset" => (70.0, 21.0),
                    _ => (150.0, 21.0),
                }
            }
            _ => (0.0, 0.0),
        }
    }

    /// Index into `Page.images` for a node, if any.
    pub(crate) fn image_index(&self, node: u32) -> Option<u32> {
        self.page
            .images
            .iter()
            .position(|i| i.node.to_native() == node)
            .map(|i| i as u32)
    }

    /// Records an out-of-flow child for the nearest positioned ancestor.
    pub(crate) fn defer_abs(&mut self, node: u32, static_x: f32, static_y: f32) {
        match self.abs_frames.last_mut() {
            Some(frame) => frame.push(PendingAbs {
                node,
                static_x,
                static_y,
            }),
            None => {
                // No unit frame (should not happen); lay out in flow.
                let cb = ContainingBlock::new(static_x, static_y, self.vw, None);
                self.layout_block(node, cb, BlockMode::Flow);
            }
        }
    }

    /// Opens a containing-block frame for absolute descendants.
    pub(crate) fn push_abs_frame(&mut self) {
        self.abs_frames.push(Vec::new());
    }

    /// Closes the frame opened by [`Self::push_abs_frame`] and lays its
    /// entries out against `padding_box`, appending them to `container`.
    pub(crate) fn pop_abs_frame(&mut self, container: u32, padding_box: Rect) {
        let frame = self.abs_frames.pop().unwrap_or_default();
        self.place_abs(container, padding_box, frame);
    }

    fn place_abs(&mut self, container: u32, padding_box: Rect, frame: Vec<PendingAbs>) {
        for p in frame {
            let cb = ContainingBlock::new(
                padding_box.x,
                padding_box.y,
                padding_box.w,
                Some(padding_box.h),
            );
            let r = self.layout_block(
                p.node,
                cb,
                BlockMode::Absolute {
                    static_x: p.static_x,
                    static_y: p.static_y,
                },
            );
            if r.idx != NONE {
                self.tree.append_child(container, r.idx);
            }
        }
    }

    pub(crate) fn enter(&mut self) -> bool {
        if self.depth >= MAX_DEPTH {
            return false;
        }
        self.depth += 1;
        true
    }

    pub(crate) fn leave(&mut self) {
        self.depth -= 1;
    }
}

/// Combines two adjoining vertical margins (CSS 2.1 §8.3.1).
pub fn collapse_margins(a: f32, b: f32) -> f32 {
    if a >= 0.0 && b >= 0.0 {
        a.max(b)
    } else if a < 0.0 && b < 0.0 {
        a.min(b)
    } else {
        a + b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_ops() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 5.0, 10.0, 10.0);
        assert!(a.intersects(&b));
        assert_eq!(a.intersect(&b), Some(Rect::new(5.0, 5.0, 5.0, 5.0)));
        assert!(!a.intersects(&Rect::new(10.0, 0.0, 1.0, 1.0)));
        let e = Edges {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        };
        assert_eq!(a.inset(&e), Rect::new(4.0, 1.0, 4.0, 6.0));
    }

    #[test]
    fn margin_collapsing_rules() {
        assert_eq!(collapse_margins(10.0, 20.0), 20.0);
        assert_eq!(collapse_margins(-10.0, -20.0), -20.0);
        assert_eq!(collapse_margins(10.0, -4.0), 6.0);
    }
}
