//! Block formatting: widths, heights, margin collapsing, positioning,
//! list markers and intrinsic widths.

use css_subset::{
    BorderStyle, BoxSizing, ComputedStyle, Display, LengthUnit, ListStyleType, Overflow, Position,
    Side,
};
use page_format::NodeKind;

use super::{
    collapse_margins, BoxKind, Edges, LayoutBox, Layouter, Level, PlacedGlyph, Rect, TextRun, NONE,
};

/// The containing block a box is laid out in: content box of the parent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContainingBlock {
    /// Left edge.
    pub x: f32,
    /// Provisional top edge for the box's border box (flow) or the
    /// containing block's top (absolute).
    pub y: f32,
    /// Width (percent base).
    pub w: f32,
    /// Height when definite (percent base for heights), else `None`.
    pub h: Option<f32>,
}

impl ContainingBlock {
    /// Builds a containing block.
    pub const fn new(x: f32, y: f32, w: f32, h: Option<f32>) -> ContainingBlock {
        ContainingBlock { x, y, w, h }
    }
}

/// How a block's size and position are determined.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockMode {
    /// Normal flow: `width: auto` fills the containing block.
    Flow,
    /// `inline-block` and friends: `width: auto` shrinks to fit.
    ShrinkToFit,
    /// Absolutely positioned against `cb`; `static_*` is where the box
    /// would have been in flow.
    Absolute {
        /// Static x position.
        static_x: f32,
        /// Static y position.
        static_y: f32,
    },
}

/// What a block layout produced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockResult {
    /// The box index, or [`NONE`] if nothing was generated.
    pub idx: u32,
    /// Border-box left edge.
    pub x: f32,
    /// Border-box width.
    pub width: f32,
    /// Border-box height.
    pub height: f32,
    /// Top margin including margins collapsed through from the first child.
    pub margin_top: f32,
    /// Bottom margin including margins collapsed through from the last child.
    pub margin_bottom: f32,
}

impl BlockResult {
    /// A result for a node that generates no box.
    pub fn empty(cb: ContainingBlock) -> BlockResult {
        BlockResult {
            idx: NONE,
            x: cb.x,
            width: 0.0,
            height: 0.0,
            margin_top: 0.0,
            margin_bottom: 0.0,
        }
    }
}

/// Resolved margins (`None` = auto), padding and border widths.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Sides {
    pub margin: [Option<f32>; 4],
    pub padding: Edges,
    pub border: Edges,
}

impl Sides {
    pub fn margin_or0(&self, side: Side) -> f32 {
        self.margin[side as usize].unwrap_or(0.0)
    }

    pub fn margin_edges(&self) -> Edges {
        Edges {
            top: self.margin_or0(Side::Top),
            right: self.margin_or0(Side::Right),
            bottom: self.margin_or0(Side::Bottom),
            left: self.margin_or0(Side::Left),
        }
    }

    pub fn edges_h(&self) -> f32 {
        self.padding.horizontal() + self.border.horizontal()
    }

    pub fn edges_v(&self) -> f32 {
        self.padding.vertical() + self.border.vertical()
    }
}

/// Result of laying out a block container's in-flow children.
struct FlowResult {
    height: f32,
    first_margin: Option<f32>,
    last_margin: Option<f32>,
}

impl<'a> Layouter<'a> {
    /// Margins, padding and border widths of a style against `cb_w`.
    pub(crate) fn sides(&self, s: &ComputedStyle, cb_w: f32) -> Sides {
        let mut sides = Sides::default();
        for side in Side::ALL {
            let i = side as usize;
            sides.margin[i] = self.len(s.margin[i], cb_w);
            let pad = self.len0(s.padding[i], cb_w).max(0.0);
            let bw = if s.border_style[i] == BorderStyle::None {
                0.0
            } else {
                s.border_width[i].max(0.0)
            };
            match side {
                Side::Top => {
                    sides.padding.top = pad;
                    sides.border.top = bw;
                }
                Side::Right => {
                    sides.padding.right = pad;
                    sides.border.right = bw;
                }
                Side::Bottom => {
                    sides.padding.bottom = pad;
                    sides.border.bottom = bw;
                }
                Side::Left => {
                    sides.padding.left = pad;
                    sides.border.left = bw;
                }
            }
        }
        sides
    }

