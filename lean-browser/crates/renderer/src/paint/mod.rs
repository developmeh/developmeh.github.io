//! Strip-buffer painting (plan §7).
//!
//! The viewport is painted in horizontal bands of at most 64 device rows
//! (or fewer so the band never exceeds 320 KB). For every band the layout
//! tree is walked iteratively and every box whose device rectangle touches
//! the band is drawn with `tiny-skia` (rectangles, rounded rectangles) or
//! by hand (glyph masks, decoded images), then the band is handed to the
//! sink: a PNG stream writer in headless mode or the window surface.
//!
//! Pixels are premultiplied RGBA8 (tiny-skia's native format). The canvas
//! is always opaque, so the bytes double as straight RGBA for PNG output.

pub mod image;

use css_subset::{BorderStyle, ComputedStyle, Rgba, TextDecoration, Visibility};
use lean_alloc::{scope, Tag};
use page_format::ArchivedPage;
use tiny_skia::{FillRule, Paint, PathBuilder, PixmapMut, Transform};

use crate::fonts::FontSet;
use crate::layout::{BoxKind, LayoutBox, LayoutTree, Rect, TextRun, NONE};
use crate::text::TextEngine;

/// Strip buffer cap from plan §7.
pub const STRIP_BYTES_CAP: usize = 320 * 1024;
/// Maximum rows per strip.
pub const STRIP_MAX_ROWS: u32 = 64;

/// The single strip allocation reused for every band and every paint.
pub struct StripBuffer {
    data: Vec<u8>,
    width: u32,
    rows: u32,
}

impl StripBuffer {
    /// A strip for a viewport `width` device px wide.
    pub fn new(width: u32) -> StripBuffer {
        let mut s = StripBuffer {
            data: Vec::new(),
            width: 0,
            rows: 0,
        };
        s.resize(width);
        s
    }

    /// Rows per band for a given width.
    pub fn rows_for(width: u32) -> u32 {
        let width = width.max(1) as usize;
        ((STRIP_BYTES_CAP / (width * 4)) as u32).clamp(1, STRIP_MAX_ROWS)
    }

    /// Re-targets the buffer to a new width (reallocating only if the
    /// byte size changes).
    pub fn resize(&mut self, width: u32) {
        let width = width.max(1);
        let rows = Self::rows_for(width);
        let bytes = width as usize * rows as usize * 4;
        if bytes != self.data.len() {
            let _tag = scope(Tag::Strip);
            self.data = vec![0; bytes];
        }
        self.width = width;
        self.rows = rows;
    }

    /// Rows per band.
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Width in device px.
    pub fn width(&self) -> u32 {
        self.width
    }
}

/// What to paint.
#[derive(Clone, Copy, Debug)]
pub struct PaintParams {
    /// Scroll offset in CSS px.
    pub scroll_y: f32,
    /// Viewport width in CSS px.
    pub viewport_w: f32,
    /// Viewport height in CSS px.
    pub viewport_h: f32,
    /// Device pixel ratio.
    pub dpr: f32,
    /// Canvas background.
    pub background: Rgba,
}

impl PaintParams {
    /// Viewport size in device px.
    pub fn device_size(&self) -> (u32, u32) {
        (
            (self.viewport_w * self.dpr).round().max(1.0) as u32,
            (self.viewport_h * self.dpr).round().max(1.0) as u32,
        )
    }
}

struct Painter<'p> {
    page: &'p ArchivedPage,
    fonts: &'p FontSet,
    text: &'p mut TextEngine,
    dpr: f32,
    scroll_y: f32,
    /// Device y of the current band's first row.
    band_y0: f32,
    width: u32,
    band_h: u32,
    /// One decoded image kept across bands (they are painted top-down, so
    /// this avoids re-decoding an image for every band it spans).
    image_cache: Option<(u32, u32, u32, image::Decoded)>,
}

