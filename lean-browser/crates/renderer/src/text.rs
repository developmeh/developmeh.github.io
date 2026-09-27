//! Text shaping, font metrics and the byte-capped glyph cache (plan §7).
//!
//! One `swash` [`ShapeContext`] and one [`ScaleContext`] live for the whole
//! process and are reused for every run. Glyph masks are A8 bitmaps keyed
//! by `(face, size * 64, glyph id)` and evicted least-recently-used when
//! the cache exceeds its byte cap (256 KB by default).

use std::collections::HashMap;

use lean_alloc::{scope, Tag};
use swash::scale::{Render, ScaleContext, Source};
use swash::shape::ShapeContext;
use swash::zeno::Format;
use swash::GlyphId;

use crate::fonts::{FaceId, FontSet};

/// Default glyph cache cap (plan §7).
pub const GLYPH_CACHE_CAP: usize = 256 * 1024;

/// Font metrics scaled to a pixel size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FontMetricsPx {
    /// Baseline to top of the alignment box.
    pub ascent: f32,
    /// Baseline to bottom of the alignment box (positive).
    pub descent: f32,
    /// Recommended extra line spacing.
    pub leading: f32,
    /// Height of a lower-case `x`.
    pub x_height: f32,
    /// Underline position relative to the baseline (negative = below).
    pub underline_offset: f32,
    /// Strike-through position relative to the baseline.
    pub strikeout_offset: f32,
    /// Underline / strike-through thickness.
    pub stroke_size: f32,
}

impl FontMetricsPx {
    /// `line-height: normal`.
    pub fn natural_line_height(&self) -> f32 {
        self.ascent + self.descent + self.leading
    }

    /// Metrics used when no font is available: keeps layout meaningful.
    pub fn synthetic(size: f32) -> FontMetricsPx {
        FontMetricsPx {
            ascent: size * 0.9,
            descent: size * 0.25,
            leading: 0.0,
            x_height: size * 0.5,
            underline_offset: -size * 0.1,
            strikeout_offset: size * 0.3,
            stroke_size: (size / 14.0).max(1.0),
        }
    }
}

/// One shaped glyph in a run.
#[derive(Clone, Copy, Debug)]
pub struct ShapedGlyph {
    /// Glyph id in the face.
    pub id: GlyphId,
    /// Horizontal advance in px.
    pub advance: f32,
    /// Horizontal offset in px.
    pub x: f32,
    /// Vertical offset in px.
    pub y: f32,
    /// Byte offset of the cluster's first character in the shaped text.
    pub cluster: u32,
}

/// A rasterized glyph mask.
#[derive(Debug)]
pub struct CachedGlyph {
    /// Horizontal offset of the bitmap from the glyph origin.
    pub left: i32,
    /// Vertical offset of the bitmap top above the baseline.
    pub top: i32,
    /// Bitmap width.
    pub width: u32,
    /// Bitmap height.
    pub height: u32,
    /// `width * height` coverage bytes.
    pub data: Vec<u8>,
    tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlyphKey {
    face: FaceId,
    size_x64: u32,
    glyph: GlyphId,
}

/// LRU-by-bytes glyph mask cache.
pub struct GlyphCache {
    map: HashMap<GlyphKey, CachedGlyph>,
    bytes: usize,
    cap: usize,
    tick: u64,
    hits: u64,
    misses: u64,
}

impl GlyphCache {
    /// A cache holding at most `cap` bytes of mask data.
    pub fn new(cap: usize) -> GlyphCache {
        GlyphCache {
            map: HashMap::new(),
            bytes: 0,
            cap,
            tick: 0,
            hits: 0,
            misses: 0,
        }
    }

    /// Bytes of mask data currently held.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Number of cached glyphs.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// `(hits, misses)` since creation.
    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }

