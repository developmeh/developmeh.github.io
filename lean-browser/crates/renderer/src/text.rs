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

/// Bytes of mask data the default glyph cache holds. Together with the
/// fixed-size map (1024 buckets of 64 B + control bytes, ~67 KB) this
/// stays under plan §7's 256 KB glyph-cache component
/// ([`GLYPH_CACHE_BUDGET`]).
pub const GLYPH_CACHE_CAP: usize = 184 * 1024;

/// Maximum cached masks. The map is allocated once for twice this many
/// entries and never reallocates: hashbrown only rehashes in place when
/// the live count is at most half its capacity, which the entry cap
/// guarantees, so tombstones from evictions cannot trigger a resize.
pub const GLYPH_CACHE_ENTRIES: usize = 448;

/// The `lean-alloc` budget for `Tag::GlyphCache` (plan §7: 256 KB).
pub const GLYPH_CACHE_BUDGET: usize = 256 * 1024;

/// Largest glyph size (device px) that is rasterized at all. Larger sizes
/// draw nothing: a validated page may ask for any `font-size`, and the
/// rasterizer's scratch grows with the square of the size (a 60 000 px
/// glyph is a multi-GB mask). 1024 px keeps the transient mask at about
/// 1 MB; `Layouter::style` clamps CSS sizes to [`MAX_FONT_PX`] first.
pub const MAX_GLYPH_PX: f32 = 1024.0;

/// Largest `font-size` in CSS px the layouter honours (device size is
/// `MAX_FONT_PX * dpr`, at or under [`MAX_GLYPH_PX`] for `--dpr` ≤ 2).
pub const MAX_FONT_PX: f32 = 512.0;

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

/// LRU-by-bytes glyph mask cache. `bytes() <= cap` always holds: a mask
/// larger than the cap is never inserted; it is parked in a single
/// `oversize` slot that the next miss replaces.
pub struct GlyphCache {
    map: HashMap<GlyphKey, CachedGlyph>,
    bytes: usize,
    cap: usize,
    tick: u64,
    hits: u64,
    misses: u64,
    oversize: Option<CachedGlyph>,
}

impl GlyphCache {
    /// A cache holding at most `cap` bytes of mask data in at most
    /// [`GLYPH_CACHE_ENTRIES`] entries. The map is allocated here, once,
    /// under `Tag::GlyphCache`.
    pub fn new(cap: usize) -> GlyphCache {
        let _tag = scope(Tag::GlyphCache);
        GlyphCache {
            map: HashMap::with_capacity(2 * GLYPH_CACHE_ENTRIES),
            bytes: 0,
            cap,
            tick: 0,
            hits: 0,
            misses: 0,
            oversize: None,
        }
    }

