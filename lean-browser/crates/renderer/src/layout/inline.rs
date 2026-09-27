//! Inline formatting: items, white-space processing, shaping, UAX #14 line
//! breaking and line placement.
//!
//! The pipeline for one run of inline-level siblings is:
//!
//! 1. **items**: a depth-first walk over the inline subtree produces text
//!    items (one per face after coverage fallback), atomic inline boxes
//!    (laid out with shrink-to-fit at a provisional origin), inline box
//!    open/close markers (for backgrounds, borders and padding), forced
//!    breaks and `<wbr>`s. Text is appended to one *paragraph string* with
//!    `white-space` collapsing applied;
//! 2. **pieces**: every text item is cut at the break opportunities
//!    `unicode-linebreak` reports for the paragraph, so a piece is the
//!    unit of line filling. Trailing collapsible spaces of a piece hang
//!    (they count as zero width when testing fit and for alignment);
//! 3. **lines**: greedy first-fit; a piece that never fits overflows;
//! 4. **placement**: each line's height and baseline follow from the
//!    strut (the container's font and `line-height`) and the items on it;
//!    glyphs, inline fragments and atomics are then written to the tree.

use std::ops::Range;

use css_subset::{ComputedStyle, TextAlign, TextDecoration, TextTransform, VerticalAlign, WhiteSpace, WordBreak};
use unicode_linebreak::{linebreaks, BreakOpportunity};

use super::{
    BlockMode, BoxKind, ContainingBlock, Edges, LayoutBox, Layouter, Level, PlacedGlyph, Rect,
    TextRun, NONE,
};
use crate::fonts::FaceId;
use crate::text::{FontMetricsPx, ShapedGlyph};

/// Tolerance for "fits" comparisons.
const EPS: f32 = 0.01;

struct TextItem {
    node: u32,
    face: Option<FaceId>,
    size: f32,
    color: css_subset::Rgba,
    decoration: TextDecoration,
    metrics: FontMetricsPx,
    above: f32,
    below: f32,
    range: Range<usize>,
    ws: WhiteSpace,
    break_all: bool,
    glyphs: Range<usize>,
}

struct OpenItem {
    node: u32,
    style: u16,
    /// margin + border + padding on the left.
    left_edge: f32,
    right_edge: f32,
    margin_left: f32,
    margin_right: f32,
    border: Edges,
    padding: Edges,
    metrics: FontMetricsPx,
    above: f32,
    below: f32,
}

struct AtomicItem {
    idx: u32,
    end: u32,
    /// Margin-box size.
    w: f32,
    h: f32,
    margin_top: f32,
    margin_bottom: f32,
    valign: VerticalAlign,
    min_w: f32,
}

enum Item {
    Text(TextItem),
    Atomic(AtomicItem),
    Open(OpenItem),
    Close,
    Break,
    Wbr,
}

struct Piece {
    item: usize,
    glyphs: Range<usize>,
    width: f32,
    /// Width without hanging trailing spaces.
    trimmed: f32,
    min_w: f32,
    break_after: Option<BreakOpportunity>,
    wraps: bool,
    /// The piece is text consisting only of collapsible spaces.
    only_spaces: bool,
}

/// Everything needed to fill and place lines for one inline run.
pub(crate) struct Ifc {
    items: Vec<Item>,
    para: String,
    glyphs: Vec<ShapedGlyph>,
    pieces: Vec<Piece>,
    strut_above: f32,
    strut_below: f32,
    x_height: f32,
    indent: f32,
}

struct Line {
    pieces: Range<usize>,
    used: f32,
}

enum Visit {
    Node(u32),
    Close,
}