    /// Whether a block establishes a new block formatting context (its
    /// margins never collapse with its children's).
    fn is_bfc_root(&self, s: &ComputedStyle, mode: BlockMode, replaced: bool) -> bool {
        replaced
            || !matches!(mode, BlockMode::Flow)
            || s.overflow_x != Overflow::Visible
            || s.overflow_y != Overflow::Visible
            || s.display.is_flex_or_grid_container()
            || matches!(
                s.display,
                Display::InlineBlock | Display::Table | Display::TableCell
            )
            || s.float != css_subset::Float::None
    }

    /// Lays out `node` as a block-level box. In `Flow` mode the border box
    /// is placed provisionally at `(cb.x + margin_left, cb.y)`; the caller
    /// shifts it once the collapsed top margin is known.
    pub fn layout_block(&mut self, node: u32, cb: ContainingBlock, mode: BlockMode) -> BlockResult {
        if !self.enter() {
            return BlockResult::empty(cb);
        }
        let r = self.layout_block_inner(node, cb, mode);
        self.leave();
        r
    }

    fn layout_block_inner(
        &mut self,
        node: u32,
        cb: ContainingBlock,
        mode: BlockMode,
    ) -> BlockResult {
        let s = self.style(node);
        let sides = self.sides(&s, cb.w);
        let replaced = self.replaced_kind(node);
        let bfc = self.is_bfc_root(&s, mode, replaced.is_some());
        let is_abs = matches!(mode, BlockMode::Absolute { .. });

        // Insets (absolute positioning) resolved against the containing block.
        let inset = |l: &Layouter, side: Side, base: f32| l.len(s.inset[side as usize], base);
        let (left, right) = if is_abs {
            (
                inset(self, Side::Left, cb.w),
                inset(self, Side::Right, cb.w),
            )
        } else {
            (None, None)
        };

        // ---- horizontal ----
        let (content_w, content_h_replaced, ml, mr) =
            self.block_width(node, &s, &sides, cb, mode, replaced.is_some(), left, right);
        let border_w = content_w + sides.edges_h();

        // ---- explicit height ----
        let explicit_h = self.explicit_height(&s, &sides, cb.h);
        let min_h = self.len(s.min_height, cb.h.unwrap_or(0.0)).unwrap_or(0.0);
        let max_h = self
            .len(s.max_height, cb.h.unwrap_or(f32::INFINITY))
            .filter(|_| s.max_height.unit != LengthUnit::Percent || cb.h.is_some())
            .unwrap_or(f32::INFINITY);

        let x = if is_abs {
            cb.x + left.unwrap_or(0.0) + ml
        } else {
            cb.x + ml
        };
        let y = cb.y;
        let idx = self.tree.push(LayoutBox {
            node,
            style: self.style_id(node),
            rect: Rect::new(x, y, border_w, 0.0),
            kind: match replaced {
                Some(NodeKind::Image) => match self.image_index(node) {
                    Some(image) => BoxKind::Image { image },
                    None => BoxKind::Placeholder,
                },
                Some(NodeKind::Svg) => BoxKind::Placeholder,
                Some(NodeKind::FormControl) => BoxKind::Control,
                _ => BoxKind::Block,
            },
            clip: s.overflow_x != Overflow::Visible || s.overflow_y != Overflow::Visible,
            border: sides.border,
            padding: sides.padding,
            first_child: NONE,
            last_child: NONE,
            next_sibling: NONE,
        });

        let positioned = s.position != Position::Static;
        if positioned {
            self.push_abs_frame();
        }

        let content_x = x + sides.border.left + sides.padding.left;
        let content_y = y + sides.border.top + sides.padding.top;
        let collapse_top = !bfc && sides.border.top == 0.0 && sides.padding.top == 0.0;
        let collapse_bottom = !bfc
            && sides.border.bottom == 0.0
            && sides.padding.bottom == 0.0
            && explicit_h.is_none();

        let flow = if let Some(h) = content_h_replaced {
            FlowResult {
                height: h,
                first_margin: None,
                last_margin: None,
            }
        } else {
            let child_cb_h = explicit_h;
            self.layout_flow_children(
                idx,
                node,
                content_x,
                content_y,
                content_w,
                child_cb_h,
                collapse_top,
                collapse_bottom,
                s.display.is_flex_or_grid_container(),
            )
        };

        let content_h = explicit_h
            .unwrap_or(flow.height)
            .max(min_h)
            .min(max_h)
            .max(0.0);
        let border_h = content_h + sides.edges_v();
        self.tree.boxes[idx as usize].rect.h = border_h;

        // ---- list marker ----
        if s.display == Display::ListItem && s.list_style_type != ListStyleType::None {
            self.add_marker(idx, node, &s, content_x, content_y);
        }

        // ---- absolute descendants of this positioned box ----
        if positioned {
            let pad = self.tree.boxes[idx as usize].padding_box();
            self.pop_abs_frame(idx, pad);
        }

        // ---- final position ----
        let end = self.tree.boxes.len() as u32;
        match mode {
            BlockMode::Absolute { static_x, static_y } => {
                let mt = sides.margin_or0(Side::Top);
                let mb = sides.margin_or0(Side::Bottom);
                let top = inset(self, Side::Top, cb.h.unwrap_or(0.0));
                let bottom = inset(self, Side::Bottom, cb.h.unwrap_or(0.0));
                let final_y = match (top, bottom) {
                    (Some(t), _) => cb.y + t + mt,
                    (None, Some(b)) => cb.y + cb.h.unwrap_or(0.0) - b - mb - border_h,
                    (None, None) => static_y + mt,
                };
                let final_x = match (left, right) {
                    (Some(_), _) => x,
                    (None, Some(r)) => cb.x + cb.w - r - mr - border_w,
                    (None, None) => static_x + ml,
                };
                self.tree.shift_range(idx, end, final_x - x, final_y - y);
            }
            _ => {
                if s.position == Position::Relative || s.position == Position::Sticky {
                    let dx = self
                        .len(s.inset[Side::Left as usize], cb.w)
                        .or_else(|| self.len(s.inset[Side::Right as usize], cb.w).map(|r| -r))
                        .unwrap_or(0.0);
                    let dy = self
                        .len(s.inset[Side::Top as usize], cb.h.unwrap_or(0.0))
                        .or_else(|| {
                            self.len(s.inset[Side::Bottom as usize], cb.h.unwrap_or(0.0))
                                .map(|b| -b)
                        })
                        .unwrap_or(0.0);
                    self.tree.shift_range(idx, end, dx, dy);
                }
            }
        }

        let mt = sides.margin_or0(Side::Top);
        let mb = sides.margin_or0(Side::Bottom);
        BlockResult {
            idx,
            x,
            width: border_w,
            height: border_h,
            margin_top: match flow.first_margin {
                Some(m) if collapse_top => collapse_margins(mt, m),
                _ => mt,
            },
            margin_bottom: match flow.last_margin {
                Some(m) if collapse_bottom => collapse_margins(mb, m),
                _ => mb,
            },
        }
    }