    /// The byte cap.
    pub fn cap(&self) -> usize {
        self.cap
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

    /// Evicts least-recently-used masks until `size` more bytes and one
    /// more entry fit.
    fn make_room(&mut self, size: usize) {
        while (self.bytes + size > self.cap || self.map.len() >= GLYPH_CACHE_ENTRIES)
            && !self.map.is_empty()
        {
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
    }

    /// Inserts `glyph`, evicting older masks as needed. A glyph larger
    /// than the cap is not cached: it goes into the `oversize` slot (a
    /// temporary that the next miss replaces) so `bytes()` never exceeds
    /// the cap. The previous oversize glyph is dropped on every insert.
    fn insert(&mut self, key: GlyphKey, mut glyph: CachedGlyph) -> &CachedGlyph {
        self.oversize = None;
        let size = glyph.data.len();
        if size > self.cap {
            return self.oversize.insert(glyph);
        }
        let _tag = scope(Tag::GlyphCache);
        self.make_room(size);
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
    /// Sizes above [`MAX_GLYPH_PX`] (or not finite) yield `None`.
    pub fn glyph(
        &mut self,
        fonts: &FontSet,
        face: FaceId,
        size: f32,
        id: GlyphId,
    ) -> Option<&CachedGlyph> {
        if !(size > 0.0 && size <= MAX_GLYPH_PX) {
            return None;
        }
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
        // The rasterizer's scratch and its output are shaping/raster
        // scratch (`Tag::Text`); only the bytes the cache keeps are copied
        // under `Tag::GlyphCache`, so that tag measures the cache alone.
        let image = {
            let _tag = scope(Tag::Text);
            let mut scaler = self.scale.builder(font).size(size).hint(true).build();
            Render::new(&[Source::Outline])
                .format(Format::Alpha)
                .render(&mut scaler, id)?
        };
        let data = if image.data.len() <= self.cache.cap() {
            self.cache.make_room(image.data.len());
            let _tag = scope(Tag::GlyphCache);
            image.data.clone()
        } else {
            image.data
        };
        let glyph = CachedGlyph {
            left: image.placement.left,
            top: image.placement.top,
            width: image.placement.width,
            height: image.placement.height,
            data,
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
    fn oversize_glyph_is_not_cached() {
        let mut c = GlyphCache::new(100);
        let key = |g: u16| GlyphKey {
            face: 0,
            size_x64: 1,
            glyph: g,
        };
        let mk = |n: usize| CachedGlyph {
            left: 0,
            top: 0,
            width: n as u32,
            height: 1,
            data: vec![0; n],
            tick: 0,
        };
        c.insert(key(1), mk(40));
        let big = c.insert(key(2), mk(500));
        assert_eq!(big.data.len(), 500);
        assert!(c.bytes() <= 100);
        assert_eq!(c.bytes(), 40, "the small glyph survives");
        assert_eq!(c.len(), 1);
        assert!(c.get(key(2)).is_none(), "oversize glyphs are not looked up");
        // The next insert drops the parked oversize glyph.
        c.insert(key(3), mk(10));
        assert!(c.oversize.is_none());
        assert_eq!(c.bytes(), 50);
    }

    #[test]
    fn huge_sizes_are_refused() {
        let mut engine = TextEngine::default();
        assert!(engine.glyph(&FontSet::empty(), 0, 100_000.0, 1).is_none());
        assert!(engine.glyph(&FontSet::empty(), 0, f32::NAN, 1).is_none());
        let Some(fonts) = fonts() else { return };
        let face = fonts
            .face(
                css_subset::FontFamily::Sans,
                css_subset::FontWeight::Normal,
                css_subset::FontStyle::Normal,
            )
            .unwrap();
        let mut out = Vec::new();
        engine.shape(&fonts, face, 16.0, "M", &mut out);
        assert!(engine
            .glyph(&fonts, face, MAX_GLYPH_PX * 2.0, out[0].id)
            .is_none());
        // Within the size cap but above the byte cap: drawn, not cached.
        let mut small = TextEngine::new(1024);
        let g = small.glyph(&fonts, face, 200.0, out[0].id).unwrap();
        assert!(g.data.len() > 1024);
        assert_eq!(small.cache().bytes(), 0);
        assert_eq!(small.cache().len(), 0);
    }

    #[test]
    fn entry_count_is_capped_and_the_map_never_grows() {
        let mut c = GlyphCache::new(usize::MAX);
        let cap0 = c.map.capacity();
        // Churn well past the entry cap: evictions leave tombstones, and a
        // resize would show as a capacity jump (hashbrown doubles).
        for i in 0..(10 * GLYPH_CACHE_ENTRIES as u16) {
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
                    width: 1,
                    height: 1,
                    data: vec![0; 1],
                    tick: 0,
                },
            );
            assert!(c.len() <= GLYPH_CACHE_ENTRIES);
        }
        assert!(c.map.capacity() <= cap0, "{} > {cap0}", c.map.capacity());
        assert_eq!(c.len(), GLYPH_CACHE_ENTRIES);
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