/// Paints the viewport band by band, calling `sink(bytes, width, rows)`
/// for each band from the top down.
#[allow(clippy::too_many_arguments)]
pub fn paint_viewport<F>(
    page: &ArchivedPage,
    fonts: &FontSet,
    text: &mut TextEngine,
    tree: &LayoutTree,
    params: &PaintParams,
    strip: &mut StripBuffer,
    mut sink: F,
) -> Result<(), String>
where
    F: FnMut(&[u8], u32, u32) -> Result<(), String>,
{
    let (w, h) = params.device_size();
    strip.resize(w);
    let rows = strip.rows();
    let mut painter = Painter {
        page,
        fonts,
        text,
        dpr: params.dpr,
        scroll_y: params.scroll_y,
        band_y0: 0.0,
        width: w,
        band_h: rows,
        image_cache: None,
    };
    let bg = composite_over_white(params.background);
    let mut y = 0u32;
    while y < h {
        let band = rows.min(h - y);
        painter.band_y0 = y as f32;
        painter.band_h = band;
        let bytes = w as usize * band as usize * 4;
        let data = &mut strip.data[..bytes];
        {
            let mut pm = PixmapMut::from_bytes(data, w, band).ok_or("bad strip size")?;
            pm.fill(tiny_skia::Color::from_rgba8(bg.r, bg.g, bg.b, 255));
            painter.paint_tree(&mut pm, tree);
        }
        sink(data, w, band)?;
        y += band;
    }
    Ok(())
}

/// The canvas is opaque: a translucent background colour is composited
/// over white.
fn composite_over_white(c: Rgba) -> Rgba {
    if c.a == 255 {
        return c;
    }
    let a = u32::from(c.a);
    let f = |v: u8| ((u32::from(v) * a + 255 * (255 - a)) / 255) as u8;
    Rgba::rgb(f(c.r), f(c.g), f(c.b))
}

fn scale_alpha(c: Rgba, alpha: f32) -> Rgba {
    if alpha >= 1.0 {
        return c;
    }
    Rgba::new(
        c.r,
        c.g,
        c.b,
        (f32::from(c.a) * alpha.clamp(0.0, 1.0)).round() as u8,
    )
}

fn skia_rect(r: &Rect) -> Option<tiny_skia::Rect> {
    if r.w <= 0.0 || r.h <= 0.0 {
        return None;
    }
    tiny_skia::Rect::from_xywh(r.x, r.y, r.w, r.h)
}

/// Rounded rectangle path; `radii` are top-left, top-right, bottom-right,
/// bottom-left, already clamped.
fn rounded_path(r: &Rect, radii: [f32; 4]) -> Option<tiny_skia::Path> {
    const K: f32 = 0.552_284_8;
    let [tl, tr, br, bl] = radii;
    let (x0, y0, x1, y1) = (r.x, r.y, r.right(), r.bottom());
    let mut pb = PathBuilder::new();
    pb.move_to(x0 + tl, y0);
    pb.line_to(x1 - tr, y0);
    if tr > 0.0 {
        pb.cubic_to(x1 - tr + tr * K, y0, x1, y0 + tr - tr * K, x1, y0 + tr);
    }
    pb.line_to(x1, y1 - br);
    if br > 0.0 {
        pb.cubic_to(x1, y1 - br + br * K, x1 - br + br * K, y1, x1 - br, y1);
    }
    pb.line_to(x0 + bl, y1);
    if bl > 0.0 {
        pb.cubic_to(x0 + bl - bl * K, y1, x0, y1 - bl + bl * K, x0, y1 - bl);
    }
    pb.line_to(x0, y0 + tl);
    if tl > 0.0 {
        pb.cubic_to(x0, y0 + tl - tl * K, x0 + tl - tl * K, y0, x0 + tl, y0);
    }
    pb.close();
    pb.finish()
}

