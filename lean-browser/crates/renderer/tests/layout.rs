//! Layout unit tests against hand-computed boxes (plan §9).
//!
//! Block-only tests need no font; tests that shape text skip when the
//! machine has no usable font (see `renderer::fonts`).

use css_subset::{
    Display, FontFamily, FontStyle, FontWeight, Length, ListStyleType, Overflow, Position, Rgba,
    Side, TextAlign, VerticalAlign,
};
use page_format::{NodeKind, Role};
use renderer::layout::{BoxKind, Rect};
use renderer::testing::{block, border, inline, margins, paddings, style, PageBuilder};
use renderer::{Document, FontSet, FontSource, TextEngine};

fn no_fonts() -> FontSet {
    FontSet::empty()
}

fn system_fonts() -> Option<FontSet> {
    let set = FontSet::load(&FontSource::Auto).ok()?;
    if set.is_empty() {
        eprintln!("no system fonts; skipping");
        None
    } else {
        Some(set)
    }
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

macro_rules! assert_rect {
    ($r:expr, $x:expr, $y:expr, $w:expr, $h:expr) => {{
        let r: Rect = $r;
        assert!(
            approx(r.x, $x) && approx(r.y, $y) && approx(r.w, $w) && approx(r.h, $h),
            "expected ({}, {}, {}, {}), got ({}, {}, {}, {})",
            $x,
            $y,
            $w,
            $h,
            r.x,
            r.y,
            r.w,
            r.h
        );
    }};
}

/// Finds the first box for `node`.
fn box_of(tree: &renderer::LayoutTree, node: u32) -> &renderer::layout::LayoutBox {
    tree.boxes
        .iter()
        .find(|b| b.node == node)
        .unwrap_or_else(|| panic!("no box for node {node}"))
}

#[test]
fn blocks_with_margins_padding_borders_and_auto_centering() {
    // body margin 8 (UA).
    let mut b = PageBuilder::new("about:test", 400, PageBuilder::ua_body());
    let a = b.leaf(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            margins(s, 10.0, 0.0, 0.0, 0.0);
            paddings(s, 5.0);
            border(s, 2.0, Rgba::BLACK);
            s.height = Length::px(50.0);
        }),
    );
    let c = b.leaf(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            s.margin = [
                Length::px(20.0),
                Length::AUTO,
                Length::px(30.0),
                Length::AUTO,
            ];
            s.width = Length::px(100.0);
            s.height = Length::px(20.0);
        }),
    );
    let file = b.build();
    let fonts = no_fonts();
    let mut text = TextEngine::default();
    let doc = Document::from_file(file, (400.0, 300.0), &fonts, &mut text);

    // Body's 8px margin collapses with A's 10px: A starts at y=10.
    assert_eq!(doc.unit_count(), 2);
    assert_eq!(doc.tops(), &[10.0, 94.0, 144.0]);
    assert_eq!(doc.content_height(), 300.0); // at least the viewport
    assert_eq!(doc.max_scroll(), 0.0);

    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    assert_rect!(box_of(&tree, a).rect, 8.0, 10.0, 384.0, 64.0);
    assert_rect!(box_of(&tree, a).content_box(), 15.0, 17.0, 370.0, 50.0);
    // C: 100px wide, centred in the 384px content box, 20px below A.
    assert_rect!(box_of(&tree, c).rect, 150.0, 94.0, 100.0, 20.0);
}

