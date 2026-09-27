//! `ComputedStyle` and its value types.

use std::hash::{Hash, Hasher};

use rkyv::{Archive, Deserialize, Portable, Serialize};

use crate::enums::*;

/// Side index into the four-element `[T; 4]` arrays (`margin`, `padding`,
/// `border_*`, `inset`), in CSS shorthand order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Side {
    /// Top edge.
    Top = 0,
    /// Right edge.
    Right = 1,
    /// Bottom edge.
    Bottom = 2,
    /// Left edge.
    Left = 3,
}

impl Side {
    /// All sides in shorthand order.
    pub const ALL: [Side; 4] = [Side::Top, Side::Right, Side::Bottom, Side::Left];
}

/// A CSS length after the loader resolved font-relative units.
///
/// `unit == Auto` means `auto`/`none`/`normal` and `value` is ignored.
#[derive(Clone, Copy, Debug, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
pub struct Length {
    /// Unit of `value`.
    pub unit: LengthUnit,
    /// Magnitude in `unit`s.
    pub value: f32,
}

impl Length {
    /// `auto`.
    pub const AUTO: Length = Length {
        unit: LengthUnit::Auto,
        value: 0.0,
    };
    /// `0px`.
    pub const ZERO: Length = Length::px(0.0);

    /// A pixel length.
    pub const fn px(value: f32) -> Length {
        Length {
            unit: LengthUnit::Px,
            value,
        }
    }

    /// A percentage of the containing block (`value` in 0..=100).
    pub const fn percent(value: f32) -> Length {
        Length {
            unit: LengthUnit::Percent,
            value,
        }
    }

    /// Whether this is `auto`.
    pub const fn is_auto(self) -> bool {
        matches!(self.unit, LengthUnit::Auto)
    }

    /// Resolves to CSS px against a containing-block size and viewport,
    /// or `None` for `auto`.
    pub fn resolve(self, percent_base: f32, viewport_w: f32, viewport_h: f32) -> Option<f32> {
        match self.unit {
            LengthUnit::Px => Some(self.value),
            LengthUnit::Percent => Some(self.value * percent_base / 100.0),
            LengthUnit::Vw => Some(self.value * viewport_w / 100.0),
            LengthUnit::Vh => Some(self.value * viewport_h / 100.0),
            LengthUnit::Auto => None,
        }
    }
}

impl Default for Length {
    fn default() -> Self {
        Length::AUTO
    }
}

impl PartialEq for Length {
    fn eq(&self, other: &Self) -> bool {
        self.unit == other.unit && canon_bits(self.value) == canon_bits(other.value)
    }
}
impl Eq for Length {}

impl Hash for Length {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.unit.hash(state);
        canon_bits(self.value).hash(state);
    }
}

impl ArchivedLength {
    /// Copies the archived value out as a native [`Length`].
    pub fn to_native(self) -> Length {
        Length {
            unit: self.unit,
            value: self.value.to_native(),
        }
    }
}

/// Bit pattern of an `f32` with `-0.0` folded into `0.0` and every NaN
/// folded into one value, so equal-looking styles intern to the same id.
#[inline]
fn canon_bits(v: f32) -> u32 {
    if v.is_nan() {
        0x7fc0_0000
    } else if v == 0.0 {
        0
    } else {
        v.to_bits()
    }
}

/// Straight (non-premultiplied) 8-bit sRGB colour with alpha.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    Portable,
    rkyv::bytecheck::CheckBytes,
)]
#[rkyv(as = Self)]
#[bytecheck(crate = rkyv::bytecheck)]
#[repr(C)]
pub struct Rgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha (255 = opaque).
    pub a: u8,
}

impl Rgba {
    /// Fully transparent black (`transparent`).
    pub const TRANSPARENT: Rgba = Rgba::new(0, 0, 0, 0);
    /// Opaque black.
    pub const BLACK: Rgba = Rgba::new(0, 0, 0, 255);
    /// Opaque white.
    pub const WHITE: Rgba = Rgba::new(255, 255, 255, 255);

