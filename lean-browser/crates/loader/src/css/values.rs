//! Converts parsed lightningcss property values into [`ComputedStyle`]
//! fields (plan §5). `em`/`rem`/`pt`/`ch` resolve to px here; `%`, `vw`
//! and `vh` stay symbolic (plan §2.3).

use std::collections::HashMap;

use css_subset::{
    AlignItems, BorderStyle, BoxSizing, Clear, ComputedStyle, Display, FlexDirection, FlexWrap,
    Float, FontFamily, FontStyle, FontWeight, GridLine, GridLineKind, GridPlacement, GridTrack,
    JustifyContent, Length, LengthUnit, ListStyleType, Overflow, Position, Rgba, Side, TextAlign,
    TextDecoration, TextTransform, TrackListRef, TrackSize, VerticalAlign, Visibility, WhiteSpace,
    WordBreak, Z_INDEX_AUTO,
};
use lightningcss::properties::align::{
    AlignContent, AlignItems as LAlignItems, AlignSelf, ContentDistribution, ContentPosition,
    GapValue, JustifyContent as LJustifyContent, SelfPosition,
};
use lightningcss::properties::border::{BorderSideWidth, LineStyle};
use lightningcss::properties::display::{
    Display as LDisplay, DisplayInside, DisplayKeyword, DisplayOutside, Visibility as LVisibility,
};
use lightningcss::properties::font::{
    AbsoluteFontSize, AbsoluteFontWeight, FontFamily as LFontFamily, FontSize, FontStyle as LFontStyle,
    FontWeight as LFontWeight, GenericFontFamily, LineHeight, VerticalAlign as LVerticalAlign,
    VerticalAlignKeyword,
};
use lightningcss::properties::grid::{
    GridLine as LGridLine, RepeatCount, TrackBreadth, TrackListItem, TrackSize as LTrackSize,
    TrackSizing,
};
use lightningcss::properties::list::{CounterStyle, ListStyleType as LListStyleType};
use lightningcss::properties::overflow::OverflowKeyword;
use lightningcss::properties::position::{Position as LPosition, ZIndex};
use lightningcss::properties::size::{BoxSizing as LBoxSizing, MaxSize, Size};
use lightningcss::properties::text::{
    TextAlign as LTextAlign, TextDecorationLine, TextTransformCase, WhiteSpace as LWhiteSpace,
    WordBreak as LWordBreak,
};
use lightningcss::properties::{custom::TokenOrValue, Property};
use lightningcss::values::calc::Calc;
use lightningcss::values::color::CssColor;
use lightningcss::values::length::{LengthPercentage, LengthPercentageOrAuto, LengthValue};
use lightningcss::traits::ToCss;
use lightningcss::values::percentage::DimensionPercentage;

/// Font-relative context for resolving lengths.
#[derive(Clone, Copy, Debug)]
pub struct LenCtx {
    /// The element's own font size (or the parent's while computing
    /// `font-size`).
    pub em: f32,
    /// The root element's font size.
    pub rem: f32,
    /// Viewport width in px, used only inside `calc()`.
    pub vw: f32,
    /// Viewport height in px, used only inside `calc()`.
    pub vh: f32,
}

/// Grid track lists, deduplicated, destined for `Page.tracks`.
#[derive(Debug, Default)]
pub struct TrackTable {
    /// Flat track storage.
    pub tracks: Vec<GridTrack>,
    index: HashMap<Vec<GridTrack>, TrackListRef>,
}

impl TrackTable {
    fn intern(&mut self, list: Vec<GridTrack>) -> TrackListRef {
        if list.is_empty() {
            return TrackListRef::NONE;
        }
        if let Some(r) = self.index.get(&list) {
            return *r;
        }
        let off = self.tracks.len();
        if off + list.len() > u16::MAX as usize {
            return TrackListRef::NONE;
        }
        let r = TrackListRef {
            off: off as u16,
            len: list.len() as u16,
        };
        self.tracks.extend_from_slice(&list);
        self.index.insert(list, r);
        r
    }
}

/// Per-element state that lives beside the [`ComputedStyle`] while
/// declarations are applied.
pub struct Apply<'a> {
    /// The style being built.
    pub st: &'a mut ComputedStyle,
    /// The parent's computed style (for `inherit`).
    pub parent: &'a ComputedStyle,
    /// Length context.
    pub len: LenCtx,
    /// `line-height: <number>` factor, kept so children recompute it
    /// against their own font size.
    pub line_height_factor: Option<f32>,
    /// Sides whose border colour is still `currentcolor`.
    pub border_current: [bool; 4],
    /// Grid track table.
    pub tracks: &'a mut TrackTable,
}

impl Apply<'_> {
    /// Resolves the deferred pieces after every declaration was applied:
    /// `currentcolor` borders, zero width for `border-style: none`.
    pub fn finish(&mut self) {
        for i in 0..4 {
            if self.border_current[i] {
                self.st.border_color[i] = self.st.color;
            }
            if self.st.border_style[i] == BorderStyle::None {
                self.st.border_width[i] = 0.0;
            }
        }
        self.st.opacity = self.st.opacity.clamp(0.0, 1.0);
    }
}

// ---------------------------------------------------------------------------
// Lengths

/// A single `<length>` to our [`Length`].
pub fn length_value(v: &LengthValue, ctx: &LenCtx) -> Length {
    use LengthValue::*;
    let px = |x: f32| Length::px(x);
    match v {
        Px(x) => px(*x),
        Em(x) => px(x * ctx.em),
        Rem(x) => px(x * ctx.rem),
        Ex(x) | Ch(x) => px(x * ctx.em * 0.5),
        Rex(x) | Rch(x) => px(x * ctx.rem * 0.5),
        Cap(x) | Ic(x) => px(x * ctx.em),
        Rcap(x) | Ric(x) => px(x * ctx.rem),
        Lh(x) => px(x * ctx.em * 1.2),
        Rlh(x) => px(x * ctx.rem * 1.2),
        Vw(x) | Lvw(x) | Svw(x) | Dvw(x) | Cqw(x) | Vi(x) | Svi(x) | Lvi(x) | Dvi(x) | Cqi(x)
        | Vmax(x) | Lvmax(x) | Svmax(x) | Dvmax(x) | Cqmax(x) => Length {
            unit: LengthUnit::Vw,
            value: *x,
        },
        Vh(x) | Lvh(x) | Svh(x) | Dvh(x) | Cqh(x) | Vb(x) | Svb(x) | Lvb(x) | Dvb(x) | Cqb(x)
        | Vmin(x) | Lvmin(x) | Svmin(x) | Dvmin(x) | Cqmin(x) => Length {
            unit: LengthUnit::Vh,
            value: *x,
        },
        other => px(other.to_px().unwrap_or(0.0)),
    }
}

/// Px value of a length for use inside `calc()` (viewport units resolved
/// against the bucket).
fn calc_px(l: Length, ctx: &LenCtx) -> f32 {
    l.resolve(0.0, ctx.vw, ctx.vh).unwrap_or(0.0)
}