#[test]
fn margins_collapse_through_a_wrapper_but_not_a_bfc_root() {
    let mut b = PageBuilder::new("about:test", 300, PageBuilder::ua_body());
    // Wrapper with no padding/border: child's 16px margin escapes.
    let wrapper = b.open(NodeKind::Div, Role::Generic, block(|_| {}));
    let p1 = b.leaf(
        NodeKind::P,
        Role::Paragraph,
        block(|s| {
            margins(s, 16.0, 0.0, 16.0, 0.0);
            s.height = Length::px(10.0);
        }),
    );
    b.close();
    // overflow: hidden establishes a BFC: its child's margin stays inside.
    let bfc = b.open(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            s.overflow_y = Overflow::Hidden;
        }),
    );
    let p2 = b.leaf(
        NodeKind::P,
        Role::Paragraph,
        block(|s| {
            margins(s, 16.0, 0.0, 16.0, 0.0);
            s.height = Length::px(10.0);
        }),
    );
    b.close();
    let file = b.build();
    let fonts = no_fonts();
    let mut text = TextEngine::default();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);

    // max(body 8, p1 16) = 16 above the wrapper; wrapper height is p1's 10.
    assert_rect!(box_of(&tree, wrapper).rect, 8.0, 16.0, 284.0, 10.0);
    assert_rect!(box_of(&tree, p1).rect, 8.0, 16.0, 284.0, 10.0);
    // p1's bottom margin (16) collapses with the wrapper's (0): bfc at 42.
    assert_rect!(box_of(&tree, bfc).rect, 8.0, 42.0, 284.0, 42.0);
    assert_rect!(box_of(&tree, p2).rect, 8.0, 58.0, 284.0, 10.0);
    assert!(box_of(&tree, bfc).clip);
    // bfc ends at 84 with no bottom margin; body's 8px closes the document.
    assert_eq!(doc.tops(), &[16.0, 42.0, 92.0]);
}

#[test]
fn inline_block_is_aligned_on_the_line_and_centred() {
    // No fonts: the strut uses synthetic metrics (ascent 14.4, descent 4).
    let mut b = PageBuilder::new("about:test", 200, block(|_| {}));
    let container = b.open(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            s.line_height = Length::px(20.0);
            s.text_align = TextAlign::Center;
        }),
    );
    let ib = b.leaf(
        NodeKind::Span,
        Role::Generic,
        style(|s| {
            s.display = Display::InlineBlock;
            s.width = Length::px(40.0);
            s.height = Length::px(10.0);
        }),
    );
    b.close();
    let file = b.build();
    let fonts = no_fonts();
    let mut text = TextEngine::default();
    let doc = Document::from_file(file, (200.0, 100.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    // One 20px line; the 10px box sits on the baseline at 15.2.
    assert_rect!(box_of(&tree, container).rect, 0.0, 0.0, 200.0, 20.0);
    assert_rect!(box_of(&tree, ib).rect, 80.0, 5.2, 40.0, 10.0);
}

#[test]
fn absolute_and_relative_positioning() {
    let mut b = PageBuilder::new("about:test", 300, PageBuilder::ua_body());
    let rel = b.open(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            s.position = Position::Relative;
            s.inset[Side::Left as usize] = Length::px(5.0);
            s.height = Length::px(100.0);
            paddings(s, 10.0);
        }),
    );
    let abs = b.leaf(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            s.position = Position::Absolute;
            s.inset[Side::Top as usize] = Length::px(10.0);
            s.inset[Side::Left as usize] = Length::px(20.0);
            s.width = Length::px(30.0);
            s.height = Length::px(40.0);
        }),
    );
    let fixed = b.leaf(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            s.position = Position::Fixed;
            s.inset[Side::Bottom as usize] = Length::px(0.0);
            s.inset[Side::Right as usize] = Length::px(0.0);
            s.width = Length::px(10.0);
            s.height = Length::px(10.0);
        }),
    );
    b.close();
    let file = b.build();
    let fonts = no_fonts();
    let mut text = TextEngine::default();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    // relative: shifted right by 5; padding box is (13, 8, 284, 120).
    assert_rect!(box_of(&tree, rel).rect, 13.0, 8.0, 284.0, 120.0);
    assert_rect!(box_of(&tree, abs).rect, 33.0, 18.0, 30.0, 40.0);
    // fixed degrades to absolute against the same containing block.
    assert_rect!(
        box_of(&tree, fixed).rect,
        13.0 + 284.0 - 10.0,
        8.0 + 120.0 - 10.0,
        10.0,
        10.0
    );
    // Out-of-flow boxes do not add to the flow height.
    assert_eq!(doc.tops(), &[8.0, 136.0]);
}