    /// Resolves the content width and horizontal margins (CSS 2.1 §10.3).
    /// Returns `(content_w, replaced_content_h, margin_left, margin_right)`.
    #[allow(clippy::too_many_arguments)]
    fn block_width(
        &mut self,
        node: u32,
        s: &ComputedStyle,
        sides: &Sides,
        cb: ContainingBlock,
        mode: BlockMode,
        replaced: bool,
        left: Option<f32>,
        right: Option<f32>,
    ) -> (f32, Option<f32>, f32, f32) {
        let edges_h = sides.edges_h();
        let border_box = s.box_sizing == BoxSizing::BorderBox;
        let to_content = |w: f32| {
            if border_box {
                (w - edges_h).max(0.0)
            } else {
                w
            }
        };
        let min_w = self.len(s.min_width, cb.w).map(to_content).unwrap_or(0.0);
        let max_w = self
            .len(s.max_width, cb.w)
            .map(to_content)
            .unwrap_or(f32::INFINITY);
        let clamp = |w: f32| w.max(min_w).min(max_w).max(0.0);
        let ml0 = sides.margin_or0(Side::Left);
        let mr0 = sides.margin_or0(Side::Right);

        if replaced {
            let (iw, ih) = self.intrinsic_size(node);
            let ratio = if ih > 0.0 { iw / ih } else { 1.0 };
            let ex_w = self.len(s.width, cb.w).map(to_content);
            let ex_h = self.explicit_height(s, sides, cb.h);
            let mut w = match (ex_w, ex_h) {
                (Some(w), _) => w,
                (None, Some(h)) => h * ratio,
                (None, None) => iw,
            };
            w = clamp(w);
            let h = match ex_h {
                Some(h) => h,
                None => {
                    if ratio > 0.0 {
                        w / ratio
                    } else {
                        ih
                    }
                }
            };
            let (ml, mr) = self.auto_margins(sides, cb.w, w + edges_h, mode);
            return (w, Some(h), ml, mr);
        }

        match self.len(s.width, cb.w).map(to_content) {
            Some(w) => {
                let w = clamp(w);
                let (ml, mr) = self.auto_margins(sides, cb.w, w + edges_h, mode);
                (w, None, ml, mr)
            }
            None => match mode {
                BlockMode::Flow => {
                    let fill = cb.w - ml0 - mr0 - edges_h;
                    let w = clamp(fill);
                    if w < fill - 0.01 {
                        // CSS 2.1 §10.4: a `max-width` violation re-runs the
                        // width rules with the clamped width, so `auto`
                        // margins centre the box instead of being zero.
                        let (ml, mr) = self.auto_margins(sides, cb.w, w + edges_h, mode);
                        (w, None, ml, mr)
                    } else {
                        (w, None, ml0, mr0)
                    }
                }
                BlockMode::Absolute { .. } if left.is_some() && right.is_some() => {
                    let w = clamp(
                        cb.w - left.unwrap_or(0.0) - right.unwrap_or(0.0) - ml0 - mr0 - edges_h,
                    );
                    (w, None, ml0, mr0)
                }
                _ => {
                    let avail =
                        (cb.w - left.unwrap_or(0.0) - right.unwrap_or(0.0) - ml0 - mr0 - edges_h)
                            .max(0.0);
                    let (min_c, max_c) = self.intrinsic_widths(node);
                    let w = clamp(min_c.max(avail.min(max_c)));
                    (w, None, ml0, mr0)
                }
            },
        }
    }