/// Evaluates `calc()` over length-percentages: returns `(px, percent)`
/// contributions, or `None` for unsupported functions (`min()`, `max()`,
/// `clamp()`, …).
fn eval_calc(c: &Calc<LengthPercentage>, ctx: &LenCtx) -> Option<(f32, f32)> {
    match c {
        Calc::Value(v) => match length_percentage(v, ctx) {
            Length {
                unit: LengthUnit::Percent,
                value,
            } => Some((0.0, value)),
            l => Some((calc_px(l, ctx), 0.0)),
        },
        Calc::Number(n) => Some((*n, 0.0)),
        Calc::Sum(a, b) => {
            let (ap, aq) = eval_calc(a, ctx)?;
            let (bp, bq) = eval_calc(b, ctx)?;
            Some((ap + bp, aq + bq))
        }
        Calc::Product(k, v) => {
            let (p, q) = eval_calc(v, ctx)?;
            Some((p * k, q * k))
        }
        Calc::Function(_) => None,
    }
}

/// `<length-percentage>` to [`Length`]. A `calc()` mixing px and `%` keeps
/// the percentage part (documented approximation: `calc(100% - 20px)` is
/// stored as `100%`).
pub fn length_percentage(v: &LengthPercentage, ctx: &LenCtx) -> Length {
    match v {
        DimensionPercentage::Dimension(d) => length_value(d, ctx),
        DimensionPercentage::Percentage(p) => Length::percent(p.0 * 100.0),
        DimensionPercentage::Calc(c) => match eval_calc(c, ctx) {
            Some((_, pct)) if pct != 0.0 => Length::percent(pct),
            Some((px, _)) => Length::px(px),
            None => Length::ZERO,
        },
    }
}

/// `<length-percentage> | auto`.
pub fn lp_or_auto(v: &LengthPercentageOrAuto, ctx: &LenCtx) -> Length {
    match v {
        LengthPercentageOrAuto::Auto => Length::AUTO,
        LengthPercentageOrAuto::LengthPercentage(lp) => length_percentage(lp, ctx),
    }
}

/// `width`/`height`/`min-*`.
pub fn size(v: &Size, ctx: &LenCtx) -> Length {
    match v {
        Size::LengthPercentage(lp) => length_percentage(lp, ctx),
        Size::FitContentFunction(lp) => length_percentage(lp, ctx),
        // auto, min-content, max-content, fit-content, stretch
        _ => Length::AUTO,
    }
}

/// `max-width`/`max-height` (`none` is `Auto`).
pub fn max_size(v: &MaxSize, ctx: &LenCtx) -> Length {
    match v {
        MaxSize::LengthPercentage(lp) => length_percentage(lp, ctx),
        MaxSize::FitContentFunction(lp) => length_percentage(lp, ctx),
        _ => Length::AUTO,
    }
}

fn border_side_width(w: &BorderSideWidth, ctx: &LenCtx) -> f32 {
    match w {
        BorderSideWidth::Thin => 1.0,
        BorderSideWidth::Medium => 3.0,
        BorderSideWidth::Thick => 5.0,
        BorderSideWidth::Length(l) => match l {
            lightningcss::values::length::Length::Value(v) => {
                calc_px(length_value(v, ctx), ctx)
            }
            lightningcss::values::length::Length::Calc(_) => 3.0,
        },
    }
}

fn line_style(s: &LineStyle) -> BorderStyle {
    match s {
        LineStyle::None | LineStyle::Hidden => BorderStyle::None,
        LineStyle::Dotted => BorderStyle::Dotted,
        LineStyle::Dashed => BorderStyle::Dashed,
        _ => BorderStyle::Solid,
    }
}

// ---------------------------------------------------------------------------
// Colours and fonts

/// Converts a colour; `currentcolor` yields `None`.
pub fn color(c: &CssColor) -> Option<Rgba> {
    match c {
        CssColor::CurrentColor => None,
        CssColor::LightDark(light, _) => color(light),
        other => match other.to_rgb() {
            Ok(CssColor::RGBA(rgba)) => Some(Rgba::new(rgba.red, rgba.green, rgba.blue, rgba.alpha)),
            _ => None,
        },
    }
}

/// Computes `font-size` against the parent's font size.
pub fn font_size(fs: &FontSize, parent_px: f32, ctx: &LenCtx) -> f32 {
    let ctx = LenCtx {
        em: parent_px,
        ..*ctx
    };
    let px = match fs {
        FontSize::Length(lp) => match length_percentage(lp, &ctx) {
            Length {
                unit: LengthUnit::Percent,
                value,
            } => parent_px * value / 100.0,
            l => calc_px(l, &ctx),
        },
        FontSize::Absolute(a) => match a {
            AbsoluteFontSize::XXSmall => 9.0,
            AbsoluteFontSize::XSmall => 10.0,
            AbsoluteFontSize::Small => 13.0,
            AbsoluteFontSize::Medium => 16.0,
            AbsoluteFontSize::Large => 18.0,
            AbsoluteFontSize::XLarge => 24.0,
            AbsoluteFontSize::XXLarge => 32.0,
            AbsoluteFontSize::XXXLarge => 48.0,
        },
        FontSize::Relative(r) => match r {
            lightningcss::properties::font::RelativeFontSize::Smaller => parent_px / 1.2,
            lightningcss::properties::font::RelativeFontSize::Larger => parent_px * 1.2,
        },
    };
    if px.is_finite() && px >= 0.0 {
        px
    } else {
        parent_px
    }
}