#[test]
fn units_intersecting_a_scrolled_viewport() {
    let mut b = PageBuilder::new("about:test", 300, PageBuilder::ua_body());
    for _ in 0..3 {
        b.leaf(
            NodeKind::Div,
            Role::Generic,
            block(|s| s.height = Length::px(100.0)),
        );
    }
    let file = b.build();
    let fonts = no_fonts();
    let mut text = TextEngine::default();
    let doc = Document::from_file(file, (300.0, 150.0), &fonts, &mut text);
    assert_eq!(doc.tops(), &[8.0, 108.0, 208.0, 316.0]);
    assert_eq!(doc.max_scroll(), 166.0);
    assert_eq!(doc.units_in(0.0, 150.0), 0..2);
    assert_eq!(doc.units_in(120.0, 270.0), 1..3);
    assert_eq!(doc.units_in(300.0, 450.0), 2..3);
    let tree = doc.layout_viewport(120.0, &fonts, &mut text);
    assert_eq!(tree.roots.len(), 2);
    let second = doc.unit_nodes(1).next().unwrap();
    assert_rect!(box_of(&tree, second).rect, 8.0, 108.0, 284.0, 100.0);
}

#[test]
fn more_than_max_units_blocks_are_grouped_not_dropped() {
    use renderer::document::MAX_UNITS;
    let n = MAX_UNITS + 5;
    let mut b = PageBuilder::new("about:test", 300, PageBuilder::ua_body());
    for i in 0..n {
        let h = 10.0 + (i % 3) as f32; // 10, 11, 12, ...
        let mut s = block(|s| s.height = Length::px(h));
        margins(&mut s, 4.0, 0.0, 6.0, 0.0); // sibling margins collapse to 6
        b.leaf(NodeKind::Div, Role::Generic, s);
    }
    let file = b.build();
    let fonts = no_fonts();
    let mut text = TextEngine::default();
    let doc = Document::from_file(file, (300.0, 150.0), &fonts, &mut text);
    assert_eq!(doc.top_level_len(), n);
    assert_eq!(doc.group(), 2);
    assert_eq!(doc.unit_count(), n.div_ceil(2));
    assert_eq!(doc.tops().len(), doc.unit_count() + 1);
    // Every block is laid out: total height is the sum of all blocks plus
    // the collapsed margins between them (body margin 8, first margin 8
    // collapses with body's... body has no border, so 8 ∨ 4 = 8).
    let heights: f32 = (0..n).map(|i| 10.0 + (i % 3) as f32).sum();
    let expected = 8.0 + heights + 6.0 * (n as f32 - 1.0) + 8.0;
    assert!(
        (doc.content_height() - expected).abs() < 0.5,
        "content {} expected {expected}",
        doc.content_height()
    );
    assert_eq!(doc.tops()[0], 8.0);
    // Unit 1 starts with block 2: 8 + 10 + 6 + 11 + 6.
    assert_eq!(doc.tops()[1], 41.0);
    // Both blocks of a visible unit are painted at the right places.
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    let nodes: Vec<u32> = doc.unit_nodes(1).collect();
    assert_eq!(nodes.len(), 2);
    assert_rect!(box_of(&tree, nodes[0]).rect, 8.0, 41.0, 284.0, 12.0);
    assert_rect!(box_of(&tree, nodes[1]).rect, 8.0, 59.0, 284.0, 10.0);
    // A unit in the middle of the page is found by its top.
    let mid = doc.tops()[100] + 1.0;
    assert_eq!(doc.units_in(mid, mid + 1.0), 100..101);
}