    /// Resolves `auto` horizontal margins for a box of `outer_w` (border-box
    /// width) in a containing block of `cb_w`.
    pub(crate) fn auto_margins(
        &self,
        sides: &Sides,
        cb_w: f32,
        outer_w: f32,
        mode: BlockMode,
    ) -> (f32, f32) {
        let (ml, mr) = (
            sides.margin[Side::Left as usize],
            sides.margin[Side::Right as usize],
        );
        if !matches!(mode, BlockMode::Flow) {
            return (ml.unwrap_or(0.0), mr.unwrap_or(0.0));
        }
        match (ml, mr) {
            (Some(l), Some(r)) => (l, r),
            (None, None) => {
                let free = ((cb_w - outer_w) / 2.0).max(0.0);
                (free, free)
            }
            (None, Some(r)) => ((cb_w - outer_w - r).max(0.0), r),
            (Some(l), None) => (l, (cb_w - outer_w - l).max(0.0)),
        }
    }

    /// Explicit content height, if `height` is definite.
    fn explicit_height(&self, s: &ComputedStyle, sides: &Sides, cb_h: Option<f32>) -> Option<f32> {
        if s.height.unit == LengthUnit::Percent && cb_h.is_none() {
            return None;
        }
        let h = self.len(s.height, cb_h.unwrap_or(0.0))?;
        Some(if s.box_sizing == BoxSizing::BorderBox {
            (h - sides.edges_v()).max(0.0)
        } else {
            h.max(0.0)
        })
    }

