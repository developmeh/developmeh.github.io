//! A mapped page plus the per-unit `tops[]` array (plan §7 `heights[]`).
//!
//! The document keeps exactly one `f32` per scrollbar unit (the top edge of
//! the unit's border box in page coordinates, plus one sentinel for the end
//! of the content). Everything else is recomputed per paint from the
//! read-only page file.

use std::ops::Range;
use std::path::Path;

use css_subset::{BoxSizing, ComputedStyle, Rgba, Side};
use lean_alloc::{scope, Tag};
use page_format::{ArchivedPage, NodeKind, PageFile, NONE};

use crate::fonts::FontSet;
use crate::layout::{collapse_margins, ContainingBlock, LayoutTree, Layouter};
use crate::text::TextEngine;

/// Cap on `tops[]` from plan §7 (64 KB of `f32`).
pub const MAX_UNITS: usize = 16 * 1024;

/// A validated page ready for viewport layout.
pub struct Document {
    file: PageFile,
    units: Vec<u32>,
    /// `tops[i]` is the border-box top of unit `i`; `tops[units.len()]` is
    /// the bottom of the content.
    tops: Vec<f32>,
    /// Content box of the units' parent (usually `<body>`): `(x, width)`.
    unit_cb: (f32, f32),
    viewport: (f32, f32),
    background: Rgba,
}

impl Document {
    /// Maps and validates `path`, then computes `tops[]` for `viewport`.
    pub fn open(
        path: &Path,
        viewport: (f32, f32),
        fonts: &FontSet,
        text: &mut TextEngine,
    ) -> Result<Document, String> {
        let file = {
            let _tag = scope(Tag::PageFile);
            PageFile::open(path).map_err(|e| format!("rejected page file {}: {e}", path.display()))?
        };
        Ok(Document::from_file(file, viewport, fonts, text))
    }

    /// Wraps an already validated page file.
    pub fn from_file(
        file: PageFile,
        viewport: (f32, f32),
        fonts: &FontSet,
        text: &mut TextEngine,
    ) -> Document {
        let mut doc = Document {
            file,
            units: Vec::new(),
            tops: Vec::new(),
            unit_cb: (0.0, viewport.0),
            viewport,
            background: Rgba::WHITE,
        };
        doc.find_units();
        doc.background = doc.compute_background();
        doc.relayout(viewport, fonts, text);
        doc
    }

    /// The validated archive.
    pub fn page(&self) -> &ArchivedPage {
        self.file.page()
    }

    /// Scrollbar units (node indices).
    pub fn units(&self) -> &[u32] {
        &self.units
    }

    /// Top edges per unit plus the end sentinel.
    pub fn tops(&self) -> &[f32] {
        &self.tops
    }

    /// Canvas background (html, else body, else white).
    pub fn background(&self) -> Rgba {
        self.background
    }

    /// Viewport in CSS px.
    pub fn viewport(&self) -> (f32, f32) {
        self.viewport
    }

    /// Height of the whole document in CSS px (at least the viewport).
    pub fn content_height(&self) -> f32 {
        self.tops
            .last()
            .copied()
            .unwrap_or(0.0)
            .max(self.viewport.1)
    }

    /// Largest useful scroll offset.
    pub fn max_scroll(&self) -> f32 {
        (self.content_height() - self.viewport.1).max(0.0)
    }

    /// The `<body>` node, or the root when there is none.
    fn body(&self) -> u32 {
        let page = self.page();
        page.children(0)
            .find(|&c| page.nodes[c as usize].kind == NodeKind::Body)
            .unwrap_or(0)
    }

    fn find_units(&mut self) {
        let page = self.page();
        let mut units: Vec<u32> = page.top_level.iter().map(|u| u.to_native()).collect();
        if units.is_empty() {
            let body = self.body();
            let children: Vec<u32> = page.children(body).collect();
            let all_blocks = !children.is_empty()
                && children.iter().all(|&c| {
                    let n = &page.nodes[c as usize];
                    let d = page.styles[n.style.to_native() as usize].display;
                    d.is_block_level() || d == css_subset::Display::None
                });
            units = if all_blocks { children } else if page.nodes.len() > 1 { vec![body] } else { Vec::new() };
        }
        // Plan §7: group into at most MAX_UNITS scrollbar units.
        if units.len() > MAX_UNITS {
            let step = units.len().div_ceil(MAX_UNITS);
            units = units.iter().step_by(step).copied().collect();
        }
        self.units = units;
    }

    fn compute_background(&self) -> Rgba {
        let page = self.page();
        let style_of = |n: u32| page.styles[page.nodes[n as usize].style.to_native() as usize].background_color;
        let html_bg = style_of(0);
        if !html_bg.is_transparent() {
            return html_bg;
        }
        let body = self.body();
        let body_bg = style_of(body);
        if !body_bg.is_transparent() {
            return body_bg;
        }
        Rgba::WHITE
    }