#[test]
fn replaced_element_keeps_aspect_ratio_under_max_width() {
    let mut b = PageBuilder::new("about:test", 300, block(|_| {}));
    let img = b.image(
        block(|s| s.max_width = Length::percent(50.0)),
        b"\x89PNG not really",
        page_format::ImageFormat::Png,
        400,
        200,
    );
    let file = b.build();
    let fonts = no_fonts();
    let mut text = TextEngine::default();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    // 400x200 clamped to 150 wide keeps 2:1.
    assert_rect!(box_of(&tree, img).rect, 0.0, 0.0, 150.0, 75.0);
    assert!(matches!(
        box_of(&tree, img).kind,
        BoxKind::Image { image: 0 }
    ));
}

#[test]
fn text_wraps_at_break_opportunities_and_aligns() {
    let Some(fonts) = system_fonts() else { return };
    let mut text = TextEngine::default();
    let face = fonts
        .face(FontFamily::Sans, FontWeight::Normal, FontStyle::Normal)
        .unwrap();
    let w_aaa_bbb = text.measure(&fonts, face, 16.0, "aaa bbb");
    let w_ccc = text.measure(&fonts, face, 16.0, "ccc");
    let m = text.metrics(&fonts, Some(face), 16.0);

    // Container just wide enough for "aaa bbb": "ccc" goes to line 2.
    let cw = w_aaa_bbb + 1.0;
    let mut b = PageBuilder::new("about:test", 300, block(|_| {}));
    let base = inline(|_| {});
    let p = b.open(
        NodeKind::P,
        Role::Paragraph,
        block(|s| {
            s.width = Length::px(cw);
            s.line_height = Length::px(20.0);
            s.text_align = TextAlign::Right;
        }),
    );
    b.text("aaa bbb ccc", base);
    b.close();
    let file = b.build();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);

    let runs: Vec<&renderer::layout::LayoutBox> = tree
        .boxes
        .iter()
        .filter(|b| matches!(b.kind, BoxKind::Text(_)))
        .collect();
    assert_eq!(runs.len(), 2, "two lines expected:\n{}", tree.dump());
    assert_rect!(box_of(&tree, p).rect, 0.0, 0.0, cw, 40.0);
    let BoxKind::Text(r0) = &runs[0].kind else {
        unreachable!()
    };
    let BoxKind::Text(r1) = &runs[1].kind else {
        unreachable!()
    };
    assert_eq!(r0.glyphs.len(), 8); // "aaa bbb " incl. hanging space
    assert_eq!(r1.glyphs.len(), 3);
    // Right-aligned: line 1 ends (ignoring the hanging space) at cw, line 2 too.
    assert!(approx(runs[0].rect.x + w_aaa_bbb, cw), "{}", tree.dump());
    assert!(approx(runs[1].rect.x + w_ccc, cw));
    // Baselines: half-leading around the font's ascent+descent in 20px lines.
    let half = (20.0 - m.ascent - m.descent) / 2.0;
    assert!(approx(runs[0].rect.y, half));
    assert!(approx(runs[1].rect.y, 20.0 + half));
    assert!(approx(r0.baseline, m.ascent));
}

