//! Fieldless property enums. Every enum is `#[repr(u8)]`, `Portable`, and
//! archived as itself, so the byte in the page file *is* the enum value and
//! bytecheck rejects any byte outside the declared discriminants.

/// Declares a `#[repr(u8)]` CSS keyword enum that is its own archived type.
macro_rules! css_enum {
    (
        $(#[$meta:meta])*
        $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $value:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(
            Clone, Copy, Debug, Default, PartialEq, Eq, Hash,
            rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
            rkyv::Portable, rkyv::bytecheck::CheckBytes,
        )]
        #[rkyv(as = Self)]
        #[bytecheck(crate = rkyv::bytecheck)]
        #[repr(u8)]
        #[allow(missing_docs)] // variants are the CSS keywords themselves
        pub enum $name {
            $( $(#[$vmeta])* $variant = $value ),+
        }

        impl $name {
            /// All variants, in declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];

            /// Discriminant byte as stored in the page file.
            #[inline]
            pub const fn as_u8(self) -> u8 {
                self as u8
            }

            /// Decodes a discriminant byte; `None` if it is not a variant.
            pub fn from_u8(byte: u8) -> Option<Self> {
                match byte {
                    $( $value => Some($name::$variant), )+
                    _ => None,
                }
            }
        }
    };
}

css_enum! {
    /// `display`. Table values are kept so the loader can record them; before
    /// M4 the renderer treats `table*` as block/inline-block (plan §5).
    Display {
        #[default]
        Inline = 0,
        Block = 1,
        InlineBlock = 2,
        Flex = 3,
        InlineFlex = 4,
        Grid = 5,
        InlineGrid = 6,
        ListItem = 7,
        Table = 8,
        TableRow = 9,
        TableCell = 10,
        TableRowGroup = 11,
        TableHeaderGroup = 12,
        TableFooterGroup = 13,
        TableCaption = 14,
        /// Only ever present in interactive mode (M6); frozen pages drop
        /// `display: none` subtrees in compression pass 1.
        None = 15,
    }
}

impl Display {
    /// True for values that establish a block-level box in normal flow.
    pub const fn is_block_level(self) -> bool {
        matches!(
            self,
            Display::Block
                | Display::Flex
                | Display::Grid
                | Display::ListItem
                | Display::Table
                | Display::TableRow
                | Display::TableRowGroup
                | Display::TableHeaderGroup
                | Display::TableFooterGroup
                | Display::TableCaption
        )
    }

    /// True for flex and grid containers (whose children are "items").
    pub const fn is_flex_or_grid_container(self) -> bool {
        matches!(
            self,
            Display::Flex | Display::InlineFlex | Display::Grid | Display::InlineGrid
        )
    }
}

css_enum! {
    /// `position`. `fixed` degrades to absolute and `sticky` to relative at
    /// layout time (plan §5 fallbacks); the computed value is kept verbatim.
    Position {
        #[default]
        Static = 0,
        Relative = 1,
        Absolute = 2,
        Fixed = 3,
        Sticky = 4,
    }
}

css_enum! {
    /// `float`.
    Float {
        #[default]
        None = 0,
        Left = 1,
        Right = 2,
    }
}

css_enum! {
    /// `clear`.
    Clear {
        #[default]
        None = 0,
        Left = 1,
        Right = 2,
        Both = 3,
    }
}

css_enum! {
    /// `border-*-style`. `dashed`/`dotted` render as solid (plan §5).
    BorderStyle {
        #[default]
        None = 0,
        Solid = 1,
        Dashed = 2,
        Dotted = 3,
    }
}

css_enum! {
    /// `box-sizing`.
    BoxSizing {
        #[default]
        ContentBox = 0,
        BorderBox = 1,
    }
}

css_enum! {
    /// `overflow`. `scroll`/`auto` map to `Hidden` (clip, no inner scrollbars).
    Overflow {
        #[default]
        Visible = 0,
        Hidden = 1,
    }
}

css_enum! {
    /// `visibility`.
    Visibility {
        #[default]
        Visible = 0,
        Hidden = 1,
    }
}

css_enum! {
    /// `font-family`, already mapped to one of the three bundled Noto faces.
    FontFamily {
        #[default]
        Sans = 0,
        Serif = 1,
        Mono = 2,
    }
}

css_enum! {
    /// `font-weight`, bucketed to 400/700.
    FontWeight {
        #[default]
        Normal = 0,
        Bold = 1,
    }
}

css_enum! {
    /// `font-style`. Oblique is treated as italic.
    FontStyle {
        #[default]
        Normal = 0,
        Italic = 1,
    }
}

css_enum! {
    /// `text-align`. `justify` renders as `start` (left, since LTR only).
    TextAlign {
        #[default]
        Left = 0,
        Right = 1,
        Center = 2,
        Justify = 3,
    }
}

css_enum! {
    /// `text-transform`.
    TextTransform {
        #[default]
        None = 0,
        Uppercase = 1,
        Lowercase = 2,
        Capitalize = 3,
    }
}

css_enum! {
    /// `white-space`.
    WhiteSpace {
        #[default]
        Normal = 0,
        Nowrap = 1,
        Pre = 2,
        PreWrap = 3,
        PreLine = 4,
    }
}

impl WhiteSpace {
    /// Whether runs of collapsible whitespace collapse to one space.
    pub const fn collapses(self) -> bool {
        matches!(
            self,
            WhiteSpace::Normal | WhiteSpace::Nowrap | WhiteSpace::PreLine
        )
    }

    /// Whether newlines in the source are preserved as line breaks.
    pub const fn preserves_newlines(self) -> bool {
        matches!(
            self,
            WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::PreLine
        )
    }

    /// Whether lines may wrap at soft break opportunities.
    pub const fn wraps(self) -> bool {
        !matches!(self, WhiteSpace::Nowrap | WhiteSpace::Pre)
    }
}

css_enum! {
    /// `word-break`.
    WordBreak {
        #[default]
        Normal = 0,
        BreakAll = 1,
    }
}

css_enum! {
    /// `vertical-align` (inline boxes and replaced elements only).
    VerticalAlign {
        #[default]
        Baseline = 0,
        Middle = 1,
        Top = 2,
        Bottom = 3,
    }
}

css_enum! {
    /// `list-style-type`.
    ListStyleType {
        #[default]
        Disc = 0,
        Circle = 1,
        Square = 2,
        Decimal = 3,
        None = 4,
    }
}

css_enum! {
    /// `flex-direction`.
    FlexDirection {
        #[default]
        Row = 0,
        RowReverse = 1,
        Column = 2,
        ColumnReverse = 3,
    }
}

css_enum! {
    /// `flex-wrap`.
    FlexWrap {
        #[default]
        NoWrap = 0,
        Wrap = 1,
        WrapReverse = 2,
    }
}

css_enum! {
    /// `justify-content` (flex) and `justify-items`-free subset for grid.
    JustifyContent {
        #[default]
        FlexStart = 0,
        FlexEnd = 1,
        Center = 2,
        SpaceBetween = 3,
        SpaceAround = 4,
        SpaceEvenly = 5,
        Stretch = 6,
    }
}

css_enum! {
    /// `align-items` / `align-self` / `align-content` values.
    AlignItems {
        #[default]
        Stretch = 0,
        FlexStart = 1,
        FlexEnd = 2,
        Center = 3,
        Baseline = 4,
        /// `align-self: auto` (inherit the container's `align-items`).
        Auto = 5,
        SpaceBetween = 6,
        SpaceAround = 7,
        SpaceEvenly = 8,
    }
}

css_enum! {
    /// Unit of a [`super::Length`]. `em`/`rem`/`pt`/`ch` are resolved to px
    /// by the loader; `%`, `vw`, `vh` stay symbolic so intra-bucket resizes
    /// need no re-cascade (plan §2.3).
    LengthUnit {
        #[default]
        Px = 0,
        Percent = 1,
        Vw = 2,
        Vh = 3,
        /// `auto` (also `none` for `max-*` and `normal` for `line-height`).
        Auto = 4,
    }
}

css_enum! {
    /// Sizing function of one grid track.
    TrackSize {
        #[default]
        Auto = 0,
        Px = 1,
        Percent = 2,
        Fr = 3,
        MinContent = 4,
        MaxContent = 5,
    }
}

css_enum! {
    /// Kind of a `grid-column`/`grid-row` line reference.
    GridLineKind {
        #[default]
        Auto = 0,
        /// An explicit line number (1-based, negative counts from the end).
        Line = 1,
        /// `span n`.
        Span = 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_discriminants() {
        for &d in Display::ALL {
            assert_eq!(Display::from_u8(d.as_u8()), Some(d));
        }
        assert_eq!(Display::from_u8(200), None);
        assert_eq!(Display::default(), Display::Inline);
    }

    #[test]
    fn archived_enum_is_one_byte() {
        assert_eq!(size_of::<rkyv::Archived<Display>>(), 1);
        assert_eq!(size_of::<rkyv::Archived<LengthUnit>>(), 1);
    }

    #[test]
    fn bytecheck_rejects_unknown_discriminant() {
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&Position::Fixed).unwrap();
        assert!(rkyv::access::<Position, rkyv::rancor::Error>(&bytes).is_ok());
        let mut bad = bytes.clone();
        *bad.last_mut().unwrap() = 99;
        assert!(rkyv::access::<Position, rkyv::rancor::Error>(&bad).is_err());
    }
}