    /// Builds a colour from components.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba { r, g, b, a }
    }

    /// Opaque colour from RGB.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
        Rgba::new(r, g, b, 255)
    }

    /// Whether painting this colour has no effect.
    pub const fn is_transparent(self) -> bool {
        self.a == 0
    }
}

/// `text-decoration-line` bits.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    Portable,
    rkyv::bytecheck::CheckBytes,
)]
#[rkyv(as = Self)]
#[bytecheck(crate = rkyv::bytecheck)]
#[repr(transparent)]
pub struct TextDecoration(u8);

bitflags::bitflags! {
    impl TextDecoration: u8 {
        /// `underline`.
        const UNDERLINE = 1 << 0;
        /// `overline`.
        const OVERLINE = 1 << 1;
        /// `line-through`.
        const LINE_THROUGH = 1 << 2;
    }
}

/// One track of a `grid-template-columns`/`rows` list. Track lists live in
/// the page file's `tracks` table; styles reference them by [`TrackListRef`].
#[derive(Clone, Copy, Debug, Default, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
pub struct GridTrack {
    /// Sizing function.
    pub size: TrackSize,
    /// Magnitude for `Px`, `Percent`, `Fr`; ignored otherwise.
    pub value: f32,
}

impl PartialEq for GridTrack {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && canon_bits(self.value) == canon_bits(other.value)
    }
}
impl Eq for GridTrack {}
impl Hash for GridTrack {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.size.hash(state);
        canon_bits(self.value).hash(state);
    }
}

impl ArchivedGridTrack {
    /// Copies the archived value out as a native [`GridTrack`].
    pub fn to_native(self) -> GridTrack {
        GridTrack {
            size: self.size,
            value: self.value.to_native(),
        }
    }
}

/// A `(offset, len)` slice of the page's `tracks` table; `len == 0` means
/// `none` (implicit tracks only).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
pub struct TrackListRef {
    /// First track index.
    pub off: u16,
    /// Number of tracks.
    pub len: u16,
}

impl TrackListRef {
    /// `none`.
    pub const NONE: TrackListRef = TrackListRef { off: 0, len: 0 };

    /// End index (exclusive), or `None` on overflow.
    pub fn end(self) -> Option<usize> {
        (self.off as usize).checked_add(self.len as usize)
    }
}

impl ArchivedTrackListRef {
    /// Copies the archived value out as a native [`TrackListRef`].
    pub fn to_native(self) -> TrackListRef {
        TrackListRef {
            off: self.off.to_native(),
            len: self.len.to_native(),
        }
    }
}

/// One side of a `grid-column`/`grid-row` placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
pub struct GridLine {
    /// `auto`, an explicit line, or a span.
    pub kind: GridLineKind,
    /// Line number (may be negative) or span count.
    pub value: i16,
}

impl GridLine {
    /// `auto`.
    pub const AUTO: GridLine = GridLine {
        kind: GridLineKind::Auto,
        value: 0,
    };
}

/// `grid-column` or `grid-row`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
pub struct GridPlacement {
    /// Start line.
    pub start: GridLine,
    /// End line.
    pub end: GridLine,
}

/// `z-index: auto` sentinel.
pub const Z_INDEX_AUTO: i16 = i16::MIN;

/// Hashing/equality helper so `ComputedStyle` can be an interning key even
/// though it holds `f32` fields.
trait StyleKey {
    fn key_hash<H: Hasher>(&self, state: &mut H);
    fn key_eq(&self, other: &Self) -> bool;
}

impl StyleKey for f32 {
    fn key_hash<H: Hasher>(&self, state: &mut H) {
        canon_bits(*self).hash(state);
    }
    fn key_eq(&self, other: &Self) -> bool {
        canon_bits(*self) == canon_bits(*other)
    }
}

impl<T: StyleKey> StyleKey for [T; 4] {
    fn key_hash<H: Hasher>(&self, state: &mut H) {
        for v in self {
            v.key_hash(state);
        }
    }
    fn key_eq(&self, other: &Self) -> bool {
        self.iter().zip(other).all(|(a, b)| a.key_eq(b))
    }
}