impl Painter<'_> {
    /// Page-space CSS rect to band device space.
    fn dev(&self, r: &Rect) -> Rect {
        Rect::new(
            r.x * self.dpr,
            (r.y - self.scroll_y) * self.dpr - self.band_y0,
            r.w * self.dpr,
            r.h * self.dpr,
        )
    }

    fn band_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width as f32, self.band_h as f32)
    }

    fn style(&self, id: u16) -> ComputedStyle {
        self.page.styles[id as usize].to_native()
    }

    fn paint_tree(&mut self, pm: &mut PixmapMut<'_>, tree: &LayoutTree) {
        let band = self.band_rect();
        // (box, clip in device space, accumulated opacity)
        let mut stack: Vec<(u32, Option<Rect>, f32)> =
            tree.roots.iter().rev().map(|&r| (r, None, 1.0)).collect();
        while let Some((idx, clip, alpha)) = stack.pop() {
            let b = &tree.boxes[idx as usize];
            let s = self.style(b.style);
            if s.visibility == Visibility::Hidden {
                continue;
            }
            let alpha = alpha * s.opacity.clamp(0.0, 1.0);
            let dev = self.dev(&b.rect);
            let visible_clip = match clip {
                Some(c) => c.intersect(&band),
                None => Some(band),
            };
            if let Some(vc) = visible_clip {
                let reach = match &b.kind {
                    // Glyphs and decorations may poke outside the run's box.
                    BoxKind::Text(t) => t.size * self.dpr * 0.5,
                    _ => 0.0,
                };
                let probe = Rect::new(
                    dev.x - reach,
                    dev.y - reach,
                    dev.w + 2.0 * reach,
                    dev.h + 2.0 * reach,
                );
                if probe.intersects(&vc) {
                    self.paint_box(pm, b, &s, &dev, &vc, alpha);
                }
            }
            // Children (overflow may escape the parent's rect, so they are
            // culled individually, not by the parent).
            let child_clip = if b.clip {
                let pad = self.dev(&b.padding_box());
                Some(match clip {
                    Some(c) => c.intersect(&pad).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
                    None => pad,
                })
            } else {
                clip
            };
            let mut child = b.first_child;
            let mut kids = Vec::new();
            while child != NONE {
                kids.push(child);
                child = tree.boxes[child as usize].next_sibling;
            }
            for &k in kids.iter().rev() {
                stack.push((k, child_clip, alpha));
            }
        }
    }

    fn paint_box(
        &mut self,
        pm: &mut PixmapMut<'_>,
        b: &LayoutBox,
        s: &ComputedStyle,
        dev: &Rect,
        clip: &Rect,
        alpha: f32,
    ) {
        match &b.kind {
            BoxKind::Block | BoxKind::Inline => {
                self.paint_background_and_borders(pm, b, s, dev, clip, alpha);
            }
            BoxKind::Text(run) => self.paint_text(pm, b, run, dev, clip, alpha),
            BoxKind::Image { image } => {
                self.paint_background_and_borders(pm, b, s, dev, clip, alpha);
                let content = self.dev(&b.content_box());
                self.paint_image(pm, *image, &content, clip, alpha);
            }
            BoxKind::Placeholder => {
                self.paint_background_and_borders(pm, b, s, dev, clip, alpha);
                let content = self.dev(&b.content_box());
                fill_rect(
                    pm,
                    &content,
                    clip,
                    scale_alpha(Rgba::rgb(0xEE, 0xEE, 0xEE), alpha),
                );
                stroke_rect(
                    pm,
                    &content,
                    clip,
                    scale_alpha(Rgba::rgb(0x99, 0x99, 0x99), alpha),
                    self.dpr,
                );
            }
            BoxKind::Control => {
                self.paint_background_and_borders(pm, b, s, dev, clip, alpha);
                let content = self.dev(&b.content_box());
                fill_rect(pm, &content, clip, scale_alpha(Rgba::WHITE, alpha));
                stroke_rect(
                    pm,
                    &content,
                    clip,
                    scale_alpha(Rgba::rgb(0x76, 0x76, 0x76), alpha),
                    self.dpr,
                );
            }
        }
    }

    fn paint_background_and_borders(
        &self,
        pm: &mut PixmapMut<'_>,
        b: &LayoutBox,
        s: &ComputedStyle,
        dev: &Rect,
        clip: &Rect,
        alpha: f32,
    ) {
        let bg = scale_alpha(s.background_color, alpha);
        let radii = self.radii(s, &b.rect);
        let rounded = radii.iter().any(|&r| r > 0.0);
        if !bg.is_transparent() {
            if rounded {
                fill_rounded(pm, dev, radii, clip, bg);
            } else {
                fill_rect(pm, dev, clip, bg);
            }
        }
        let bw = [
            b.border.top * self.dpr,
            b.border.right * self.dpr,
            b.border.bottom * self.dpr,
            b.border.left * self.dpr,
        ];
        if bw.iter().all(|&w| w <= 0.0) {
            return;
        }
        let colors: Vec<Rgba> = (0..4)
            .map(|i| {
                if s.border_style[i] == BorderStyle::None {
                    Rgba::TRANSPARENT
                } else {
                    scale_alpha(s.border_color[i], alpha)
                }
            })
            .collect();
        let uniform = colors.iter().all(|c| *c == colors[0]) && bw.iter().all(|&w| w == bw[0]);
        if rounded && uniform && !colors[0].is_transparent() {
            // Ring between the outer and inner rounded rects.
            let inner = Rect::new(
                dev.x + bw[3],
                dev.y + bw[0],
                dev.w - bw[1] - bw[3],
                dev.h - bw[0] - bw[2],
            );
            let inner_radii = [
                (radii[0] - bw[0]).max(0.0),
                (radii[1] - bw[0]).max(0.0),
                (radii[2] - bw[0]).max(0.0),
                (radii[3] - bw[0]).max(0.0),
            ];
            if let (Some(outer), Some(inner)) =
                (rounded_path(dev, radii), rounded_path(&inner, inner_radii))
            {
                let mut pb = PathBuilder::new();
                pb.push_path(&outer);
                pb.push_path(&inner);
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color_rgba8(colors[0].r, colors[0].g, colors[0].b, colors[0].a);
                    paint.anti_alias = true;
                    pm.fill_path(
                        &path,
                        &paint,
                        FillRule::EvenOdd,
                        Transform::identity(),
                        None,
                    );
                }
            }
            return;
        }
        // Per-side rectangles (dashed/dotted render solid, plan §5).
        let sides = [
            Rect::new(dev.x, dev.y, dev.w, bw[0]),
            Rect::new(dev.right() - bw[1], dev.y, bw[1], dev.h),
            Rect::new(dev.x, dev.bottom() - bw[2], dev.w, bw[2]),
            Rect::new(dev.x, dev.y, bw[3], dev.h),
        ];
        for (i, side) in sides.iter().enumerate() {
            if bw[i] > 0.0 && !colors[i].is_transparent() {
                fill_rect(pm, side, clip, colors[i]);
            }
        }
    }

    /// Border radii in device px, clamped so adjacent radii fit.
    fn radii(&self, s: &ComputedStyle, rect: &Rect) -> [f32; 4] {
        let mut r = [0.0f32; 4];
        for (i, l) in s.border_radius.iter().enumerate() {
            r[i] = l.resolve(rect.w, 0.0, 0.0).unwrap_or(0.0).max(0.0) * self.dpr;
        }
        let max = (rect.w.min(rect.h) * self.dpr / 2.0).max(0.0);
        for v in &mut r {
            *v = v.min(max);
        }
        r
    }

    fn paint_text(
        &mut self,
        pm: &mut PixmapMut<'_>,
        b: &LayoutBox,
        run: &TextRun,
        dev: &Rect,
        clip: &Rect,
        alpha: f32,
    ) {
        let color = scale_alpha(run.color, alpha);
        if color.is_transparent() {
            return;
        }
        let baseline = dev.y + run.baseline * self.dpr;
        if let Some(face) = run.face {
            let size = run.size * self.dpr;
            let data = pm.data_mut();
            for g in &run.glyphs {
                let gx = (dev.x + g.x * self.dpr).round() as i32;
                let gy = (baseline - g.y * self.dpr).round() as i32;
                let Some(mask) = self.text.glyph(self.fonts, face, size, g.id) else {
                    continue;
                };
                blend_mask(
                    data,
                    self.width,
                    self.band_h,
                    clip,
                    gx + mask.left,
                    gy - mask.top,
                    &mask.data,
                    mask.width,
                    mask.height,
                    color,
                );
            }
        }
        // Decorations.
        let deco = run.decoration;
        if deco.is_empty() {
            return;
        }
        let thickness = (run.metrics.stroke_size * self.dpr).max(1.0);
        let mut line = |y: f32| {
            fill_rect(
                pm,
                &Rect::new(dev.x, y.round(), dev.w, thickness),
                clip,
                color,
            );
        };
        if deco.contains(TextDecoration::UNDERLINE) {
            line(baseline - run.metrics.underline_offset * self.dpr);
        }
        if deco.contains(TextDecoration::LINE_THROUGH) {
            line(baseline - run.metrics.strikeout_offset * self.dpr);
        }
        if deco.contains(TextDecoration::OVERLINE) {
            line(dev.y);
        }
        let _ = b;
    }

    fn paint_image(
        &mut self,
        pm: &mut PixmapMut<'_>,
        image: u32,
        content: &Rect,
        clip: &Rect,
        alpha: f32,
    ) {
        let dw = content.w.round().max(1.0) as u32;
        let dh = content.h.round().max(1.0) as u32;
        let cached =
            matches!(&self.image_cache, Some((i, w, h, _)) if *i == image && *w == dw && *h == dh);
        if !cached {
            let img = &self.page.images[image as usize];
            let bytes = self.page.blob_of(img);
            match image::decode(bytes, img.format, dw, dh) {
                Ok(d) => self.image_cache = Some((image, dw, dh, d)),
                Err(_) => {
                    self.image_cache = None;
                    fill_rect(
                        pm,
                        content,
                        clip,
                        scale_alpha(Rgba::rgb(0xEE, 0xEE, 0xEE), alpha),
                    );
                    stroke_rect(
                        pm,
                        content,
                        clip,
                        scale_alpha(Rgba::rgb(0x99, 0x99, 0x99), alpha),
                        self.dpr,
                    );
                    return;
                }
            }
        }
        if let Some((_, _, _, d)) = &self.image_cache {
            blend_rgba(
                pm.data_mut(),
                self.width,
                self.band_h,
                clip,
                content.x.round() as i32,
                content.y.round() as i32,
                d,
                alpha,
            );
        }
    }
}