/// Maps a `font-family` list to a bundled generic face. The first family
/// with a recognisable name decides; unknown names are skipped.
pub fn font_family(list: &[LFontFamily<'_>]) -> FontFamily {
    for f in list {
        match f {
            LFontFamily::Generic(g) => {
                return match g {
                    GenericFontFamily::Serif | GenericFontFamily::UISerif => FontFamily::Serif,
                    GenericFontFamily::Monospace | GenericFontFamily::UIMonospace => FontFamily::Mono,
                    _ => FontFamily::Sans,
                }
            }
            LFontFamily::FamilyName(name) => {
                let text = name.to_css_string(Default::default()).unwrap_or_default();
                if let Some(f) = family_by_name(text.trim_matches(|c| c == '"' || c == '\'')) {
                    return f;
                }
            }
        }
    }
    FontFamily::Sans
}

fn family_by_name(name: &str) -> Option<FontFamily> {
    let n = name.to_ascii_lowercase();
    const MONO: &[&str] = &[
        "mono", "courier", "consolas", "menlo", "monaco", "fira code", "source code", "jetbrains",
        "cascadia", "inconsolata", "ubuntu mono", "sf mono", "hack", "iosevka",
    ];
    const SERIF: &[&str] = &[
        "serif", "georgia", "times", "garamond", "palatino", "book", "cambria", "merriweather",
        "lora", "playfair", "baskerville", "charter", "literata", "pt serif", "noto serif",
        "source serif", "libre",
    ];
    const SANS: &[&str] = &[
        "sans", "arial", "helvetica", "verdana", "tahoma", "segoe", "roboto", "inter", "open",
        "lato", "system-ui", "-apple-system", "blinkmac", "ubuntu", "cantarell", "noto", "dejavu",
        "liberation", "source sans", "ibm plex", "montserrat", "poppins", "nunito", "raleway",
        "work sans", "fira sans", "pt sans", "trebuchet", "calibri", "geneva", "avenir",
    ];
    if MONO.iter().any(|m| n.contains(m)) {
        return Some(FontFamily::Mono);
    }
    if SANS.iter().any(|m| n.contains(m)) {
        return Some(FontFamily::Sans);
    }
    if SERIF.iter().any(|m| n.contains(m)) {
        return Some(FontFamily::Serif);
    }
    None
}

fn font_weight(w: &LFontWeight, parent: FontWeight) -> FontWeight {
    match w {
        LFontWeight::Absolute(a) => match a {
            AbsoluteFontWeight::Weight(n) => {
                if *n >= 600.0 {
                    FontWeight::Bold
                } else {
                    FontWeight::Normal
                }
            }
            AbsoluteFontWeight::Normal => FontWeight::Normal,
            AbsoluteFontWeight::Bold => FontWeight::Bold,
        },
        LFontWeight::Bolder => FontWeight::Bold,
        LFontWeight::Lighter => {
            let _ = parent;
            FontWeight::Normal
        }
    }
}

fn display(d: &LDisplay) -> Display {
    match d {
        LDisplay::Keyword(k) => match k {
            DisplayKeyword::None => Display::None,
            DisplayKeyword::TableRowGroup => Display::TableRowGroup,
            DisplayKeyword::TableHeaderGroup => Display::TableHeaderGroup,
            DisplayKeyword::TableFooterGroup => Display::TableFooterGroup,
            DisplayKeyword::TableRow => Display::TableRow,
            DisplayKeyword::TableCell => Display::TableCell,
            DisplayKeyword::TableCaption => Display::TableCaption,
            DisplayKeyword::TableColumnGroup | DisplayKeyword::TableColumn => Display::None,
            // `contents` and ruby: rendered as inline (fallback).
            _ => Display::Inline,
        },
        LDisplay::Pair(p) => {
            let block = !matches!(p.outside, DisplayOutside::Inline);
            if p.is_list_item {
                return if block { Display::ListItem } else { Display::Inline };
            }
            match (&p.inside, block) {
                (DisplayInside::Flow, true) => Display::Block,
                (DisplayInside::Flow, false) => Display::Inline,
                (DisplayInside::FlowRoot, true) => Display::Block,
                (DisplayInside::FlowRoot, false) => Display::InlineBlock,
                (DisplayInside::Table, true) => Display::Table,
                (DisplayInside::Table, false) => Display::InlineBlock,
                (DisplayInside::Flex(_) | DisplayInside::Box(_), true) => Display::Flex,
                (DisplayInside::Flex(_) | DisplayInside::Box(_), false) => Display::InlineFlex,
                (DisplayInside::Grid, true) => Display::Grid,
                (DisplayInside::Grid, false) => Display::InlineGrid,
                (DisplayInside::Ruby, _) => Display::Inline,
            }
        }
    }
}

fn track_breadth(b: &TrackBreadth, ctx: &LenCtx) -> GridTrack {
    match b {
        TrackBreadth::Length(lp) => match length_percentage(lp, ctx) {
            Length {
                unit: LengthUnit::Percent,
                value,
            } => GridTrack {
                size: TrackSize::Percent,
                value,
            },
            l => GridTrack {
                size: TrackSize::Px,
                value: calc_px(l, ctx),
            },
        },
        TrackBreadth::Flex(f) => GridTrack {
            size: TrackSize::Fr,
            value: *f,
        },
        TrackBreadth::MinContent => GridTrack {
            size: TrackSize::MinContent,
            value: 0.0,
        },
        TrackBreadth::MaxContent => GridTrack {
            size: TrackSize::MaxContent,
            value: 0.0,
        },
        TrackBreadth::Auto => GridTrack::default(),
    }
}

fn track_size(t: &LTrackSize, ctx: &LenCtx) -> GridTrack {
    match t {
        LTrackSize::TrackBreadth(b) => track_breadth(b, ctx),
        LTrackSize::MinMax { max, .. } => track_breadth(max, ctx),
        LTrackSize::FitContent(_) => GridTrack::default(),
    }
}

fn track_list(sizing: &TrackSizing<'_>, ctx: &LenCtx, tracks: &mut TrackTable) -> TrackListRef {
    const MAX_TRACKS: usize = 64;
    let TrackSizing::TrackList(list) = sizing else {
        return TrackListRef::NONE;
    };
    let mut out = Vec::new();
    for item in &list.items {
        match item {
            TrackListItem::TrackSize(t) => out.push(track_size(t, ctx)),
            TrackListItem::TrackRepeat(r) => {
                let n = match r.count {
                    RepeatCount::Number(n) => n.max(1) as usize,
                    RepeatCount::AutoFill | RepeatCount::AutoFit => 1,
                };
                for _ in 0..n {
                    for t in &r.track_sizes {
                        out.push(track_size(t, ctx));
                    }
                    if out.len() >= MAX_TRACKS {
                        break;
                    }
                }
            }
        }
        if out.len() >= MAX_TRACKS {
            out.truncate(MAX_TRACKS);
            break;
        }
    }
    tracks.intern(out)
}

fn grid_line(l: &LGridLine<'_>) -> GridLine {
    match l {
        LGridLine::Line { index, .. } => GridLine {
            kind: GridLineKind::Line,
            value: (*index).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        },
        LGridLine::Span { index, .. } => GridLine {
            kind: GridLineKind::Span,
            value: (*index).clamp(1, i16::MAX as i32) as i16,
        },
        _ => GridLine::AUTO,
    }
}

fn justify(j: &LJustifyContent) -> JustifyContent {
    match j {
        LJustifyContent::Normal => JustifyContent::FlexStart,
        LJustifyContent::ContentDistribution(d) => match d {
            ContentDistribution::SpaceBetween => JustifyContent::SpaceBetween,
            ContentDistribution::SpaceAround => JustifyContent::SpaceAround,
            ContentDistribution::SpaceEvenly => JustifyContent::SpaceEvenly,
            ContentDistribution::Stretch => JustifyContent::Stretch,
        },
        LJustifyContent::ContentPosition { value, .. } => content_position(value),
        LJustifyContent::Left { .. } => JustifyContent::FlexStart,
        LJustifyContent::Right { .. } => JustifyContent::FlexEnd,
    }
}

fn content_position(p: &ContentPosition) -> JustifyContent {
    match p {
        ContentPosition::Center => JustifyContent::Center,
        ContentPosition::Start | ContentPosition::FlexStart => JustifyContent::FlexStart,
        ContentPosition::End | ContentPosition::FlexEnd => JustifyContent::FlexEnd,
    }
}

fn self_position(p: &SelfPosition) -> AlignItems {
    match p {
        SelfPosition::Center => AlignItems::Center,
        SelfPosition::Start | SelfPosition::SelfStart | SelfPosition::FlexStart => AlignItems::FlexStart,
        SelfPosition::End | SelfPosition::SelfEnd | SelfPosition::FlexEnd => AlignItems::FlexEnd,
    }
}

fn align_items(a: &LAlignItems) -> AlignItems {
    match a {
        LAlignItems::Normal | LAlignItems::Stretch => AlignItems::Stretch,
        LAlignItems::BaselinePosition(_) => AlignItems::Baseline,
        LAlignItems::SelfPosition { value, .. } => self_position(value),
    }
}

fn align_self(a: &AlignSelf) -> AlignItems {
    match a {
        AlignSelf::Auto => AlignItems::Auto,
        AlignSelf::Normal | AlignSelf::Stretch => AlignItems::Stretch,
        AlignSelf::BaselinePosition(_) => AlignItems::Baseline,
        AlignSelf::SelfPosition { value, .. } => self_position(value),
    }
}

fn align_content(a: &AlignContent) -> AlignItems {
    match a {
        AlignContent::Normal => AlignItems::Stretch,
        AlignContent::BaselinePosition(_) => AlignItems::Baseline,
        AlignContent::ContentDistribution(d) => match d {
            ContentDistribution::SpaceBetween => AlignItems::SpaceBetween,
            ContentDistribution::SpaceAround => AlignItems::SpaceAround,
            ContentDistribution::SpaceEvenly => AlignItems::SpaceEvenly,
            ContentDistribution::Stretch => AlignItems::Stretch,
        },
        AlignContent::ContentPosition { value, .. } => match value {
            ContentPosition::Center => AlignItems::Center,
            ContentPosition::Start | ContentPosition::FlexStart => AlignItems::FlexStart,
            ContentPosition::End | ContentPosition::FlexEnd => AlignItems::FlexEnd,
        },
    }
}

fn list_style_type(t: &LListStyleType<'_>) -> ListStyleType {
    match t {
        LListStyleType::None | LListStyleType::String(_) => ListStyleType::None,
        LListStyleType::CounterStyle(cs) => match cs {
            CounterStyle::Predefined(_) => ListStyleType::Decimal,
            CounterStyle::Name(n) => match &*n.0.to_ascii_lowercase() {
                "disc" => ListStyleType::Disc,
                "circle" => ListStyleType::Circle,
                "square" => ListStyleType::Square,
                "none" => ListStyleType::None,
                _ => ListStyleType::Decimal,
            },
            _ => ListStyleType::Disc,
        },
    }
}

fn decoration(line: &TextDecorationLine) -> TextDecoration {
    let mut d = TextDecoration::empty();
    if line.contains(TextDecorationLine::Underline) {
        d |= TextDecoration::UNDERLINE;
    }
    if line.contains(TextDecorationLine::Overline) {
        d |= TextDecoration::OVERLINE;
    }
    if line.contains(TextDecorationLine::LineThrough) {
        d |= TextDecoration::LINE_THROUGH;
    }
    d
}

fn line_height(lh: &LineHeight, a: &mut Apply<'_>) {
    match lh {
        LineHeight::Normal => {
            a.st.line_height = Length::AUTO;
            a.line_height_factor = None;
        }
        LineHeight::Number(n) => {
            a.line_height_factor = Some(*n);
            a.st.line_height = Length::px(n * a.st.font_size);
        }
        LineHeight::Length(lp) => {
            a.line_height_factor = None;
            a.st.line_height = match length_percentage(lp, &a.len) {
                Length {
                    unit: LengthUnit::Percent,
                    value,
                } => Length::px(value / 100.0 * a.st.font_size),
                l => Length::px(calc_px(l, &a.len)),
            };
        }
    }
}

// ---------------------------------------------------------------------------
// Application

const T: usize = Side::Top as usize;
const R: usize = Side::Right as usize;
const B: usize = Side::Bottom as usize;
const L: usize = Side::Left as usize;

/// Applies one parsed declaration. `font-size` is handled by the caller
/// beforehand (it must be known before other lengths resolve) and is
/// ignored here.
pub fn apply(prop: &Property<'_>, a: &mut Apply<'_>) {
    use Property as P;
    let ctx = a.len;
    match prop {
        P::Display(d) => a.st.display = display(d),
        P::Position(p) => {
            a.st.position = match p {
                LPosition::Static => Position::Static,
                LPosition::Relative => Position::Relative,
                LPosition::Absolute => Position::Absolute,
                LPosition::Fixed => Position::Fixed,
                LPosition::Sticky(_) => Position::Sticky,
            }
        }
        P::Top(v) => a.st.inset[T] = lp_or_auto(v, &ctx),
        P::Right(v) => a.st.inset[R] = lp_or_auto(v, &ctx),
        P::Bottom(v) => a.st.inset[B] = lp_or_auto(v, &ctx),
        P::Left(v) => a.st.inset[L] = lp_or_auto(v, &ctx),
        P::Inset(i) => {
            a.st.inset = [
                lp_or_auto(&i.top, &ctx),
                lp_or_auto(&i.right, &ctx),
                lp_or_auto(&i.bottom, &ctx),
                lp_or_auto(&i.left, &ctx),
            ]
        }
        P::Width(s) => a.st.width = size(s, &ctx),
        P::Height(s) => a.st.height = size(s, &ctx),
        P::MinWidth(s) => a.st.min_width = size(s, &ctx),
        P::MinHeight(s) => a.st.min_height = size(s, &ctx),
        P::MaxWidth(s) => a.st.max_width = max_size(s, &ctx),
        P::MaxHeight(s) => a.st.max_height = max_size(s, &ctx),
        P::MarginTop(v) => a.st.margin[T] = lp_or_auto(v, &ctx),
        P::MarginRight(v) => a.st.margin[R] = lp_or_auto(v, &ctx),
        P::MarginBottom(v) => a.st.margin[B] = lp_or_auto(v, &ctx),
        P::MarginLeft(v) => a.st.margin[L] = lp_or_auto(v, &ctx),
        P::MarginInlineStart(v) => a.st.margin[L] = lp_or_auto(v, &ctx),
        P::MarginInlineEnd(v) => a.st.margin[R] = lp_or_auto(v, &ctx),
        P::MarginBlockStart(v) => a.st.margin[T] = lp_or_auto(v, &ctx),
        P::MarginBlockEnd(v) => a.st.margin[B] = lp_or_auto(v, &ctx),
        P::Margin(m) => {
            a.st.margin = [
                lp_or_auto(&m.top, &ctx),
                lp_or_auto(&m.right, &ctx),
                lp_or_auto(&m.bottom, &ctx),
                lp_or_auto(&m.left, &ctx),
            ]
        }
        P::PaddingTop(v) => a.st.padding[T] = lp_or_auto(v, &ctx),
        P::PaddingRight(v) => a.st.padding[R] = lp_or_auto(v, &ctx),
        P::PaddingBottom(v) => a.st.padding[B] = lp_or_auto(v, &ctx),
        P::PaddingLeft(v) => a.st.padding[L] = lp_or_auto(v, &ctx),
        P::PaddingInlineStart(v) => a.st.padding[L] = lp_or_auto(v, &ctx),
        P::PaddingInlineEnd(v) => a.st.padding[R] = lp_or_auto(v, &ctx),
        P::PaddingBlockStart(v) => a.st.padding[T] = lp_or_auto(v, &ctx),
        P::PaddingBlockEnd(v) => a.st.padding[B] = lp_or_auto(v, &ctx),
        P::Padding(m) => {
            a.st.padding = [
                lp_or_auto(&m.top, &ctx),
                lp_or_auto(&m.right, &ctx),
                lp_or_auto(&m.bottom, &ctx),
                lp_or_auto(&m.left, &ctx),
            ]
        }
        // Borders
        P::BorderTopWidth(w) => a.st.border_width[T] = border_side_width(w, &ctx),
        P::BorderRightWidth(w) => a.st.border_width[R] = border_side_width(w, &ctx),
        P::BorderBottomWidth(w) => a.st.border_width[B] = border_side_width(w, &ctx),
        P::BorderLeftWidth(w) => a.st.border_width[L] = border_side_width(w, &ctx),
        P::BorderWidth(w) => {
            a.st.border_width = [
                border_side_width(&w.top, &ctx),
                border_side_width(&w.right, &ctx),
                border_side_width(&w.bottom, &ctx),
                border_side_width(&w.left, &ctx),
            ]
        }
        P::BorderTopStyle(s) => a.st.border_style[T] = line_style(s),
        P::BorderRightStyle(s) => a.st.border_style[R] = line_style(s),
        P::BorderBottomStyle(s) => a.st.border_style[B] = line_style(s),
        P::BorderLeftStyle(s) => a.st.border_style[L] = line_style(s),
        P::BorderStyle(s) => {
            a.st.border_style = [
                line_style(&s.top),
                line_style(&s.right),
                line_style(&s.bottom),
                line_style(&s.left),
            ]
        }
        P::BorderTopColor(c) => set_border_color(a, T, c),
        P::BorderRightColor(c) => set_border_color(a, R, c),
        P::BorderBottomColor(c) => set_border_color(a, B, c),
        P::BorderLeftColor(c) => set_border_color(a, L, c),
        P::BorderColor(c) => {
            set_border_color(a, T, &c.top);
            set_border_color(a, R, &c.right);
            set_border_color(a, B, &c.bottom);
            set_border_color(a, L, &c.left);
        }
        P::Border(b) => {
            for i in 0..4 {
                a.st.border_width[i] = border_side_width(&b.width, &ctx);
                a.st.border_style[i] = line_style(&b.style);
                set_border_color(a, i, &b.color);
            }
        }
        P::BorderTop(b) => set_border_side(a, T, &b.width, &b.style, &b.color),
        P::BorderRight(b) => set_border_side(a, R, &b.width, &b.style, &b.color),
        P::BorderBottom(b) => set_border_side(a, B, &b.width, &b.style, &b.color),
        P::BorderLeft(b) => set_border_side(a, L, &b.width, &b.style, &b.color),
        P::BorderTopLeftRadius(r, _) => a.st.border_radius[0] = length_percentage(&r.0, &ctx),
        P::BorderTopRightRadius(r, _) => a.st.border_radius[1] = length_percentage(&r.0, &ctx),
        P::BorderBottomRightRadius(r, _) => a.st.border_radius[2] = length_percentage(&r.0, &ctx),
        P::BorderBottomLeftRadius(r, _) => a.st.border_radius[3] = length_percentage(&r.0, &ctx),
        P::BorderRadius(r, _) => {
            a.st.border_radius = [
                length_percentage(&r.top_left.0, &ctx),
                length_percentage(&r.top_right.0, &ctx),
                length_percentage(&r.bottom_right.0, &ctx),
                length_percentage(&r.bottom_left.0, &ctx),
            ]
        }
        P::BoxSizing(b, _) => {
            a.st.box_sizing = match b {
                LBoxSizing::ContentBox => BoxSizing::ContentBox,
                LBoxSizing::BorderBox => BoxSizing::BorderBox,
            }
        }
        P::Overflow(o) => {
            a.st.overflow_x = overflow(&o.x);
            a.st.overflow_y = overflow(&o.y);
        }
        P::OverflowX(o) => a.st.overflow_x = overflow(o),
        P::OverflowY(o) => a.st.overflow_y = overflow(o),
        P::Color(c) => {
            if let Some(c) = color(c) {
                a.st.color = c;
            } else {
                a.st.color = a.parent.color;
            }
        }
        P::BackgroundColor(c) => a.st.background_color = color(c).unwrap_or(a.st.color),
        P::Background(layers) => {
            if let Some(last) = layers.last() {
                a.st.background_color = color(&last.color).unwrap_or(a.st.color);
            }
        }
        P::Opacity(o) => a.st.opacity = o.0,
        P::Visibility(v) => {
            a.st.visibility = match v {
                LVisibility::Visible => Visibility::Visible,
                _ => Visibility::Hidden,
            }
        }
        // Fonts (font-size handled by the caller)
        P::FontFamily(list) => a.st.font_family = font_family(list),
        P::FontWeight(w) => a.st.font_weight = font_weight(w, a.parent.font_weight),
        P::FontStyle(s) => {
            a.st.font_style = match s {
                LFontStyle::Normal => FontStyle::Normal,
                _ => FontStyle::Italic,
            }
        }
        P::LineHeight(lh) => line_height(lh, a),
        P::Font(f) => {
            a.st.font_family = font_family(&f.family);
            a.st.font_weight = font_weight(&f.weight, a.parent.font_weight);
            a.st.font_style = match f.style {
                LFontStyle::Normal => FontStyle::Normal,
                _ => FontStyle::Italic,
            };
            line_height(&f.line_height, a);
        }
        P::FontSize(_) => {}
        // Text
        P::TextAlign(t) => {
            a.st.text_align = match t {
                LTextAlign::Left | LTextAlign::Start | LTextAlign::MatchParent => TextAlign::Left,
                LTextAlign::Right | LTextAlign::End => TextAlign::Right,
                LTextAlign::Center => TextAlign::Center,
                LTextAlign::Justify | LTextAlign::JustifyAll => TextAlign::Justify,
            }
        }
        P::TextDecorationLine(l, _) => a.st.text_decoration = decoration(l),
        P::TextDecoration(d, _) => a.st.text_decoration = decoration(&d.line),
        P::TextTransform(t) => {
            a.st.text_transform = match t.case {
                TextTransformCase::None => TextTransform::None,
                TextTransformCase::Uppercase => TextTransform::Uppercase,
                TextTransformCase::Lowercase => TextTransform::Lowercase,
                TextTransformCase::Capitalize => TextTransform::Capitalize,
            }
        }
        P::TextIndent(t) => a.st.text_indent = length_percentage(&t.value, &ctx),
        P::WhiteSpace(w) => {
            a.st.white_space = match w {
                LWhiteSpace::Normal => WhiteSpace::Normal,
                LWhiteSpace::NoWrap => WhiteSpace::Nowrap,
                LWhiteSpace::Pre => WhiteSpace::Pre,
                LWhiteSpace::PreWrap | LWhiteSpace::BreakSpaces => WhiteSpace::PreWrap,
                LWhiteSpace::PreLine => WhiteSpace::PreLine,
            }
        }
        P::WordBreak(w) => {
            a.st.word_break = match w {
                LWordBreak::BreakAll => WordBreak::BreakAll,
                _ => WordBreak::Normal,
            }
        }
        P::VerticalAlign(v) => {
            a.st.vertical_align = match v {
                LVerticalAlign::Keyword(k) => match k {
                    VerticalAlignKeyword::Middle => VerticalAlign::Middle,
                    VerticalAlignKeyword::Top
                    | VerticalAlignKeyword::TextTop
                    | VerticalAlignKeyword::Super => VerticalAlign::Top,
                    VerticalAlignKeyword::Bottom
                    | VerticalAlignKeyword::TextBottom
                    | VerticalAlignKeyword::Sub => VerticalAlign::Bottom,
                    VerticalAlignKeyword::Baseline => VerticalAlign::Baseline,
                },
                LVerticalAlign::Length(_) => VerticalAlign::Baseline,
            }
        }
        P::ListStyleType(t) => a.st.list_style_type = list_style_type(t),
        P::ListStyle(l) => a.st.list_style_type = list_style_type(&l.list_style_type),
        P::ZIndex(z) => {
            a.st.z_index = match z {
                ZIndex::Auto => Z_INDEX_AUTO,
                ZIndex::Integer(i) => (*i).clamp(i16::MIN as i32 + 1, i16::MAX as i32) as i16,
            }
        }
        // Flex and gaps
        P::RowGap(g) => a.st.row_gap = gap(g, &ctx),
        P::ColumnGap(g) => a.st.column_gap = gap(g, &ctx),
        P::Gap(g) => {
            a.st.row_gap = gap(&g.row, &ctx);
            a.st.column_gap = gap(&g.column, &ctx);
        }
        P::FlexDirection(d, _) => a.st.flex_direction = flex_direction(d),
        P::FlexWrap(w, _) => a.st.flex_wrap = flex_wrap(w),
        P::FlexFlow(f, _) => {
            a.st.flex_direction = flex_direction(&f.direction);
            a.st.flex_wrap = flex_wrap(&f.wrap);
        }
        P::FlexGrow(g, _) => a.st.flex_grow = g.max(0.0),
        P::FlexShrink(s, _) => a.st.flex_shrink = s.max(0.0),
        P::FlexBasis(b, _) => a.st.flex_basis = lp_or_auto(b, &ctx),
        P::Flex(f, _) => {
            a.st.flex_grow = f.grow.max(0.0);
            a.st.flex_shrink = f.shrink.max(0.0);
            a.st.flex_basis = lp_or_auto(&f.basis, &ctx);
        }
        P::Order(o, _) => a.st.order = (*o).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        P::JustifyContent(j, _) => a.st.justify_content = justify(j),
        P::AlignItems(v, _) => a.st.align_items = align_items(v),
        P::AlignSelf(v, _) => a.st.align_self = align_self(v),
        P::AlignContent(v, _) => a.st.align_content = align_content(v),
        // Grid
        P::GridTemplateColumns(t) => a.st.grid_template_columns = track_list(t, &ctx, a.tracks),
        P::GridTemplateRows(t) => a.st.grid_template_rows = track_list(t, &ctx, a.tracks),
        P::GridColumnStart(l) => a.st.grid_column.start = grid_line(l),
        P::GridColumnEnd(l) => a.st.grid_column.end = grid_line(l),
        P::GridRowStart(l) => a.st.grid_row.start = grid_line(l),
        P::GridRowEnd(l) => a.st.grid_row.end = grid_line(l),
        P::GridColumn(g) => {
            a.st.grid_column = GridPlacement {
                start: grid_line(&g.start),
                end: grid_line(&g.end),
            }
        }
        P::GridRow(g) => {
            a.st.grid_row = GridPlacement {
                start: grid_line(&g.start),
                end: grid_line(&g.end),
            }
        }
        // Properties lightningcss does not model arrive as custom tokens.
        P::Custom(c) => {
            let name = c.name.as_ref().to_ascii_lowercase();
            let Some(TokenOrValue::Token(lightningcss::properties::custom::Token::Ident(kw))) =
                c.value.0.first()
            else {
                return;
            };
            let kw = kw.to_ascii_lowercase();
            match name.as_str() {
                "float" => {
                    a.st.float = match kw.as_str() {
                        "left" | "inline-start" => Float::Left,
                        "right" | "inline-end" => Float::Right,
                        _ => Float::None,
                    }
                }
                "clear" => {
                    a.st.clear = match kw.as_str() {
                        "left" | "inline-start" => Clear::Left,
                        "right" | "inline-end" => Clear::Right,
                        "both" => Clear::Both,
                        _ => Clear::None,
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn set_border_color(a: &mut Apply<'_>, side: usize, c: &CssColor) {
    match color(c) {
        Some(c) => {
            a.st.border_color[side] = c;
            a.border_current[side] = false;
        }
        None => a.border_current[side] = true,
    }
}

fn set_border_side(a: &mut Apply<'_>, side: usize, w: &BorderSideWidth, s: &LineStyle, c: &CssColor) {
    a.st.border_width[side] = border_side_width(w, &a.len);
    a.st.border_style[side] = line_style(s);
    set_border_color(a, side, c);
}

fn overflow(o: &OverflowKeyword) -> Overflow {
    match o {
        OverflowKeyword::Visible => Overflow::Visible,
        _ => Overflow::Hidden,
    }
}

fn gap(g: &GapValue, ctx: &LenCtx) -> Length {
    match g {
        GapValue::Normal => Length::ZERO,
        GapValue::LengthPercentage(lp) => length_percentage(lp, ctx),
    }
}

fn flex_direction(d: &lightningcss::properties::flex::FlexDirection) -> FlexDirection {
    use lightningcss::properties::flex::FlexDirection as F;
    match d {
        F::Row => FlexDirection::Row,
        F::RowReverse => FlexDirection::RowReverse,
        F::Column => FlexDirection::Column,
        F::ColumnReverse => FlexDirection::ColumnReverse,
    }
}

fn flex_wrap(w: &lightningcss::properties::flex::FlexWrap) -> FlexWrap {
    use lightningcss::properties::flex::FlexWrap as F;
    match w {
        F::NoWrap => FlexWrap::NoWrap,
        F::Wrap => FlexWrap::Wrap,
        F::WrapReverse => FlexWrap::WrapReverse,
    }
}

// ---------------------------------------------------------------------------
// CSS-wide keywords

/// Whether a property inherits by default.
pub fn is_inherited(name: &str) -> bool {
    matches!(
        name,
        "color"
            | "visibility"
            | "font"
            | "font-family"
            | "font-size"
            | "font-weight"
            | "font-style"
            | "line-height"
            | "text-align"
            | "text-transform"
            | "text-indent"
            | "white-space"
            | "word-break"
            | "list-style"
            | "list-style-type"
    )
}

/// Copies the fields a property name covers from `src` into `a.st`
/// (used for `inherit`, `initial`, `unset`, `revert`).
pub fn copy_property(name: &str, src: &ComputedStyle, a: &mut Apply<'_>) {
    let st = &mut *a.st;
    let sides = |name: &str| -> Option<usize> {
        if name.ends_with("-top") || name.ends_with("-block-start") {
            Some(T)
        } else if name.ends_with("-right") || name.ends_with("-inline-end") {
            Some(R)
        } else if name.ends_with("-bottom") || name.ends_with("-block-end") {
            Some(B)
        } else if name.ends_with("-left") || name.ends_with("-inline-start") {
            Some(L)
        } else {
            None
        }
    };
    match name {
        "display" => st.display = src.display,
        "position" => st.position = src.position,
        "top" => st.inset[T] = src.inset[T],
        "right" => st.inset[R] = src.inset[R],
        "bottom" => st.inset[B] = src.inset[B],
        "left" => st.inset[L] = src.inset[L],
        "inset" => st.inset = src.inset,
        "float" => st.float = src.float,
        "clear" => st.clear = src.clear,
        "width" => st.width = src.width,
        "height" => st.height = src.height,
        "min-width" => st.min_width = src.min_width,
        "min-height" => st.min_height = src.min_height,
        "max-width" => st.max_width = src.max_width,
        "max-height" => st.max_height = src.max_height,
        "margin" => st.margin = src.margin,
        "padding" => st.padding = src.padding,
        "border" => {
            st.border_width = src.border_width;
            st.border_style = src.border_style;
            st.border_color = src.border_color;
            a.border_current = [true; 4];
        }
        "border-width" => st.border_width = src.border_width,
        "border-style" => st.border_style = src.border_style,
        "border-color" => {
            st.border_color = src.border_color;
            a.border_current = [true; 4];
        }
        "border-radius" => st.border_radius = src.border_radius,
        "box-sizing" => st.box_sizing = src.box_sizing,
        "overflow" => {
            st.overflow_x = src.overflow_x;
            st.overflow_y = src.overflow_y;
        }
        "overflow-x" => st.overflow_x = src.overflow_x,
        "overflow-y" => st.overflow_y = src.overflow_y,
        "color" => st.color = src.color,
        "background" | "background-color" => st.background_color = src.background_color,
        "opacity" => st.opacity = src.opacity,
        "visibility" => st.visibility = src.visibility,
        "font" => {
            st.font_family = src.font_family;
            st.font_size = src.font_size;
            st.font_weight = src.font_weight;
            st.font_style = src.font_style;
            st.line_height = src.line_height;
        }
        "font-family" => st.font_family = src.font_family,
        "font-weight" => st.font_weight = src.font_weight,
        "font-style" => st.font_style = src.font_style,
        "line-height" => st.line_height = src.line_height,
        "text-align" => st.text_align = src.text_align,
        "text-decoration" | "text-decoration-line" => st.text_decoration = src.text_decoration,
        "text-transform" => st.text_transform = src.text_transform,
        "text-indent" => st.text_indent = src.text_indent,
        "white-space" => st.white_space = src.white_space,
        "word-break" => st.word_break = src.word_break,
        "vertical-align" => st.vertical_align = src.vertical_align,
        "list-style" | "list-style-type" => st.list_style_type = src.list_style_type,
        "z-index" => st.z_index = src.z_index,
        "gap" => {
            st.row_gap = src.row_gap;
            st.column_gap = src.column_gap;
        }
        "row-gap" => st.row_gap = src.row_gap,
        "column-gap" => st.column_gap = src.column_gap,
        "flex-direction" => st.flex_direction = src.flex_direction,
        "flex-wrap" => st.flex_wrap = src.flex_wrap,
        "flex-flow" => {
            st.flex_direction = src.flex_direction;
            st.flex_wrap = src.flex_wrap;
        }
        "flex-grow" => st.flex_grow = src.flex_grow,
        "flex-shrink" => st.flex_shrink = src.flex_shrink,
        "flex-basis" => st.flex_basis = src.flex_basis,
        "flex" => {
            st.flex_grow = src.flex_grow;
            st.flex_shrink = src.flex_shrink;
            st.flex_basis = src.flex_basis;
        }
        "order" => st.order = src.order,
        "justify-content" => st.justify_content = src.justify_content,
        "align-items" => st.align_items = src.align_items,
        "align-self" => st.align_self = src.align_self,
        "align-content" => st.align_content = src.align_content,
        "grid-template-columns" => st.grid_template_columns = src.grid_template_columns,
        "grid-template-rows" => st.grid_template_rows = src.grid_template_rows,
        "grid-column" => st.grid_column = src.grid_column,
        "grid-row" => st.grid_row = src.grid_row,
        n if n.starts_with("margin-") => {
            if let Some(s) = sides(n) {
                st.margin[s] = src.margin[s];
            }
        }
        n if n.starts_with("padding-") => {
            if let Some(s) = sides(n) {
                st.padding[s] = src.padding[s];
            }
        }
        n if n.starts_with("border-") && n.ends_with("-width") => {
            if let Some(s) = sides(&n[..n.len() - 6]) {
                st.border_width[s] = src.border_width[s];
            }
        }
        n if n.starts_with("border-") && n.ends_with("-style") => {
            if let Some(s) = sides(&n[..n.len() - 6]) {
                st.border_style[s] = src.border_style[s];
            }
        }
        n if n.starts_with("border-") && n.ends_with("-color") => {
            if let Some(s) = sides(&n[..n.len() - 6]) {
                st.border_color[s] = src.border_color[s];
                a.border_current[s] = true;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightningcss::declaration::DeclarationBlock;
    use lightningcss::stylesheet::ParserOptions;

    fn run(css: &str) -> ComputedStyle {
        let block = DeclarationBlock::parse_string(css, ParserOptions::default()).unwrap();
        let mut st = ComputedStyle::INITIAL;
        let parent = ComputedStyle::INITIAL;
        let mut tracks = TrackTable::default();
        let mut a = Apply {
            st: &mut st,
            parent: &parent,
            len: LenCtx {
                em: 16.0,
                rem: 16.0,
                vw: 1280.0,
                vh: 800.0,
            },
            line_height_factor: None,
            border_current: [true; 4],
            tracks: &mut tracks,
        };
        for p in &block.declarations {
            apply(p, &mut a);
        }
        a.finish();
        st
    }

    #[test]
    fn lengths_resolve() {
        let s = run("width: 50%; height: 2em; margin: 1rem auto 3pt 10vw; padding: 1ch; min-width: calc(10px + 2em); max-width: calc(100% - 20px)");
        assert_eq!(s.width, Length::percent(50.0));
        assert_eq!(s.height, Length::px(32.0));
        assert_eq!(s.margin[0], Length::px(16.0));
        assert!(s.margin[1].is_auto());
        assert_eq!(s.margin[2], Length::px(4.0));
        assert_eq!(s.margin[3].unit, LengthUnit::Vw);
        assert_eq!(s.padding[0], Length::px(8.0));
        assert_eq!(s.min_width, Length::px(42.0));
        assert_eq!(s.max_width, Length::percent(100.0));
    }

    #[test]
    fn borders_and_colors() {
        let s = run("border: 2px solid; color: rgb(1 2 3); border-left-color: #fff; border-bottom-style: none; border-top-width: thick; border-radius: 4px 8px");
        assert_eq!(s.border_color[0], Rgba::rgb(1, 2, 3));
        assert_eq!(s.border_color[3], Rgba::WHITE);
        assert_eq!(s.border_width[2], 0.0);
        assert_eq!(s.border_width[0], 5.0);
        assert_eq!(s.border_style[1], BorderStyle::Solid);
        assert_eq!(s.border_radius[1], Length::px(8.0));
        assert_eq!(s.border_radius[3], Length::px(8.0));
        let s = run("background: url(x.png) red; opacity: 2");
        assert_eq!(s.background_color, Rgba::rgb(255, 0, 0));
        assert_eq!(s.opacity, 1.0);
    }

    #[test]
    fn display_and_text() {
        let s = run("display: inline-flex; float: right; clear: both; white-space: nowrap; text-decoration: underline line-through; text-align: end; vertical-align: sub; list-style: square inside; font-family: 'Fira Code', monospace; font-weight: 650; font-style: oblique; line-height: 1.5; text-transform: uppercase; z-index: 3; overflow: auto; position: sticky");
        assert_eq!(s.display, Display::InlineFlex);
        assert_eq!(s.float, Float::Right);
        assert_eq!(s.clear, Clear::Both);
        assert_eq!(s.white_space, WhiteSpace::Nowrap);
        assert_eq!(s.text_decoration, TextDecoration::UNDERLINE | TextDecoration::LINE_THROUGH);
        assert_eq!(s.text_align, TextAlign::Right);
        assert_eq!(s.vertical_align, VerticalAlign::Bottom);
        assert_eq!(s.list_style_type, ListStyleType::Square);
        assert_eq!(s.font_family, FontFamily::Mono);
        assert_eq!(s.font_weight, FontWeight::Bold);
        assert_eq!(s.font_style, FontStyle::Italic);
        assert_eq!(s.line_height, Length::px(24.0));
        assert_eq!(s.text_transform, TextTransform::Uppercase);
        assert_eq!(s.z_index, 3);
        assert_eq!(s.overflow_x, Overflow::Hidden);
        assert_eq!(s.position, Position::Sticky);
        assert_eq!(run("display: list-item").display, Display::ListItem);
        assert_eq!(run("display: none").display, Display::None);
        assert_eq!(run("display: table-cell").display, Display::TableCell);
        assert_eq!(run("list-style-type: lower-roman").list_style_type, ListStyleType::Decimal);
        assert_eq!(run("list-style-type: none").list_style_type, ListStyleType::None);
    }

    #[test]
    fn flex_and_grid() {
        let s = run("flex: 2 0 10px; flex-flow: column wrap; justify-content: space-between; align-items: center; align-self: flex-end; gap: 4px 8px; grid-template-columns: repeat(2, 1fr) 100px; grid-column: 1 / span 2; order: -1");
        assert_eq!(s.flex_grow, 2.0);
        assert_eq!(s.flex_shrink, 0.0);
        assert_eq!(s.flex_basis, Length::px(10.0));
        assert_eq!(s.flex_direction, FlexDirection::Column);
        assert_eq!(s.flex_wrap, FlexWrap::Wrap);
        assert_eq!(s.justify_content, JustifyContent::SpaceBetween);
        assert_eq!(s.align_items, AlignItems::Center);
        assert_eq!(s.align_self, AlignItems::FlexEnd);
        assert_eq!(s.row_gap, Length::px(4.0));
        assert_eq!(s.column_gap, Length::px(8.0));
        assert_eq!(s.grid_template_columns, TrackListRef { off: 0, len: 3 });
        assert_eq!(s.grid_column.start, GridLine { kind: GridLineKind::Line, value: 1 });
        assert_eq!(s.grid_column.end, GridLine { kind: GridLineKind::Span, value: 2 });
        assert_eq!(s.order, -1);
    }

    #[test]
    fn font_sizes() {
        let ctx = LenCtx {
            em: 20.0,
            rem: 16.0,
            vw: 1280.0,
            vh: 800.0,
        };
        let parse = |s: &str| {
            let css = format!("font-size: {s}");
            let b = DeclarationBlock::parse_string(&css, ParserOptions::default()).unwrap();
            match &b.declarations[0] {
                Property::FontSize(f) => font_size(f, 20.0, &ctx),
                _ => panic!(),
            }
        };
        assert_eq!(parse("2em"), 40.0);
        assert_eq!(parse("150%"), 30.0);
        assert_eq!(parse("1rem"), 16.0);
        assert_eq!(parse("12pt"), 16.0);
        assert_eq!(parse("medium"), 16.0);
        assert_eq!(parse("larger"), 24.0);
        assert_eq!(parse("smaller"), 20.0 / 1.2);
    }

    #[test]
    fn family_mapping() {
        let parse = |s: &str| {
            let css = format!("font-family: {s}");
            let b = DeclarationBlock::parse_string(&css, ParserOptions::default()).unwrap();
            match &b.declarations[0] {
                Property::FontFamily(f) => font_family(f),
                _ => panic!(),
            }
        };
        assert_eq!(parse("Georgia, serif"), FontFamily::Serif);
        assert_eq!(parse("Unknown Face, sans-serif"), FontFamily::Sans);
        assert_eq!(parse("Unknown Face"), FontFamily::Sans);
        assert_eq!(parse("Menlo, Consolas, monospace"), FontFamily::Mono);
        assert_eq!(parse("system-ui"), FontFamily::Sans);
    }

    #[test]
    fn copy_for_keywords() {
        let mut st = ComputedStyle::INITIAL;
        let mut src = ComputedStyle::INITIAL;
        src.margin[3] = Length::px(7.0);
        src.border_width[1] = 2.0;
        src.color = Rgba::rgb(9, 9, 9);
        let parent = ComputedStyle::INITIAL;
        let mut tracks = TrackTable::default();
        let mut a = Apply {
            st: &mut st,
            parent: &parent,
            len: LenCtx { em: 16.0, rem: 16.0, vw: 0.0, vh: 0.0 },
            line_height_factor: None,
            border_current: [true; 4],
            tracks: &mut tracks,
        };
        copy_property("margin-left", &src, &mut a);
        copy_property("border-right-width", &src, &mut a);
        copy_property("color", &src, &mut a);
        assert_eq!(st.margin[3], Length::px(7.0));
        assert_eq!(st.border_width[1], 2.0);
        assert_eq!(st.color, Rgba::rgb(9, 9, 9));
        assert!(is_inherited("color") && !is_inherited("margin"));
    }
}