    fn get(&mut self, key: GlyphKey) -> Option<&CachedGlyph> {
        self.tick += 1;
        let tick = self.tick;
        match self.map.get_mut(&key) {
            Some(g) => {
                g.tick = tick;
                self.hits += 1;
                Some(&*g)
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    fn insert(&mut self, key: GlyphKey, mut glyph: CachedGlyph) -> &CachedGlyph {
        let _tag = scope(Tag::GlyphCache);
        // A single glyph larger than the cap is not cached; the caller gets a
        // temporary that is dropped right after painting.
        let size = glyph.data.len();
        while self.bytes + size > self.cap && !self.map.is_empty() {
            let oldest = self
                .map
                .iter()
                .min_by_key(|(_, g)| g.tick)
                .map(|(k, _)| *k)
                .expect("non-empty");
            if let Some(g) = self.map.remove(&oldest) {
                self.bytes -= g.data.len();
            }
        }
        self.tick += 1;
        glyph.tick = self.tick;
        self.bytes += size;
        self.map.entry(key).or_insert(glyph)
    }
}

/// Shaping and rasterization front end.
pub struct TextEngine {
    shape: ShapeContext,
    scale: ScaleContext,
    cache: GlyphCache,
}

impl Default for TextEngine {
    fn default() -> Self {
        TextEngine::new(GLYPH_CACHE_CAP)
    }
}

impl TextEngine {
    /// Creates the two swash contexts and an empty cache.
    pub fn new(cache_cap: usize) -> TextEngine {
        let _tag = scope(Tag::Text);
        TextEngine {
            shape: ShapeContext::with_max_entries(4),
            scale: ScaleContext::with_max_entries(4),
            cache: GlyphCache::new(cache_cap),
        }
    }

    /// The glyph cache.
    pub fn cache(&self) -> &GlyphCache {
        &self.cache
    }

    /// Scaled metrics of a face, or synthetic ones when the face is absent.
    pub fn metrics(&self, fonts: &FontSet, face: Option<FaceId>, size: f32) -> FontMetricsPx {
        let Some(font) = face.and_then(|f| fonts.font_ref(f)) else {
            return FontMetricsPx::synthetic(size);
        };
        let m = font.metrics(&[]).scale(size);
        FontMetricsPx {
            ascent: m.ascent,
            descent: m.descent,
            leading: m.leading,
            x_height: m.x_height,
            underline_offset: m.underline_offset,
            strikeout_offset: m.strikeout_offset,
            stroke_size: m.stroke_size.max(1.0),
        }
    }

    /// Splits `text` into maximal runs a single face can cover, starting
    /// from `primary` and falling back to any other face in the set.
    /// Whitespace and control characters stay with the current run.
    pub fn segment_by_coverage(
        &self,
        fonts: &FontSet,
        primary: FaceId,
        text: &str,
    ) -> Vec<(FaceId, std::ops::Range<usize>)> {
        let mut out: Vec<(FaceId, std::ops::Range<usize>)> = Vec::new();
        let charmaps: Vec<(FaceId, swash::Charmap<'_>)> = std::iter::once(primary)
            .chain(fonts.face_ids().filter(|&f| f != primary))
            .filter_map(|f| fonts.font_ref(f).map(|r| (f, r.charmap())))
            .collect();
        if charmaps.is_empty() {
            return vec![(primary, 0..text.len())];
        }
        let mut current = primary;
        for (i, ch) in text.char_indices() {
            let face = if ch.is_whitespace() || ch.is_control() {
                current
            } else {
                charmaps
                    .iter()
                    .find(|(_, cm)| cm.map(ch) != 0)
                    .map(|(f, _)| *f)
                    .unwrap_or(primary)
            };
            match out.last_mut() {
                Some((f, range)) if *f == face => range.end = i + ch.len_utf8(),
                _ => out.push((face, i..i + ch.len_utf8())),
            }
            current = face;
        }
        if out.is_empty() {
            out.push((primary, 0..0));
        }
        out
    }

    /// Shapes `text` with one face at `size` px, appending to `out`.
    /// `cluster` offsets are relative to `text`.
    pub fn shape(
        &mut self,
        fonts: &FontSet,
        face: FaceId,
        size: f32,
        text: &str,
        out: &mut Vec<ShapedGlyph>,
    ) {
        let Some(font) = fonts.font_ref(face) else {
            return;
        };
        let _tag = scope(Tag::Text);
        let mut shaper = self.shape.builder(font).size(size).build();
        shaper.add_str(text);
        shaper.shape_with(|cluster| {
            for g in cluster.glyphs {
                out.push(ShapedGlyph {
                    id: g.id,
                    advance: g.advance,
                    x: g.x,
                    y: g.y,
                    cluster: cluster.source.start,
                });
            }
        });
    }

    /// Advance width of `text` in one face (no fallback), for markers and
    /// measurement helpers.
    pub fn measure(&mut self, fonts: &FontSet, face: FaceId, size: f32, text: &str) -> f32 {
        let mut glyphs = Vec::new();
        self.shape(fonts, face, size, text, &mut glyphs);
        glyphs.iter().map(|g| g.advance).sum()
    }

    /// Rasterizes (or fetches) the A8 mask for a glyph at `size` device px.
    pub fn glyph(
        &mut self,
        fonts: &FontSet,
        face: FaceId,
        size: f32,
        id: GlyphId,
    ) -> Option<&CachedGlyph> {
        let key = GlyphKey {
            face,
            size_x64: (size * 64.0).round() as u32,
            glyph: id,
        };
        // Two lookups keep the borrow checker happy without unsafe or a
        // second hash of the key on the hit path.
        if self.cache.get(key).is_some() {
            return self.cache.map.get(&key);
        }
        let font = fonts.font_ref(face)?;
        let image = {
            let _tag = scope(Tag::Text);
            let mut scaler = self.scale.builder(font).size(size).hint(true).build();
            Render::new(&[Source::Outline])
                .format(Format::Alpha)
                .render(&mut scaler, id)?
        };
        let glyph = CachedGlyph {
            left: image.placement.left,
            top: image.placement.top,
            width: image.placement.width,
            height: image.placement.height,
            data: image.data,
            tick: 0,
        };
        Some(self.cache.insert(key, glyph))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::FontSource;

    fn fonts() -> Option<FontSet> {
        let set = FontSet::load(&FontSource::Auto).ok()?;
        if set.is_empty() {
            eprintln!("no system fonts; skipping");
            None
        } else {
            Some(set)
        }
    }

    #[test]
    fn cache_evicts_by_bytes() {
        let mut c = GlyphCache::new(100);
        for i in 0..10u16 {
            let key = GlyphKey {
                face: 0,
                size_x64: 1,
                glyph: i,
            };
            c.insert(
                key,
                CachedGlyph {
                    left: 0,
                    top: 0,
                    width: 5,
                    height: 6,
                    data: vec![0; 30],
                    tick: 0,
                },
            );
            assert!(c.bytes() <= 100);
        }
        assert_eq!(c.len(), 3);
        assert!(c
            .get(GlyphKey {
                face: 0,
                size_x64: 1,
                glyph: 9
            })
            .is_some());
        assert!(c
            .get(GlyphKey {
                face: 0,
                size_x64: 1,
                glyph: 0
            })
            .is_none());
    }

    #[test]
    fn synthetic_metrics_without_fonts() {
        let engine = TextEngine::default();
        let m = engine.metrics(&FontSet::empty(), None, 16.0);
        assert!(m.natural_line_height() > 16.0);
        let mut engine = engine;
        let mut out = Vec::new();
        engine.shape(&FontSet::empty(), 0, 16.0, "hello", &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn shapes_and_rasterizes_with_system_fonts() {
        let Some(fonts) = fonts() else { return };
        let mut engine = TextEngine::default();
        let face = fonts
            .face(
                css_subset::FontFamily::Sans,
                css_subset::FontWeight::Normal,
                css_subset::FontStyle::Normal,
            )
            .unwrap();
        let mut out = Vec::new();
        engine.shape(&fonts, face, 16.0, "Hello, world", &mut out);
        assert_eq!(out.len(), 12);
        assert_eq!(out[7].cluster, 7);
        let width: f32 = out.iter().map(|g| g.advance).sum();
        assert!(width > 60.0 && width < 120.0, "width {width}");
        let m = engine.metrics(&fonts, Some(face), 16.0);
        assert!(m.ascent > 10.0 && m.descent > 2.0);

        let g = engine.glyph(&fonts, face, 16.0, out[0].id).unwrap();
        assert!(g.width > 0 && g.height > 0);
        assert_eq!(g.data.len(), (g.width * g.height) as usize);
        assert!(g.data.iter().any(|&a| a > 0));
        let (_, misses) = engine.cache().stats();
        engine.glyph(&fonts, face, 16.0, out[0].id).unwrap();
        assert_eq!(engine.cache().stats().1, misses, "second lookup is a hit");
        assert!(engine.cache().bytes() > 0);
    }

    #[test]
    fn coverage_segments_fall_back_across_faces() {
        let Some(fonts) = fonts() else { return };
        let engine = TextEngine::default();
        let face = fonts
            .face(
                css_subset::FontFamily::Sans,
                css_subset::FontWeight::Normal,
                css_subset::FontStyle::Normal,
            )
            .unwrap();
        let segs = engine.segment_by_coverage(&fonts, face, "plain ascii text");
        assert_eq!(segs, vec![(face, 0..16)]);
        let segs = engine.segment_by_coverage(&fonts, face, "");
        assert_eq!(segs.len(), 1);
    }
}