    /// Ancestors of the units from the root down to their parent.
    fn ancestor_chain(&self) -> Vec<u32> {
        let page = self.page();
        let Some(&first) = self.units.first() else {
            return Vec::new();
        };
        let mut chain = Vec::new();
        let mut a = page.nodes[first as usize].parent.to_native();
        while a != NONE {
            chain.push(a);
            a = page.nodes[a as usize].parent.to_native();
        }
        chain.reverse();
        chain
    }

    /// Recomputes `tops[]` for a new viewport (or after a font change).
    pub fn relayout(&mut self, viewport: (f32, f32), fonts: &FontSet, text: &mut TextEngine) {
        self.viewport = viewport;
        let page = self.file.page();
        let units = self.units.clone();
        let chain = self.ancestor_chain();
        let mut layouter = Layouter::new(page, fonts, text, viewport.0, viewport.1);

        // Walk the ancestor chain (html, body) to find the units' containing
        // block and the margins that collapse through to the first unit.
        let mut x = 0.0f32;
        let mut w = viewport.0;
        let mut y = 0.0f32;
        let mut pending = 0.0f32;
        let mut bottoms: Vec<(f32, f32)> = Vec::new(); // (border+padding bottom, margin bottom)
        for &a in &chain {
            let s: ComputedStyle = layouter.style(a);
            let sides = layouter.sides(&s, w);
            let edges_h = sides.edges_h();
            let content_w = match layouter.len(s.width, w) {
                Some(cw) if s.box_sizing == BoxSizing::BorderBox => (cw - edges_h).max(0.0),
                Some(cw) => cw.max(0.0),
                None => (w - sides.margin_or0(Side::Left) - sides.margin_or0(Side::Right) - edges_h).max(0.0),
            };
            let (ml, _) = layouter.auto_margins(&sides, w, content_w + edges_h, crate::layout::BlockMode::Flow);
            x += ml + sides.border.left + sides.padding.left;
            w = content_w;
            pending = collapse_margins(pending, sides.margin_or0(Side::Top));
            let top_edge = sides.border.top + sides.padding.top;
            if top_edge > 0.0 {
                y += pending + top_edge;
                pending = 0.0;
            }
            bottoms.push((
                sides.border.bottom + sides.padding.bottom,
                sides.margin_or0(Side::Bottom),
            ));
        }
        self.unit_cb = (x, w);

        let mut tops = {
            let _tag = scope(Tag::PageFile);
            Vec::with_capacity(units.len() + 1)
        };
        for &u in &units {
            let r = layouter.layout_unit(u, ContainingBlock::new(x, y, w, None));
            // Drop the unit's transient tree right away (plan §7).
            layouter.tree = LayoutTree::default();
            let dy = collapse_margins(pending, r.margin_top);
            tops.push(y + dy);
            y += dy + r.height;
            pending = r.margin_bottom;
        }
        for &(edge, margin) in bottoms.iter().rev() {
            pending = collapse_margins(pending, margin);
            if edge > 0.0 {
                y += pending + edge;
                pending = 0.0;
            }
        }
        y += pending;
        tops.push(y);
        self.tops = tops;
    }

    /// Indices of the units intersecting `[y0, y1)` in page coordinates.
    pub fn units_in(&self, y0: f32, y1: f32) -> Range<usize> {
        let n = self.units.len();
        if n == 0 {
            return 0..0;
        }
        // tops is ascending: the first unit whose bottom (the next top) is
        // beyond y0, up to the first unit that starts at or after y1.
        let start = self.tops[1..=n].partition_point(|&b| b <= y0);
        let end = self.tops[..n].partition_point(|&t| t < y1);
        start.min(end)..end
    }

    /// Lays out the units visible at `scroll_y` into a transient tree.
    pub fn layout_viewport(&self, scroll_y: f32, fonts: &FontSet, text: &mut TextEngine) -> LayoutTree {
        let _tag = scope(Tag::Layout);
        let page = self.file.page();
        let mut layouter = Layouter::new(page, fonts, text, self.viewport.0, self.viewport.1);
        let (x, w) = self.unit_cb;
        for i in self.units_in(scroll_y, scroll_y + self.viewport.1) {
            let u = self.units[i];
            layouter.layout_unit(u, ContainingBlock::new(x, self.tops[i], w, None));
        }
        layouter.finish()
    }

    /// The `href` of the link at page point `(x, y)`, if any: hit-tests the
    /// viewport tree and walks up the DOM to a `LINK` node.
    pub fn link_at(&self, tree: &LayoutTree, x: f32, y: f32) -> Option<&str> {
        let page = self.page();
        let hit = tree
            .boxes
            .iter()
            .rev()
            .find(|b| b.rect.intersects(&crate::layout::Rect::new(x, y, 0.001, 0.001)))?;
        let mut n = hit.node;
        while n != NONE {
            let node = &page.nodes[n as usize];
            if node.node_flags().contains(css_subset::NodeFlags::LINK) {
                return page.attr(n, page_format::AttrKey::Href);
            }
            n = node.parent.to_native();
        }
        None
    }
}