/// Appends `text` to the paragraph with `white-space` processing and
/// returns the byte range it occupies.
fn append_text(para: &mut String, text: &str, ws: WhiteSpace, last_was_space: &mut bool) -> Range<usize> {
    let start = para.len();
    let collapsible = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0c');
    for c in text.chars() {
        match ws {
            WhiteSpace::Normal | WhiteSpace::Nowrap => {
                if collapsible(c) {
                    if !*last_was_space {
                        para.push(' ');
                        *last_was_space = true;
                    }
                } else {
                    para.push(c);
                    *last_was_space = false;
                }
            }
            WhiteSpace::PreLine => {
                if c == '\n' {
                    if *last_was_space && para.len() > start && para.ends_with(' ') {
                        para.pop();
                    }
                    para.push('\n');
                    *last_was_space = true;
                } else if collapsible(c) {
                    if !*last_was_space {
                        para.push(' ');
                        *last_was_space = true;
                    }
                } else {
                    para.push(c);
                    *last_was_space = false;
                }
            }
            WhiteSpace::Pre | WhiteSpace::PreWrap => match c {
                '\r' => {}
                '\t' => {
                    let col = para[para.rfind('\n').map_or(0, |i| i + 1)..].chars().count();
                    let n = 8 - (col % 8);
                    for _ in 0..n {
                        para.push(' ');
                    }
                    *last_was_space = false;
                }
                '\n' => {
                    para.push('\n');
                    *last_was_space = true;
                }
                _ => {
                    para.push(c);
                    *last_was_space = false;
                }
            },
        }
    }
    start..para.len()
}

/// Applies `text-transform`.
fn transform_text(text: &str, t: TextTransform) -> std::borrow::Cow<'_, str> {
    match t {
        TextTransform::None => std::borrow::Cow::Borrowed(text),
        TextTransform::Uppercase => text.to_uppercase().into(),
        TextTransform::Lowercase => text.to_lowercase().into(),
        TextTransform::Capitalize => {
            let mut out = String::with_capacity(text.len());
            let mut at_word_start = true;
            for c in text.chars() {
                if c.is_whitespace() {
                    at_word_start = true;
                    out.push(c);
                } else if at_word_start {
                    out.extend(c.to_uppercase());
                    at_word_start = false;
                } else {
                    out.push(c);
                }
            }
            out.into()
        }
    }
}

impl<'a> Layouter<'a> {
    /// Half-leading split of a line-height around a font's ascent/descent.
    fn above_below(&self, s: &ComputedStyle, m: &FontMetricsPx) -> (f32, f32) {
        let lh = self.line_height(s, m);
        let half = (lh - m.ascent - m.descent) / 2.0;
        (m.ascent + half, m.descent + half)
    }