macro_rules! hash_eq_key {
    ($($t:ty),* $(,)?) => {
        $(
            impl StyleKey for $t {
                fn key_hash<H: Hasher>(&self, state: &mut H) { self.hash(state); }
                fn key_eq(&self, other: &Self) -> bool { self == other }
            }
        )*
    };
}
hash_eq_key!(
    i16,
    Length,
    Rgba,
    TextDecoration,
    TrackListRef,
    GridPlacement,
    Display,
    Position,
    Float,
    Clear,
    BorderStyle,
    BoxSizing,
    Overflow,
    Visibility,
    FontFamily,
    FontWeight,
    FontStyle,
    TextAlign,
    TextTransform,
    WhiteSpace,
    WordBreak,
    VerticalAlign,
    ListStyleType,
    FlexDirection,
    FlexWrap,
    JustifyContent,
    AlignItems,
);

/// Declares `ComputedStyle` with generated `Hash`/`Eq` that treat `f32`
/// fields by canonical bit pattern.
macro_rules! computed_style {
    ($( $(#[$m:meta])* $field:ident : $ty:ty = $init:expr ),* $(,)?) => {
        /// The full computed style of one node: a fixed-size struct of enums
        /// and lengths (plan §4/§5). Interned by the loader; nodes refer to
        /// entries by `u16` id.
        #[derive(Clone, Copy, Debug, Archive, Serialize, Deserialize)]
        #[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
        pub struct ComputedStyle {
            $( $(#[$m])* pub $field: $ty, )*
        }

        impl ComputedStyle {
            /// CSS initial values for every property (the root's parent).
            pub const INITIAL: ComputedStyle = ComputedStyle { $( $field: $init, )* };
        }

        impl PartialEq for ComputedStyle {
            fn eq(&self, other: &Self) -> bool {
                true $( && StyleKey::key_eq(&self.$field, &other.$field) )*
            }
        }
        impl Eq for ComputedStyle {}
        impl Hash for ComputedStyle {
            fn hash<H: Hasher>(&self, state: &mut H) {
                $( StyleKey::key_hash(&self.$field, state); )*
            }
        }
    };
}

computed_style! {
    /// `display`.
    display: Display = Display::Inline,
    /// `position`.
    position: Position = Position::Static,
    /// `top`/`right`/`bottom`/`left`, indexed by [`Side`].
    inset: [Length; 4] = [Length::AUTO; 4],
    /// `float`.
    float: Float = Float::None,
    /// `clear`.
    clear: Clear = Clear::None,
    /// `width`.
    width: Length = Length::AUTO,
    /// `height`.
    height: Length = Length::AUTO,
    /// `min-width` (`auto` = 0 for block boxes).
    min_width: Length = Length::AUTO,
    /// `min-height`.
    min_height: Length = Length::AUTO,
    /// `max-width` (`Auto` encodes `none`).
    max_width: Length = Length::AUTO,
    /// `max-height` (`Auto` encodes `none`).
    max_height: Length = Length::AUTO,
    /// `margin-*`, indexed by [`Side`]; `auto` centres block boxes.
    margin: [Length; 4] = [Length::ZERO; 4],
    /// `padding-*`, indexed by [`Side`].
    padding: [Length; 4] = [Length::ZERO; 4],
    /// `border-*-width` in CSS px, indexed by [`Side`].
    border_width: [f32; 4] = [0.0; 4],
    /// `border-*-style`, indexed by [`Side`].
    border_style: [BorderStyle; 4] = [BorderStyle::None; 4],
    /// `border-*-color`, indexed by [`Side`] (`currentcolor` resolved).
    border_color: [Rgba; 4] = [Rgba::BLACK; 4],
    /// `border-radius` per corner: top-left, top-right, bottom-right, bottom-left.
    border_radius: [Length; 4] = [Length::ZERO; 4],
    /// `box-sizing`.
    box_sizing: BoxSizing = BoxSizing::ContentBox,
    /// `overflow-x`.
    overflow_x: Overflow = Overflow::Visible,
    /// `overflow-y`.
    overflow_y: Overflow = Overflow::Visible,
    /// `color`.
    color: Rgba = Rgba::BLACK,
    /// `background-color`.
    background_color: Rgba = Rgba::TRANSPARENT,
    /// `opacity` in 0..=1.
    opacity: f32 = 1.0,
    /// `visibility`.
    visibility: Visibility = Visibility::Visible,
    /// `font-family` mapped to a bundled generic face.
    font_family: FontFamily = FontFamily::Sans,
    /// `font-size` in CSS px (already resolved from em/rem/pt/keywords).
    font_size: f32 = 16.0,
    /// `font-weight` bucket.
    font_weight: FontWeight = FontWeight::Normal,
    /// `font-style`.
    font_style: FontStyle = FontStyle::Normal,
    /// `line-height` resolved to px; `Auto` encodes `normal`.
    line_height: Length = Length::AUTO,
    /// `text-align`.
    text_align: TextAlign = TextAlign::Left,
    /// `text-decoration-line`.
    text_decoration: TextDecoration = TextDecoration::empty(),
    /// `text-transform`.
    text_transform: TextTransform = TextTransform::None,
    /// `text-indent`.
    text_indent: Length = Length::ZERO,
    /// `white-space`.
    white_space: WhiteSpace = WhiteSpace::Normal,
    /// `word-break`.
    word_break: WordBreak = WordBreak::Normal,
    /// `vertical-align`.
    vertical_align: VerticalAlign = VerticalAlign::Baseline,
    /// `list-style-type`.
    list_style_type: ListStyleType = ListStyleType::Disc,
    /// `z-index`; [`Z_INDEX_AUTO`] encodes `auto`.
    z_index: i16 = Z_INDEX_AUTO,
    /// `row-gap`.
    row_gap: Length = Length::ZERO,
    /// `column-gap`.
    column_gap: Length = Length::ZERO,
    /// `flex-direction`.
    flex_direction: FlexDirection = FlexDirection::Row,
    /// `flex-wrap`.
    flex_wrap: FlexWrap = FlexWrap::NoWrap,
    /// `justify-content`.
    justify_content: JustifyContent = JustifyContent::FlexStart,
    /// `align-items`.
    align_items: AlignItems = AlignItems::Stretch,
    /// `align-self`.
    align_self: AlignItems = AlignItems::Auto,
    /// `align-content`.
    align_content: AlignItems = AlignItems::Stretch,
    /// `flex-grow`.
    flex_grow: f32 = 0.0,
    /// `flex-shrink`.
    flex_shrink: f32 = 1.0,
    /// `flex-basis` (`Auto` = content size).
    flex_basis: Length = Length::AUTO,
    /// `order`.
    order: i16 = 0,
    /// `grid-template-columns` as a slice of the page's track table.
    grid_template_columns: TrackListRef = TrackListRef::NONE,
    /// `grid-template-rows` as a slice of the page's track table.
    grid_template_rows: TrackListRef = TrackListRef::NONE,
    /// `grid-column`.
    grid_column: GridPlacement = GridPlacement { start: GridLine::AUTO, end: GridLine::AUTO },
    /// `grid-row`.
    grid_row: GridPlacement = GridPlacement { start: GridLine::AUTO, end: GridLine::AUTO },
}

impl Default for ComputedStyle {
    fn default() -> Self {
        ComputedStyle::INITIAL
    }
}

impl ComputedStyle {
    /// The properties that inherit by default, copied from `parent` onto a
    /// fresh initial style. This is the starting point of a cascade for a
    /// child node.
    pub fn inherit_from(parent: &ComputedStyle) -> ComputedStyle {
        ComputedStyle {
            color: parent.color,
            visibility: parent.visibility,
            font_family: parent.font_family,
            font_size: parent.font_size,
            font_weight: parent.font_weight,
            font_style: parent.font_style,
            line_height: parent.line_height,
            text_align: parent.text_align,
            text_transform: parent.text_transform,
            text_indent: parent.text_indent,
            white_space: parent.white_space,
            word_break: parent.word_break,
            list_style_type: parent.list_style_type,
            ..ComputedStyle::INITIAL
        }
    }

    /// Rounds every length and border width to a multiple of `step` px.
    /// Used by the loader when the style table would overflow (plan §6.5).
    pub fn quantize_lengths(&mut self, step: f32) {
        fn q(l: &mut Length, step: f32) {
            if l.unit != LengthUnit::Auto {
                l.value = (l.value / step).round() * step;
            }
        }
        for l in self
            .inset
            .iter_mut()
            .chain(self.margin.iter_mut())
            .chain(self.padding.iter_mut())
            .chain(self.border_radius.iter_mut())
        {
            q(l, step);
        }
        for l in [
            &mut self.width,
            &mut self.height,
            &mut self.min_width,
            &mut self.min_height,
            &mut self.max_width,
            &mut self.max_height,
            &mut self.line_height,
            &mut self.text_indent,
            &mut self.row_gap,
            &mut self.column_gap,
            &mut self.flex_basis,
        ] {
            q(l, step);
        }
        for w in &mut self.border_width {
            *w = (*w / step).round() * step;
        }
        self.font_size = (self.font_size / step).round() * step;
    }
}

impl ArchivedComputedStyle {
    /// Copies the archived style out as a native [`ComputedStyle`].
    pub fn to_native(&self) -> ComputedStyle {
        rkyv::deserialize::<ComputedStyle, rkyv::rancor::Failure>(self)
            .expect("ComputedStyle has no fallible fields")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;

    fn h<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    #[test]
    fn negative_zero_and_nan_are_canonical() {
        let a = Length::px(0.0);
        let b = Length::px(-0.0);
        assert_eq!(a, b);
        assert_eq!(h(&a), h(&b));
        let mut s1 = ComputedStyle::INITIAL;
        let mut s2 = ComputedStyle::INITIAL;
        s1.opacity = f32::NAN;
        s2.opacity = -f32::NAN;
        assert_eq!(s1, s2);
        assert_eq!(h(&s1), h(&s2));
    }

    #[test]
    fn distinct_styles_differ() {
        let mut s = ComputedStyle::INITIAL;
        s.font_size = 17.0;
        assert_ne!(s, ComputedStyle::INITIAL);
        let mut t = ComputedStyle::INITIAL;
        t.text_decoration = TextDecoration::UNDERLINE;
        assert_ne!(t, ComputedStyle::INITIAL);
    }

    #[test]
    fn archive_round_trip() {
        let mut s = ComputedStyle::INITIAL;
        s.display = Display::Block;
        s.margin[Side::Left as usize] = Length::percent(12.5);
        s.text_decoration = TextDecoration::UNDERLINE | TextDecoration::LINE_THROUGH;
        s.grid_template_columns = TrackListRef { off: 3, len: 2 };
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&s).unwrap();
        let a = rkyv::access::<ArchivedComputedStyle, rkyv::rancor::Error>(&bytes).unwrap();
        assert!(*a == s);
        assert_eq!(a.to_native(), s);
        assert_eq!(a.margin[3].to_native(), Length::percent(12.5));
    }

    #[test]
    fn archived_style_is_compact() {
        // The plan estimates ~112 bytes with 5-byte lengths; archived
        // lengths are 8 bytes (f32 alignment), which lands at 344.
        // The exact figure matters less than catching an accidental
        // blow-up (e.g. a Vec sneaking in), and the table is file-backed.
        let size = size_of::<ArchivedComputedStyle>();
        assert!(size <= 384, "ArchivedComputedStyle is {size} bytes");
    }

    #[test]
    fn quantize() {
        let mut s = ComputedStyle::INITIAL;
        s.width = Length::px(10.3);
        s.border_width[0] = 0.7;
        s.quantize_lengths(0.5);
        assert_eq!(s.width, Length::px(10.5));
        assert_eq!(s.border_width[0], 0.5);
        assert!(s.height.is_auto());
    }

    #[test]
    fn resolve_lengths() {
        assert_eq!(Length::percent(50.0).resolve(200.0, 0.0, 0.0), Some(100.0));
        assert_eq!(Length::AUTO.resolve(200.0, 0.0, 0.0), None);
        let vw = Length {
            unit: LengthUnit::Vw,
            value: 10.0,
        };
        assert_eq!(vw.resolve(0.0, 1280.0, 800.0), Some(128.0));
    }
}
