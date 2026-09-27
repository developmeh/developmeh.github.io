//! Golden-ish paint tests: paint tiny pages through the strip path and
//! check hand-computed pixels. Block-only pages need no font.

use css_subset::{Length, Rgba};
use page_format::{ImageFormat, NodeKind, Role};
use renderer::testing::{block, border, margins, PageBuilder};
use renderer::{
    paint_viewport, Document, FontSet, FontSource, PaintParams, StripBuffer, TextEngine,
};

/// Paints and returns the full RGBA image plus its width.
fn render(
    doc: &Document,
    fonts: &FontSet,
    text: &mut TextEngine,
    scroll_y: f32,
    dpr: f32,
) -> (Vec<u8>, u32, u32) {
    let (vw, vh) = doc.viewport();
    let params = PaintParams {
        scroll_y,
        viewport_w: vw,
        viewport_h: vh,
        dpr,
        background: doc.background(),
    };
    let (w, h) = params.device_size();
    let tree = doc.layout_viewport(scroll_y, fonts, text);
    let mut strip = StripBuffer::new(w);
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    let mut bands = 0;
    paint_viewport(
        doc.page(),
        fonts,
        text,
        &tree,
        &params,
        &mut strip,
        |bytes, bw, rows| {
            assert_eq!(bw, w);
            assert_eq!(bytes.len(), (bw * rows * 4) as usize);
            out.extend_from_slice(bytes);
            bands += 1;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(out.len(), (w * h * 4) as usize);
    assert!(bands >= h.div_ceil(64) as usize);
    (out, w, h)
}

fn px(img: &(Vec<u8>, u32, u32), x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.1 + x) * 4) as usize;
    [img.0[i], img.0[i + 1], img.0[i + 2], img.0[i + 3]]
}

fn boxes_page() -> Document {
    // 64x48 viewport, blue body, a red box with a 2px green border at
    // (10, 5) sized 24x14 (border box).
    let mut b = PageBuilder::new(
        "about:test",
        64,
        block(|s| s.background_color = Rgba::rgb(0, 0, 255)),
    );
    b.leaf(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            margins(s, 5.0, 0.0, 0.0, 10.0);
            s.width = Length::px(20.0);
            s.height = Length::px(10.0);
            s.background_color = Rgba::rgb(255, 0, 0);
            border(s, 2.0, Rgba::rgb(0, 255, 0));
        }),
    );
    let fonts = FontSet::empty();
    let mut text = TextEngine::default();
    Document::from_file(b.build(), (64.0, 48.0), &fonts, &mut text)
}

#[test]
fn paints_background_borders_and_fill() {
    let doc = boxes_page();
    let fonts = FontSet::empty();
    let mut text = TextEngine::default();
    let img = render(&doc, &fonts, &mut text, 0.0, 1.0);
    let blue = [0, 0, 255, 255];
    let green = [0, 255, 0, 255];
    let red = [255, 0, 0, 255];
    assert_eq!(px(&img, 0, 0), blue);
    assert_eq!(px(&img, 9, 5), blue);
    assert_eq!(px(&img, 10, 5), green);
    assert_eq!(px(&img, 11, 6), green);
    assert_eq!(px(&img, 12, 7), red);
    assert_eq!(px(&img, 31, 16), red);
    assert_eq!(px(&img, 33, 18), green);
    assert_eq!(px(&img, 34, 5), blue);
    assert_eq!(px(&img, 10, 19), blue);
    assert_eq!(px(&img, 63, 47), blue);
}

#[test]
fn scrolling_and_dpr_move_and_scale_pixels() {
    let doc = boxes_page();
    let fonts = FontSet::empty();
    let mut text = TextEngine::default();
    let red = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    let blue = [0, 0, 255, 255];

    // Scrolled by 5 CSS px: the box top is now at y=0.
    let img = render(&doc, &fonts, &mut text, 5.0, 1.0);
    assert_eq!(px(&img, 10, 0), green);
    assert_eq!(px(&img, 12, 2), red);
    assert_eq!(px(&img, 12, 14), blue);

    // DPR 2: everything doubles; the image is 128x96 and needs two bands.
    let img = render(&doc, &fonts, &mut text, 0.0, 2.0);
    assert_eq!((img.1, img.2), (128, 96));
    assert_eq!(px(&img, 19, 10), blue);
    assert_eq!(px(&img, 20, 10), green);
    assert_eq!(px(&img, 23, 13), green);
    assert_eq!(px(&img, 24, 14), red);
    assert_eq!(px(&img, 67, 10), green);
    assert_eq!(px(&img, 68, 10), blue);
}