/// Fills `r ∩ clip` with a straight-alpha colour.
fn fill_rect(pm: &mut PixmapMut<'_>, r: &Rect, clip: &Rect, color: Rgba) {
    if color.is_transparent() {
        return;
    }
    let Some(r) = r.intersect(clip) else { return };
    let Some(sr) = skia_rect(&r) else { return };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = false;
    pm.fill_rect(sr, &paint, Transform::identity(), None);
}

/// 1 CSS px outline just inside `r`.
fn stroke_rect(pm: &mut PixmapMut<'_>, r: &Rect, clip: &Rect, color: Rgba, dpr: f32) {
    let w = dpr.max(1.0);
    if r.w < 2.0 * w || r.h < 2.0 * w {
        return;
    }
    fill_rect(pm, &Rect::new(r.x, r.y, r.w, w), clip, color);
    fill_rect(pm, &Rect::new(r.x, r.bottom() - w, r.w, w), clip, color);
    fill_rect(pm, &Rect::new(r.x, r.y, w, r.h), clip, color);
    fill_rect(pm, &Rect::new(r.right() - w, r.y, w, r.h), clip, color);
}

/// Fills a rounded rectangle (clipped only to the band, plan: rare).
fn fill_rounded(pm: &mut PixmapMut<'_>, r: &Rect, radii: [f32; 4], clip: &Rect, color: Rgba) {
    if !r.intersects(clip) {
        return;
    }
    let Some(path) = rounded_path(r, radii) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.r, color.g, color.b, color.a);
    paint.anti_alias = true;
    pm.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