    /// Builds the items, paragraph, glyphs and pieces for `run`.
    /// With `parent_box == NONE` (measurement) atomics are measured, not
    /// laid out, and nothing is written to the tree.
    fn build_ifc(&mut self, parent_box: u32, run: &[u32], container: &ComputedStyle, cw: f32) -> Ifc {
        let measure = parent_box == NONE;
        let page = self.page;
        let cm = self.metrics(container);
        let (strut_above, strut_below) = self.above_below(container, &cm);
        let mut ifc = Ifc {
            items: Vec::new(),
            para: String::new(),
            glyphs: Vec::new(),
            pieces: Vec::new(),
            strut_above,
            strut_below,
            x_height: cm.x_height,
            indent: self.len0(container.text_indent, cw),
        };
        let mut last_was_space = true;
        let mut decorations: Vec<TextDecoration> = vec![container.text_decoration];
        let mut stack: Vec<Visit> = run.iter().rev().map(|&n| Visit::Node(n)).collect();

        while let Some(visit) = stack.pop() {
            let node = match visit {
                Visit::Close => {
                    decorations.pop();
                    ifc.items.push(Item::Close);
                    continue;
                }
                Visit::Node(n) => n,
            };
            match self.level(node, false) {
                Level::Skip => {}
                Level::Text => {
                    let s = self.style(node);
                    let raw = page.text_of(&page.nodes[node as usize]);
                    let text = transform_text(raw, s.text_transform);
                    let range = append_text(&mut ifc.para, &text, s.white_space, &mut last_was_space);
                    if range.is_empty() {
                        continue;
                    }
                    let decoration = decorations
                        .iter()
                        .fold(s.text_decoration, |acc, d| acc | *d);
                    let primary = self.face(&s);
                    let segments: Vec<(Option<FaceId>, Range<usize>)> = match primary {
                        Some(p) => self
                            .text
                            .segment_by_coverage(self.fonts, p, &ifc.para[range.clone()])
                            .into_iter()
                            .map(|(f, r)| (Some(f), range.start + r.start..range.start + r.end))
                            .collect(),
                        None => vec![(None, range.clone())],
                    };
                    for (face, seg) in segments {
                        let metrics = self.text.metrics(self.fonts, face, s.font_size);
                        let (above, below) = self.above_below(&s, &metrics);
                        let gstart = ifc.glyphs.len();
                        if let Some(face) = face {
                            let mut shaped = Vec::new();
                            self.text.shape(self.fonts, face, s.font_size, &ifc.para[seg.clone()], &mut shaped);
                            for g in &mut shaped {
                                g.cluster += seg.start as u32;
                            }
                            ifc.glyphs.extend(shaped);
                        }
                        ifc.items.push(Item::Text(TextItem {
                            node,
                            face,
                            size: s.font_size,
                            color: s.color,
                            decoration,
                            metrics,
                            above,
                            below,
                            range: seg,
                            ws: s.white_space,
                            break_all: s.word_break == WordBreak::BreakAll,
                            glyphs: gstart..ifc.glyphs.len(),
                        }));
                    }
                }
                Level::Br => {
                    ifc.para.push('\n');
                    last_was_space = true;
                    ifc.items.push(Item::Break);
                }
                Level::Wbr => ifc.items.push(Item::Wbr),
                Level::Inline => {
                    let s = self.style(node);
                    let sides = self.sides(&s, cw);
                    let m = self.metrics(&s);
                    let (above, below) = self.above_below(&s, &m);
                    let me = sides.margin_edges();
                    ifc.items.push(Item::Open(OpenItem {
                        node,
                        style: self.style_id(node),
                        left_edge: me.left + sides.border.left + sides.padding.left,
                        right_edge: me.right + sides.border.right + sides.padding.right,
                        margin_left: me.left,
                        margin_right: me.right,
                        border: sides.border,
                        padding: sides.padding,
                        metrics: m,
                        above,
                        below,
                    }));
                    decorations.push(s.text_decoration);
                    stack.push(Visit::Close);
                    let kids: Vec<u32> = page.children(node).collect();
                    for &k in kids.iter().rev() {
                        stack.push(Visit::Node(k));
                    }
                }
                Level::Atomic | Level::Block => {
                    let s = self.style(node);
                    if self.is_out_of_flow(&s) {
                        if !measure {
                            self.defer_abs(node, 0.0, 0.0);
                        }
                        continue;
                    }
                    let mode = if self.level(node, false) == Level::Block {
                        BlockMode::Flow
                    } else {
                        BlockMode::ShrinkToFit
                    };
                    let item = if measure {
                        let sides = self.sides(&s, 0.0);
                        let extra = sides.edges_h() + sides.margin_edges().horizontal();
                        let (min_c, max_c) = self.intrinsic_widths(node);
                        AtomicItem {
                            idx: NONE,
                            end: NONE,
                            w: max_c + extra,
                            h: 0.0,
                            margin_top: 0.0,
                            margin_bottom: 0.0,
                            valign: s.vertical_align,
                            min_w: min_c + extra,
                        }
                    } else {
                        let sides = self.sides(&s, cw);
                        let me = sides.margin_edges();
                        let r = self.layout_block(node, ContainingBlock::new(0.0, 0.0, cw, None), mode);
                        if r.idx == NONE {
                            continue;
                        }
                        self.tree.append_child(parent_box, r.idx);
                        AtomicItem {
                            idx: r.idx,
                            end: self.tree.boxes.len() as u32,
                            w: r.width + me.horizontal(),
                            h: r.height + me.vertical(),
                            margin_top: me.top,
                            margin_bottom: me.bottom,
                            valign: s.vertical_align,
                            min_w: r.width + me.horizontal(),
                        }
                    };
                    ifc.items.push(Item::Atomic(item));
                    last_was_space = false;
                }
            }
        }

        self.build_pieces(&mut ifc, container);
        ifc
    }