#[test]
fn nowrap_pre_and_br() {
    let Some(fonts) = system_fonts() else { return };
    let mut text = TextEngine::default();
    let mut b = PageBuilder::new("about:test", 300, block(|_| {}));
    let base = inline(|_| {});
    // 1) nowrap never breaks even in a 10px box.
    let nowrap = b.open(
        NodeKind::P,
        Role::Paragraph,
        block(|s| {
            s.width = Length::px(10.0);
            s.white_space = css_subset::WhiteSpace::Nowrap;
            s.line_height = Length::px(20.0);
        }),
    );
    b.text(
        "one two three",
        inline(|s| s.white_space = css_subset::WhiteSpace::Nowrap),
    );
    b.close();
    // 2) pre keeps newlines and spaces; 3 lines.
    let pre = b.open(
        NodeKind::Pre,
        Role::Code,
        block(|s| {
            s.white_space = css_subset::WhiteSpace::Pre;
            s.line_height = Length::px(20.0);
        }),
    );
    b.text(
        "a\n  b\nc",
        inline(|s| s.white_space = css_subset::WhiteSpace::Pre),
    );
    b.close();
    // 3) <br> forces a break; a trailing <br> adds no line.
    let brp = b.open(
        NodeKind::P,
        Role::Paragraph,
        block(|s| s.line_height = Length::px(20.0)),
    );
    b.text("x", base);
    b.leaf(NodeKind::LineBreak, Role::LineBreak, base);
    b.text("y", base);
    b.leaf(NodeKind::LineBreak, Role::LineBreak, base);
    b.close();
    let file = b.build();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    assert!(
        approx(box_of(&tree, nowrap).rect.h, 20.0),
        "{}",
        tree.dump()
    );
    assert!(approx(box_of(&tree, pre).rect.h, 60.0), "{}", tree.dump());
    assert!(approx(box_of(&tree, brp).rect.h, 40.0), "{}", tree.dump());
}

#[test]
fn list_items_get_outside_markers() {
    let Some(fonts) = system_fonts() else { return };
    let mut text = TextEngine::default();
    let mut b = PageBuilder::new("about:test", 300, block(|_| {}));
    let ul = b.open(
        NodeKind::Ul,
        Role::List,
        block(|s| s.padding[Side::Left as usize] = Length::px(40.0)),
    );
    let li = b.open(
        NodeKind::Li,
        Role::ListItem,
        style(|s| {
            s.display = Display::ListItem;
            s.list_style_type = ListStyleType::Decimal;
        }),
    );
    b.text("item", inline(|_| {}));
    b.close();
    b.close();
    let file = b.build();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    let li_box = box_of(&tree, li);
    assert!(approx(li_box.rect.x, 40.0));
    let _ = ul;
    // Two text runs for the li: content and marker; the marker ends at the
    // content edge and is left of it.
    let runs: Vec<&renderer::layout::LayoutBox> = tree
        .boxes
        .iter()
        .filter(|b| b.node == li && matches!(b.kind, BoxKind::Text(_)))
        .collect();
    assert_eq!(runs.len(), 1, "{}", tree.dump());
    let marker = runs[0];
    assert!(
        marker.rect.x < 40.0 && approx(marker.rect.right(), 40.0),
        "{}",
        tree.dump()
    );
    let BoxKind::Text(t) = &marker.kind else {
        unreachable!()
    };
    assert_eq!(t.glyphs.len(), 3); // "1. "
}

#[test]
fn link_hit_testing_walks_up_to_the_anchor() {
    let Some(fonts) = system_fonts() else { return };
    let mut text = TextEngine::default();
    let mut b = PageBuilder::new("about:test", 300, block(|_| {}));
    b.open(
        NodeKind::P,
        Role::Paragraph,
        block(|s| s.line_height = Length::px(20.0)),
    );
    b.link("https://example.test/x", "go", inline(|_| {}));
    b.close();
    let file = b.build();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    assert_eq!(
        doc.link_at(&tree, 3.0, 10.0),
        Some("https://example.test/x")
    );
    assert_eq!(doc.link_at(&tree, 200.0, 10.0), None);
}

#[test]
fn vertical_align_middle_image_in_text() {
    let mut b = PageBuilder::new("about:test", 300, block(|_| {}));
    let p = b.open(
        NodeKind::P,
        Role::Paragraph,
        block(|s| s.line_height = Length::px(20.0)),
    );
    let img = b.image(
        inline(|s| {
            s.width = Length::px(10.0);
            s.height = Length::px(30.0);
            s.vertical_align = VerticalAlign::Top;
        }),
        b"",
        page_format::ImageFormat::Png,
        10,
        30,
    );
    b.close();
    let file = b.build();
    let fonts = no_fonts();
    let mut text = TextEngine::default();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    // A 30px top-aligned atomic stretches the 20px line to 30px.
    assert_rect!(box_of(&tree, p).rect, 0.0, 0.0, 300.0, 30.0);
    assert_rect!(box_of(&tree, img).rect, 0.0, 0.0, 10.0, 30.0);
}