/// Source-over of a straight colour through an A8 mask into premultiplied
/// RGBA8, clipped to `clip` and the buffer.
#[allow(clippy::too_many_arguments)]
fn blend_mask(
    data: &mut [u8],
    stride: u32,
    height: u32,
    clip: &Rect,
    x0: i32,
    y0: i32,
    mask: &[u8],
    mw: u32,
    mh: u32,
    color: Rgba,
) {
    let cx0 = clip.x.floor().max(0.0) as i32;
    let cy0 = clip.y.floor().max(0.0) as i32;
    let cx1 = (clip.right().ceil() as i32).min(stride as i32);
    let cy1 = (clip.bottom().ceil() as i32).min(height as i32);
    let ca = u32::from(color.a);
    for my in 0..mh as i32 {
        let y = y0 + my;
        if y < cy0 || y >= cy1 {
            continue;
        }
        for mx in 0..mw as i32 {
            let x = x0 + mx;
            if x < cx0 || x >= cx1 {
                continue;
            }
            let cov = u32::from(mask[(my as u32 * mw + mx as u32) as usize]);
            if cov == 0 {
                continue;
            }
            let sa = ca * cov / 255; // 0..255
            let i = ((y as u32 * stride + x as u32) * 4) as usize;
            let inv = 255 - sa;
            let px = &mut data[i..i + 4];
            px[0] = ((u32::from(color.r) * sa + u32::from(px[0]) * inv) / 255) as u8;
            px[1] = ((u32::from(color.g) * sa + u32::from(px[1]) * inv) / 255) as u8;
            px[2] = ((u32::from(color.b) * sa + u32::from(px[2]) * inv) / 255) as u8;
            px[3] = (sa + u32::from(px[3]) * inv / 255) as u8;
        }
    }
}