    /// Lays out the in-flow children of a block container. Runs of
    /// inline-level children become line boxes; block-level children are
    /// stacked with margin collapsing.
    #[allow(clippy::too_many_arguments)]
    fn layout_flow_children(
        &mut self,
        parent_box: u32,
        node: u32,
        cx: f32,
        cy: f32,
        cw: f32,
        cb_h: Option<f32>,
        collapse_top: bool,
        collapse_bottom: bool,
        blockify: bool,
    ) -> FlowResult {
        let page = self.page;
        let container_style = self.style(node);
        let mut y = cy;
        let mut pending: Option<f32> = None;
        let mut first_margin: Option<f32> = None;
        let mut any_block_content = false;
        let mut last_was_block = false;
        let mut run: Vec<u32> = Vec::new();

        let children: Vec<u32> = page.children(node).collect();
        let mut i = 0;
        while i <= children.len() {
            let child = children.get(i).copied();
            let level = child.map_or(Level::Block, |c| self.level(c, blockify));
            let is_block_boundary = child.is_none() || level == Level::Block;
            if is_block_boundary {
                // Flush pending inline run into lines.
                if !run.is_empty() {
                    let line_y = y + pending.unwrap_or(0.0);
                    if let Some(h) =
                        self.layout_inline_run(parent_box, &run, cx, line_y, cw, &container_style)
                    {
                        y = line_y + h;
                        pending = None;
                        any_block_content = true;
                        last_was_block = false;
                    }
                    run.clear();
                }
                let Some(child) = child else { break };
                let cs = self.style(child);
                if self.is_out_of_flow(&cs) {
                    self.defer_abs(child, cx, y + pending.unwrap_or(0.0));
                    i += 1;
                    continue;
                }
                let cb = ContainingBlock::new(cx, y, cw, cb_h);
                let r = self.layout_block(child, cb, BlockMode::Flow);
                if r.idx != NONE {
                    self.tree.append_child(parent_box, r.idx);
                    let end = self.tree.boxes.len() as u32;
                    let dy = if !any_block_content && collapse_top {
                        // The first child's top margin escapes through the
                        // parent's top edge.
                        first_margin = Some(r.margin_top);
                        0.0
                    } else {
                        collapse_margins(pending.unwrap_or(0.0), r.margin_top)
                    };
                    self.tree.shift_range(r.idx, end, 0.0, dy);
                    y += dy + r.height;
                    pending = Some(r.margin_bottom);
                    any_block_content = true;
                    last_was_block = true;
                }
            } else if level != Level::Skip {
                if let Some(c) = child {
                    run.push(c);
                }
            }
            i += 1;
        }

        let (height, last_margin) = match pending {
            Some(m) if last_was_block && collapse_bottom => (y - cy, Some(m)),
            Some(m) => (y - cy + m, None),
            None => (y - cy, None),
        };
        FlowResult {
            height: height.max(0.0),
            first_margin,
            last_margin,
        }
    }

    /// Adds an outside list marker for a `display: list-item` box.
    fn add_marker(
        &mut self,
        idx: u32,
        node: u32,
        s: &ComputedStyle,
        content_x: f32,
        content_y: f32,
    ) {
        // A loader-written page carries a `Marker` node as the item's first
        // child holding the exact text (it knows `<ol start>`, `reversed`
        // and `<li value>`); a builder-made page has none, so synthesize.
        let text = match self.marker_child(node) {
            Some(m) => self.page.text_of(&self.page.nodes[m as usize]).to_string(),
            None => match s.list_style_type {
                ListStyleType::Disc => "\u{2022} ".to_string(),
                ListStyleType::Circle => "\u{25E6} ".to_string(),
                ListStyleType::Square => "\u{25AA} ".to_string(),
                ListStyleType::Decimal => format!("{}. ", self.list_index(node)),
                ListStyleType::None => return,
            },
        };
        if text.is_empty() {
            return;
        }
        let Some(primary) = self.face(s) else { return };
        let metrics = self.metrics(s);
        // Bullet glyphs may live in another face; fall back per character.
        let segs = self.text.segment_by_coverage(self.fonts, primary, &text);
        let face = segs.first().map_or(primary, |(f, _)| *f);
        let mut shaped = Vec::new();
        self.text
            .shape(self.fonts, face, s.font_size, &text, &mut shaped);
        let width: f32 = shaped.iter().map(|g| g.advance).sum();
        let mut glyphs = Vec::with_capacity(shaped.len());
        let mut pen = 0.0;
        for g in &shaped {
            glyphs.push(PlacedGlyph {
                id: g.id,
                x: pen + g.x,
                y: g.y,
            });
            pen += g.advance;
        }
        // Align with the first line of the item's content, else with the
        // item's own first line.
        let end = self.tree.boxes.len();
        let baseline = self.tree.boxes[idx as usize + 1..end]
            .iter()
            .find_map(|b| match &b.kind {
                BoxKind::Text(t) => Some(b.rect.y + t.baseline),
                _ => None,
            })
            .unwrap_or_else(|| {
                let lh = self.line_height(s, &metrics);
                content_y + (lh - metrics.ascent - metrics.descent) / 2.0 + metrics.ascent
            });
        let run = TextRun {
            face: Some(face),
            size: s.font_size,
            color: s.color,
            baseline: metrics.ascent,
            decoration: css_subset::TextDecoration::empty(),
            opacity: 1.0,
            metrics,
            glyphs,
        };
        let marker = self.tree.push(LayoutBox {
            node,
            style: self.style_id(node),
            rect: Rect::new(
                content_x - width,
                baseline - metrics.ascent,
                width,
                metrics.ascent + metrics.descent,
            ),
            kind: BoxKind::Text(run),
            clip: false,
            border: Edges::default(),
            padding: Edges::default(),
            first_child: NONE,
            last_child: NONE,
            next_sibling: NONE,
        });
        self.tree.append_child(idx, marker);
    }