    /// Cuts items into pieces at break opportunities.
    fn build_pieces(&self, ifc: &mut Ifc, container: &ComputedStyle) {
        let breaks: Vec<(usize, BreakOpportunity)> = linebreaks(&ifc.para).collect();
        let para_len = ifc.para.len();
        let break_at = |p: usize| -> Option<BreakOpportunity> {
            if p >= para_len {
                return None;
            }
            breaks
                .binary_search_by_key(&p, |b| b.0)
                .ok()
                .map(|i| breaks[i].1)
        };
        let container_wraps = container.white_space.wraps();
        let mut pieces: Vec<Piece> = Vec::new();

        for (ii, item) in ifc.items.iter().enumerate() {
            match item {
                Item::Text(t) => {
                    let mut bounds: Vec<usize> = vec![t.range.start];
                    for &(p, _) in &breaks {
                        if p > t.range.start && p < t.range.end {
                            bounds.push(p);
                        }
                    }
                    if t.break_all {
                        for g in &ifc.glyphs[t.glyphs.clone()] {
                            let c = g.cluster as usize;
                            if c > t.range.start && c < t.range.end {
                                bounds.push(c);
                            }
                        }
                        bounds.sort_unstable();
                        bounds.dedup();
                    }
                    bounds.push(t.range.end);
                    // A break exactly at the item start belongs to the
                    // previous piece.
                    if let Some(prev) = pieces.last_mut() {
                        if prev.break_after.is_none() {
                            prev.break_after = break_at(t.range.start);
                        }
                    }
                    let glyphs = &ifc.glyphs[t.glyphs.clone()];
                    let mut gi = 0usize;
                    for w in bounds.windows(2) {
                        let (a, b) = (w[0], w[1]);
                        let gstart = t.glyphs.start + gi;
                        let mut width = 0.0;
                        while gi < glyphs.len() && (glyphs[gi].cluster as usize) < b {
                            width += glyphs[gi].advance;
                            gi += 1;
                        }
                        let gend = t.glyphs.start + gi;
                        let bytes = &ifc.para[a..b];
                        let hang = t.ws.collapses() || t.ws == WhiteSpace::PreWrap;
                        let trailing = if hang {
                            bytes.len() - bytes.trim_end_matches(' ').len()
                        } else {
                            0
                        };
                        let trimmed = if trailing > 0 {
                            let cut = b - trailing;
                            ifc.glyphs[gstart..gend]
                                .iter()
                                .filter(|g| (g.cluster as usize) < cut)
                                .map(|g| g.advance)
                                .sum()
                        } else {
                            width
                        };
                        let mut break_after = break_at(b);
                        if t.break_all && break_after.is_none() && b < t.range.end {
                            break_after = Some(BreakOpportunity::Allowed);
                        }
                        pieces.push(Piece {
                            item: ii,
                            glyphs: gstart..gend,
                            width,
                            trimmed,
                            min_w: trimmed,
                            break_after,
                            wraps: t.ws.wraps(),
                            only_spaces: hang && bytes.bytes().all(|c| c == b' '),
                        });
                    }
                }
                Item::Atomic(a) => {
                    if let Some(prev) = pieces.last_mut() {
                        if prev.break_after.is_none() && matches!(ifc.items[prev.item], Item::Text(_)) {
                            prev.break_after = Some(BreakOpportunity::Allowed);
                        }
                    }
                    pieces.push(Piece {
                        item: ii,
                        glyphs: 0..0,
                        width: a.w,
                        trimmed: a.w,
                        min_w: a.min_w,
                        break_after: Some(BreakOpportunity::Allowed),
                        wraps: container_wraps,
                        only_spaces: false,
                    });
                }
                Item::Open(o) => {
                    pieces.push(Piece {
                        item: ii,
                        glyphs: 0..0,
                        width: o.left_edge,
                        trimmed: o.left_edge,
                        min_w: o.left_edge,
                        break_after: None,
                        wraps: container_wraps,
                        only_spaces: false,
                    });
                }
                Item::Close => {
                    let width = self.close_width(&ifc.items, ii);
                    pieces.push(Piece {
                        item: ii,
                        glyphs: 0..0,
                        width,
                        trimmed: width,
                        min_w: width,
                        break_after: None,
                        wraps: container_wraps,
                        only_spaces: false,
                    });
                }
                Item::Break => {
                    pieces.push(Piece {
                        item: ii,
                        glyphs: 0..0,
                        width: 0.0,
                        trimmed: 0.0,
                        min_w: 0.0,
                        break_after: Some(BreakOpportunity::Mandatory),
                        wraps: true,
                        only_spaces: false,
                    });
                }
                Item::Wbr => {
                    pieces.push(Piece {
                        item: ii,
                        glyphs: 0..0,
                        width: 0.0,
                        trimmed: 0.0,
                        min_w: 0.0,
                        break_after: Some(BreakOpportunity::Allowed),
                        wraps: true,
                        only_spaces: false,
                    });
                }
            }
        }
        ifc.pieces = pieces;
    }