/// Source-over of a premultiplied RGBA8 image.
#[allow(clippy::too_many_arguments)]
fn blend_rgba(
    data: &mut [u8],
    stride: u32,
    height: u32,
    clip: &Rect,
    x0: i32,
    y0: i32,
    src: &image::Decoded,
    alpha: f32,
) {
    let cx0 = clip.x.floor().max(0.0) as i32;
    let cy0 = clip.y.floor().max(0.0) as i32;
    let cx1 = (clip.right().ceil() as i32).min(stride as i32);
    let cy1 = (clip.bottom().ceil() as i32).min(height as i32);
    let ga = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
    for sy in 0..src.height as i32 {
        let y = y0 + sy;
        if y < cy0 || y >= cy1 {
            continue;
        }
        for sx in 0..src.width as i32 {
            let x = x0 + sx;
            if x < cx0 || x >= cx1 {
                continue;
            }
            let si = ((sy as u32 * src.width + sx as u32) * 4) as usize;
            let s = &src.data[si..si + 4];
            let sa = u32::from(s[3]) * ga / 255;
            if sa == 0 {
                continue;
            }
            let inv = 255 - sa;
            let i = ((y as u32 * stride + x as u32) * 4) as usize;
            let px = &mut data[i..i + 4];
            for c in 0..3 {
                px[c] = ((u32::from(s[c]) * ga / 255) + u32::from(px[c]) * inv / 255) as u8;
            }
            px[3] = (sa + u32::from(px[3]) * inv / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_rows_respect_cap() {
        assert_eq!(StripBuffer::rows_for(1280), 64);
        assert_eq!(StripBuffer::rows_for(2560), 32);
        assert_eq!(StripBuffer::rows_for(100_000), 1);
        let s = StripBuffer::new(1280);
        assert!(s.data.len() <= STRIP_BYTES_CAP);
    }

    #[test]
    fn mask_blend_is_source_over() {
        let mut data = vec![255u8; 4 * 4]; // 2x2 white
        blend_mask(
            &mut data,
            2,
            2,
            &Rect::new(0.0, 0.0, 2.0, 2.0),
            0,
            0,
            &[255, 0, 128, 0],
            2,
            2,
            Rgba::BLACK,
        );
        assert_eq!(&data[0..4], &[0, 0, 0, 255]);
        assert_eq!(&data[4..8], &[255, 255, 255, 255]);
        assert_eq!(data[8], 255 - 128);
    }

    #[test]
    fn translucent_background_composites_over_white() {
        let c = composite_over_white(Rgba::new(0, 0, 0, 128));
        assert_eq!(c, Rgba::rgb(127, 127, 127));
    }
}