    /// The `Marker` child the loader generated for a list item, if any.
    fn marker_child(&self, node: u32) -> Option<u32> {
        self.page.children(node).find(|&c| {
            let n = &self.page.nodes[c as usize];
            n.kind == NodeKind::Marker && n.text_len.to_native() > 0
        })
    }

    /// 1-based position of a list item among its parent's list items.
    fn list_index(&self, node: u32) -> usize {
        let parent = self.page.nodes[node as usize].parent.to_native();
        if parent == page_format::NONE {
            return 1;
        }
        let mut n = 0;
        for c in self.page.children(parent) {
            let sid = self.page.nodes[c as usize].style.to_native();
            if self.page.styles[sid as usize].display == Display::ListItem {
                n += 1;
            }
            if c == node {
                break;
            }
        }
        n.max(1)
    }

    /// `(min-content, max-content)` widths of a node's content box.
    pub(crate) fn intrinsic_widths(&mut self, node: u32) -> (f32, f32) {
        if !self.enter() {
            return (0.0, 0.0);
        }
        let r = self.intrinsic_widths_inner(node);
        self.leave();
        r
    }

    fn intrinsic_widths_inner(&mut self, node: u32) -> (f32, f32) {
        let s = self.style(node);
        let sides = self.sides(&s, 0.0);
        let border_box = s.box_sizing == BoxSizing::BorderBox;
        let to_content = |w: f32| {
            if border_box {
                (w - sides.edges_h()).max(0.0)
            } else {
                w
            }
        };
        let clamp = |l: &Layouter, w: f32| {
            let min_w = if s.min_width.unit == LengthUnit::Px {
                to_content(s.min_width.value)
            } else {
                0.0
            };
            let max_w = if s.max_width.unit == LengthUnit::Px {
                to_content(s.max_width.value)
            } else {
                f32::INFINITY
            };
            let _ = l;
            w.max(min_w).min(max_w).max(0.0)
        };
        if s.width.unit == LengthUnit::Px {
            let w = clamp(self, to_content(s.width.value));
            return (w, w);
        }
        if self.replaced_kind(node).is_some() {
            let (iw, ih) = self.intrinsic_size(node);
            let w = if s.height.unit == LengthUnit::Px && ih > 0.0 {
                s.height.value * iw / ih
            } else {
                iw
            };
            let w = clamp(self, w);
            return (w, w);
        }

        let page = self.page;
        let blockify = s.display.is_flex_or_grid_container();
        let (mut min_c, mut max_c) = (0.0f32, 0.0f32);
        let mut run: Vec<u32> = Vec::new();
        let children: Vec<u32> = page.children(node).collect();
        for (i, child) in children
            .iter()
            .copied()
            .map(Some)
            .chain(std::iter::once(None))
            .enumerate()
        {
            let _ = i;
            let level = child.map_or(Level::Block, |c| self.level(c, blockify));
            if child.is_none() || level == Level::Block {
                if !run.is_empty() {
                    let (rmin, rmax) = self.measure_inline_run(&run, &s);
                    min_c = min_c.max(rmin);
                    max_c = max_c.max(rmax);
                    run.clear();
                }
                if let Some(child) = child {
                    let cs = self.style(child);
                    if self.is_out_of_flow(&cs) {
                        continue;
                    }
                    let csides = self.sides(&cs, 0.0);
                    let extra = csides.edges_h() + csides.margin_edges().horizontal();
                    let (cmin, cmax) = self.intrinsic_widths(child);
                    min_c = min_c.max(cmin + extra);
                    max_c = max_c.max(cmax + extra);
                }
            } else if level != Level::Skip {
                if let Some(c) = child {
                    run.push(c);
                }
            }
        }
        (clamp(self, min_c), clamp(self, max_c))
    }
}