    /// Right edge width of the inline box closed by item `close_idx`.
    fn close_width(&self, items: &[Item], close_idx: usize) -> f32 {
        let mut depth = 0usize;
        for item in items[..close_idx].iter().rev() {
            match item {
                Item::Close => depth += 1,
                Item::Open(o) => {
                    if depth == 0 {
                        return o.right_edge;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        0.0
    }

    /// Whether a run has anything that generates a line box.
    fn has_content(ifc: &Ifc) -> bool {
        ifc.pieces.iter().any(|p| match ifc.items[p.item] {
            Item::Text(_) => !p.only_spaces,
            Item::Atomic(_) | Item::Break => true,
            _ => false,
        })
    }

    /// Whether a line may end after piece `i`.
    fn can_break_after(pieces: &[Piece], i: usize) -> bool {
        match pieces[i].break_after {
            Some(BreakOpportunity::Mandatory) => true,
            Some(BreakOpportunity::Allowed) => {
                pieces[i].wraps && pieces.get(i + 1).is_none_or(|q| q.wraps)
            }
            None => false,
        }
    }

    /// Greedy first-fit line filling.
    fn break_lines(ifc: &Ifc, avail_first: f32, avail: f32) -> Vec<Line> {
        let pieces = &ifc.pieces;
        let n = pieces.len();
        let mut lines: Vec<Line> = Vec::new();
        let mut start = 0usize;
        while start < n {
            let avail = if lines.is_empty() { avail_first } else { avail };
            let mut width = 0.0f32;
            let mut last_break: Option<(usize, f32)> = None;
            let mut end: Option<(usize, f32)> = None;
            let mut i = start;
            let mut used = 0.0f32;
            while i < n {
                let p = &pieces[i];
                let fits = width + p.trimmed <= avail + EPS;
                if !fits && i > start {
                    if let Some((bi, bused)) = last_break {
                        end = Some((bi + 1, bused));
                        break;
                    }
                }
                width += p.width;
                used = width - p.width + p.trimmed;
                if p.break_after == Some(BreakOpportunity::Mandatory) {
                    end = Some((i + 1, used));
                    break;
                }
                if Self::can_break_after(pieces, i) {
                    last_break = Some((i, used));
                }
                i += 1;
            }
            let (e, u) = end.unwrap_or((n, used));
            lines.push(Line {
                pieces: start..e,
                used: u,
            });
            start = e;
        }
        lines
    }

    /// `(min-content, max-content)` of an inline run (measurement mode).
    pub(crate) fn measure_inline_run(&mut self, run: &[u32], container: &ComputedStyle) -> (f32, f32) {
        let ifc = self.build_ifc(NONE, run, container, 0.0);
        let pieces = &ifc.pieces;
        let mut max_c = 0.0f32;
        let mut min_c = 0.0f32;
        let mut seg_w = 0.0f32;
        let mut word_w = 0.0f32;
        for (i, p) in pieces.iter().enumerate() {
            let last_of_seg = p.break_after == Some(BreakOpportunity::Mandatory) || i + 1 == pieces.len();
            let breakable = Self::can_break_after(pieces, i);
            if breakable || last_of_seg {
                word_w += p.min_w;
                min_c = min_c.max(word_w);
                word_w = 0.0;
            } else {
                word_w += p.min_w.max(p.width);
            }
            if last_of_seg {
                seg_w += p.trimmed;
                max_c = max_c.max(seg_w);
                seg_w = 0.0;
            } else {
                seg_w += p.width;
            }
        }
        let indent = ifc.indent.max(0.0);
        (min_c + indent, max_c + indent)
    }

    /// Lays out a run of inline-level siblings as lines starting at `y`.
    /// Returns the height of the lines, or `None` when the run generates
    /// no line box (collapsible white space only).
    pub(crate) fn layout_inline_run(
        &mut self,
        parent_box: u32,
        run: &[u32],
        cx: f32,
        y: f32,
        cw: f32,
        container: &ComputedStyle,
    ) -> Option<f32> {
        let ifc = self.build_ifc(parent_box, run, container, cw);
        if !Self::has_content(&ifc) {
            return None;
        }
        let lines = Self::break_lines(&ifc, (cw - ifc.indent).max(0.0), cw.max(0.0));
        Some(self.place_lines(parent_box, &ifc, &lines, cx, y, cw, container))
    }

    #[allow(clippy::too_many_arguments)]
    fn place_lines(
        &mut self,
        parent_box: u32,
        ifc: &Ifc,
        lines: &[Line],
        cx: f32,
        y0: f32,
        cw: f32,
        container: &ComputedStyle,
    ) -> f32 {
        struct OpenFrag {
            item: usize,
            start_x: f32,
            has_left: bool,
        }
        let mut y = y0;
        let mut open: Vec<OpenFrag> = Vec::new();

        for (li, line) in lines.iter().enumerate() {
            // ---- vertical metrics ----
            let mut above = ifc.strut_above;
            let mut below = ifc.strut_below;
            let mut tb_max = 0.0f32;
            for p in &ifc.pieces[line.pieces.clone()] {
                match &ifc.items[p.item] {
                    Item::Text(t) => {
                        above = above.max(t.above);
                        below = below.max(t.below);
                    }
                    Item::Open(o) => {
                        above = above.max(o.above);
                        below = below.max(o.below);
                    }
                    Item::Atomic(a) => match a.valign {
                        VerticalAlign::Baseline => above = above.max(a.h),
                        VerticalAlign::Middle => {
                            let up = a.h / 2.0 + ifc.x_height / 2.0;
                            above = above.max(up);
                            below = below.max(a.h - up);
                        }
                        VerticalAlign::Top | VerticalAlign::Bottom => tb_max = tb_max.max(a.h),
                    },
                    _ => {}
                }
            }
            let line_h = (above + below).max(tb_max);
            let baseline = y + above;

            // ---- horizontal start ----
            let indent = if li == 0 { ifc.indent } else { 0.0 };
            let extra = (cw - indent - line.used).max(0.0);
            let mut x = cx
                + indent
                + match container.text_align {
                    TextAlign::Right => extra,
                    TextAlign::Center => extra / 2.0,
                    TextAlign::Left | TextAlign::Justify => 0.0,
                };
            for o in &mut open {
                o.start_x = x;
                o.has_left = false;
            }

            let mut frags: Vec<LayoutBox> = Vec::new();
            let mut runs: Vec<LayoutBox> = Vec::new();
            let mut cur: Option<(usize, f32, f32, Vec<PlacedGlyph>)> = None; // item, start x, pen, glyphs

            let flush = |cur: &mut Option<(usize, f32, f32, Vec<PlacedGlyph>)>, runs: &mut Vec<LayoutBox>, items: &[Item], baseline: f32| {
                if let Some((item, start_x, pen, glyphs)) = cur.take() {
                    let Item::Text(t) = &items[item] else { return };
                    runs.push(LayoutBox {
                        node: t.node,
                        style: 0,
                        rect: Rect::new(
                            start_x,
                            baseline - t.metrics.ascent,
                            pen,
                            t.metrics.ascent + t.metrics.descent,
                        ),
                        kind: BoxKind::Text(TextRun {
                            face: t.face,
                            size: t.size,
                            color: t.color,
                            baseline: t.metrics.ascent,
                            decoration: t.decoration,
                            metrics: t.metrics,
                            glyphs,
                        }),
                        clip: false,
                        border: Edges::default(),
                        padding: Edges::default(),
                        first_child: NONE,
                        last_child: NONE,
                        next_sibling: NONE,
                    });
                }
            };

            for pi in line.pieces.clone() {
                let p = &ifc.pieces[pi];
                match &ifc.items[p.item] {
                    Item::Text(_) => {
                        if cur.as_ref().is_some_and(|c| c.0 != p.item) {
                            flush(&mut cur, &mut runs, &ifc.items, baseline);
                        }
                        let c = cur.get_or_insert_with(|| (p.item, x, 0.0, Vec::new()));
                        for g in &ifc.glyphs[p.glyphs.clone()] {
                            c.3.push(PlacedGlyph {
                                id: g.id,
                                x: c.2 + g.x,
                                y: g.y,
                            });
                            c.2 += g.advance;
                        }
                        x += p.width;
                    }
                    Item::Atomic(a) => {
                        flush(&mut cur, &mut runs, &ifc.items, baseline);
                        if a.idx != NONE {
                            let border_h = a.h - a.margin_top - a.margin_bottom;
                            let target_y = match a.valign {
                                VerticalAlign::Baseline => baseline - a.margin_bottom - border_h,
                                VerticalAlign::Middle => {
                                    baseline - ifc.x_height / 2.0 - a.h / 2.0 + a.margin_top
                                }
                                VerticalAlign::Top => y + a.margin_top,
                                VerticalAlign::Bottom => y + line_h - a.margin_bottom - border_h,
                            };
                            let cur_y = self.tree.boxes[a.idx as usize].rect.y;
                            self.tree.shift_range(a.idx, a.end, x, target_y - cur_y);
                        }
                        x += p.width;
                    }
                    Item::Open(_) => {
                        flush(&mut cur, &mut runs, &ifc.items, baseline);
                        open.push(OpenFrag {
                            item: p.item,
                            start_x: x,
                            has_left: true,
                        });
                        x += p.width;
                    }
                    Item::Close => {
                        flush(&mut cur, &mut runs, &ifc.items, baseline);
                        x += p.width;
                        if let Some(o) = open.pop() {
                            if let Item::Open(oi) = &ifc.items[o.item] {
                                frags.push(inline_fragment(oi, o.start_x, x, o.has_left, true, baseline));
                            }
                        }
                    }
                    Item::Break | Item::Wbr => {}
                }
            }
            flush(&mut cur, &mut runs, &ifc.items, baseline);
            for o in &open {
                if let Item::Open(oi) = &ifc.items[o.item] {
                    frags.push(inline_fragment(oi, o.start_x, x, o.has_left, false, baseline));
                }
            }
            for f in frags {
                let idx = self.tree.push(f);
                self.tree.append_child(parent_box, idx);
            }
            for mut r in runs {
                r.style = self.style_id(r.node);
                let idx = self.tree.push(r);
                self.tree.append_child(parent_box, idx);
            }
            y += line_h;
        }
        y - y0
    }
}

/// One line's fragment of an inline element.
fn inline_fragment(o: &OpenItem, start_x: f32, end_x: f32, has_left: bool, has_right: bool, baseline: f32) -> LayoutBox {
    let ml = if has_left { o.margin_left } else { 0.0 };
    let mr = if has_right { o.margin_right } else { 0.0 };
    let border = Edges {
        top: o.border.top,
        bottom: o.border.bottom,
        left: if has_left { o.border.left } else { 0.0 },
        right: if has_right { o.border.right } else { 0.0 },
    };
    let padding = Edges {
        top: o.padding.top,
        bottom: o.padding.bottom,
        left: if has_left { o.padding.left } else { 0.0 },
        right: if has_right { o.padding.right } else { 0.0 },
    };
    let top = baseline - o.metrics.ascent - padding.top - border.top;
    let h = o.metrics.ascent + o.metrics.descent + padding.vertical() + border.vertical();
    LayoutBox {
        node: o.node,
        style: o.style,
        rect: Rect::new(start_x + ml, top, (end_x - mr - start_x - ml).max(0.0), h),
        kind: BoxKind::Inline,
        clip: false,
        border,
        padding,
        first_child: NONE,
        last_child: NONE,
        next_sibling: NONE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_whitespace() {
        let mut para = String::new();
        let mut last = true;
        let r = append_text(&mut para, "  Hello \n  world  ", WhiteSpace::Normal, &mut last);
        assert_eq!(&para[r], "Hello world ");
        let r2 = append_text(&mut para, " again", WhiteSpace::Normal, &mut last);
        assert_eq!(&para[r2], "again");
    }

    #[test]
    fn preserves_pre_and_expands_tabs() {
        let mut para = String::new();
        let mut last = true;
        let r = append_text(&mut para, "a\tb\n\tc", WhiteSpace::Pre, &mut last);
        assert_eq!(&para[r], "a       b\n        c");
    }

    #[test]
    fn pre_line_keeps_newlines_only() {
        let mut para = String::new();
        let mut last = true;
        let r = append_text(&mut para, "a  \n  b", WhiteSpace::PreLine, &mut last);
        assert_eq!(&para[r], "a\nb");
    }

    #[test]
    fn transforms() {
        assert_eq!(transform_text("hello world", TextTransform::Capitalize), "Hello World");
        assert_eq!(transform_text("Hi", TextTransform::Uppercase), "HI");
        assert_eq!(transform_text("Hi", TextTransform::Lowercase), "hi");
    }
}