#[test]
fn max_width_clamped_block_centres_with_auto_margins() {
    // A `width: auto` block whose fill width violates `max-width` re-runs
    // the width rules with the clamped width (CSS 2.1 §10.4), so `auto`
    // margins centre it instead of collapsing to zero.
    let mut b = PageBuilder::new("about:test", 400, block(|_| {}));
    let page = b.leaf(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            s.max_width = Length::px(200.0);
            s.margin[Side::Left as usize] = Length::AUTO;
            s.margin[Side::Right as usize] = Length::AUTO;
            s.height = Length::px(10.0);
        }),
    );
    let left = b.leaf(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            s.max_width = Length::px(200.0);
            s.height = Length::px(10.0);
        }),
    );
    let file = b.build();
    let mut text = TextEngine::default();
    let fonts = no_fonts();
    let doc = Document::from_file(file, (400.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    assert_rect!(box_of(&tree, page).rect, 100.0, 0.0, 200.0, 10.0);
    assert_rect!(box_of(&tree, left).rect, 0.0, 10.0, 200.0, 10.0);
}

#[test]
fn newline_across_an_inline_boundary_in_pre_breaks_once() {
    // `<pre><span>a</span>\n<span>b</span></pre>`: the mandatory break
    // after "\n" belongs to the text piece that ends there; the following
    // inline box must not claim it again (that produced an empty line).
    let Some(fonts) = system_fonts() else { return };
    let mut text = TextEngine::default();
    let mut b = PageBuilder::new("about:test", 300, block(|_| {}));
    let pre_inline = inline(|s| s.white_space = css_subset::WhiteSpace::Pre);
    let pre = b.open(
        NodeKind::Pre,
        Role::Code,
        block(|s| {
            s.white_space = css_subset::WhiteSpace::Pre;
            s.line_height = Length::px(20.0);
        }),
    );
    b.open(
        NodeKind::Span,
        Role::Generic,
        inline(|s| {
            s.white_space = css_subset::WhiteSpace::Pre;
            s.color = Rgba::rgb(200, 0, 0);
        }),
    );
    b.text("a", pre_inline);
    b.close();
    b.text("\n", pre_inline);
    b.open(
        NodeKind::Span,
        Role::Generic,
        inline(|s| {
            s.white_space = css_subset::WhiteSpace::Pre;
            s.color = Rgba::rgb(0, 0, 200);
        }),
    );
    b.text("b", pre_inline);
    b.close();
    b.text("\n", pre_inline);
    b.close();
    let file = b.build();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    assert!(approx(box_of(&tree, pre).rect.h, 40.0), "{}", tree.dump());
}

#[test]
fn inline_opacity_reaches_text_runs() {
    let Some(fonts) = system_fonts() else { return };
    let mut text = TextEngine::default();
    let mut b = PageBuilder::new("about:test", 300, block(|_| {}));
    b.open(NodeKind::P, Role::Paragraph, block(|_| {}));
    b.open(NodeKind::Span, Role::Generic, inline(|s| s.opacity = 0.5));
    let t = b.text("faded", inline(|_| {}));
    b.close();
    b.close();
    let file = b.build();
    let doc = Document::from_file(file, (300.0, 300.0), &fonts, &mut text);
    let tree = doc.layout_viewport(0.0, &fonts, &mut text);
    let run = box_of(&tree, t);
    let BoxKind::Text(r) = &run.kind else {
        panic!("{}", tree.dump())
    };
    assert!(approx(r.opacity, 0.5), "{}", tree.dump());
}