#[test]
fn strip_bands_stitch_seamlessly() {
    // A tall page whose single box spans many 64-row bands.
    let mut b = PageBuilder::new("about:test", 32, block(|_| {}));
    b.leaf(
        NodeKind::Div,
        Role::Generic,
        block(|s| {
            s.height = Length::px(300.0);
            s.background_color = Rgba::rgb(10, 20, 30);
            border(s, 1.0, Rgba::BLACK);
        }),
    );
    let fonts = FontSet::empty();
    let mut text = TextEngine::default();
    let doc = Document::from_file(b.build(), (32.0, 320.0), &fonts, &mut text);
    let img = render(&doc, &fonts, &mut text, 0.0, 1.0);
    for y in 1..301 {
        assert_eq!(px(&img, 0, y), [0, 0, 0, 255], "left border at row {y}");
        assert_eq!(px(&img, 16, y), [10, 20, 30, 255], "fill at row {y}");
        assert_eq!(px(&img, 31, y), [0, 0, 0, 255], "right border at row {y}");
    }
    assert_eq!(px(&img, 16, 0), [0, 0, 0, 255]);
    assert_eq!(px(&img, 16, 301), [0, 0, 0, 255]);
    assert_eq!(px(&img, 16, 302), [255, 255, 255, 255]);
}

#[test]
fn images_decode_at_paint_and_unsupported_ones_are_placeholders() {
    let png = {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, 2, 1);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[255, 0, 0, 255, 0, 0, 255, 255])
            .unwrap();
        drop(w);
        out
    };
    let mut b = PageBuilder::new("about:test", 40, block(|_| {}));
    b.image(
        block(|s| {
            s.width = Length::px(8.0);
            s.height = Length::px(4.0);
        }),
        &png,
        ImageFormat::Png,
        2,
        1,
    );
    b.image(
        block(|s| {
            s.width = Length::px(8.0);
            s.height = Length::px(4.0);
        }),
        b"GIF89a",
        ImageFormat::Unsupported,
        8,
        4,
    );
    let fonts = FontSet::empty();
    let mut text = TextEngine::default();
    let doc = Document::from_file(b.build(), (40.0, 20.0), &fonts, &mut text);
    let img = render(&doc, &fonts, &mut text, 0.0, 1.0);
    // Left half red, right half blue, scaled 4x.
    assert_eq!(px(&img, 0, 0), [255, 0, 0, 255]);
    assert_eq!(px(&img, 3, 3), [255, 0, 0, 255]);
    assert_eq!(px(&img, 4, 0), [0, 0, 255, 255]);
    assert_eq!(px(&img, 7, 3), [0, 0, 255, 255]);
    assert_eq!(px(&img, 8, 0), [255, 255, 255, 255]);
    // Placeholder: grey fill with a darker outline.
    assert_eq!(px(&img, 0, 4), [0x99, 0x99, 0x99, 255]);
    assert_eq!(px(&img, 3, 5), [0xEE, 0xEE, 0xEE, 255]);
    assert_eq!(px(&img, 7, 7), [0x99, 0x99, 0x99, 255]);
}

#[test]
fn text_paints_dark_pixels_inside_its_run_box() {
    let fonts = FontSet::load(&FontSource::Auto).unwrap();
    if fonts.is_empty() {
        eprintln!("no system fonts; skipping");
        return;
    }
    let mut text = TextEngine::default();
    let mut b = PageBuilder::new("about:test", 200, block(|_| {}));
    b.open(
        NodeKind::P,
        Role::Paragraph,
        block(|s| s.line_height = Length::px(24.0)),
    );
    b.text("Hello", renderer::testing::inline(|_| {}));
    b.close();
    let doc = Document::from_file(b.build(), (200.0, 40.0), &fonts, &mut text);
    let img = render(&doc, &fonts, &mut text, 0.0, 1.0);
    let dark = |x: u32, y: u32| px(&img, x, y)[0] < 128;
    let count_dark_in = |x0: u32, x1: u32, y0: u32, y1: u32| {
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| dark(x, y))
            .count()
    };
    // Glyph ink inside the first line, none far to the right or below.
    assert!(count_dark_in(0, 60, 0, 24) > 20);
    assert_eq!(count_dark_in(120, 200, 0, 40), 0);
    assert_eq!(count_dark_in(0, 200, 30, 40), 0);
    assert!(text.cache().bytes() > 0);
}
