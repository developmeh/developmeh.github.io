//! The rkyv archive root and its tables (plan §4).

use css_subset::{ComputedStyle, GridTrack, NodeFlags};
use rkyv::{Archive, Deserialize, Portable, Serialize};

/// "No node": used for a missing `parent`/`first_child`/`next_sibling`.
pub const NONE: u32 = u32::MAX;

/// Declares a `#[repr(u8)]` enum stored as a single validated byte.
macro_rules! byte_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$vmeta:meta])* $variant:ident = $value:literal ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(
            Clone, Copy, Debug, Default, PartialEq, Eq, Hash,
            Archive, Serialize, Deserialize, Portable, rkyv::bytecheck::CheckBytes,
        )]
        #[rkyv(as = Self)]
        #[bytecheck(crate = rkyv::bytecheck)]
        #[repr(u8)]
        #[allow(missing_docs)]
        pub enum $name {
            $( $(#[$vmeta])* $variant = $value ),+
        }

        impl $name {
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

byte_enum! {
    /// What a node is. Non-element kinds come first; the rest is the tag
    /// of an element (unknown tags map to `Element`).
    NodeKind {
        /// A text run (`text_off`/`text_len` are meaningful).
        #[default]
        Text = 0,
        /// `<img>` with an entry in the image table.
        Image = 1,
        /// Inline `<svg>` with an entry in the image table.
        Svg = 2,
        /// `<input>`, `<textarea>`, `<select>`, `<button>`.
        FormControl = 3,
        /// `<br>`.
        LineBreak = 4,
        /// Generated list marker (`::marker` substitute).
        Marker = 5,
        /// Any element whose tag has no dedicated value below.
        Element = 6,
        /// `::before`/`::after` pseudo-element with string `content`.
        Pseudo = 7,

        Html = 16,
        Body = 17,
        Div = 18,
        P = 19,
        Span = 20,
        A = 21,
        H1 = 22,
        H2 = 23,
        H3 = 24,
        H4 = 25,
        H5 = 26,
        H6 = 27,
        Ul = 28,
        Ol = 29,
        Li = 30,
        Dl = 31,
        Dt = 32,
        Dd = 33,
        Pre = 34,
        Code = 35,
        Blockquote = 36,
        Em = 37,
        Strong = 38,
        B = 39,
        I = 40,
        U = 41,
        S = 42,
        Small = 43,
        Sub = 44,
        Sup = 45,
        Hr = 46,
        Table = 47,
        Thead = 48,
        Tbody = 49,
        Tfoot = 50,
        Tr = 51,
        Td = 52,
        Th = 53,
        Caption = 54,
        Nav = 55,
        Main = 56,
        Header = 57,
        Footer = 58,
        Aside = 59,
        Article = 60,
        Section = 61,
        Figure = 62,
        Figcaption = 63,
        Form = 64,
        Label = 65,
        Fieldset = 66,
        Legend = 67,
        Details = 68,
        Summary = 69,
        Iframe = 70,
        Video = 71,
        Audio = 72,
        Canvas = 73,
        Object = 74,
        Noscript = 75,
        Kbd = 76,
        Samp = 77,
        Var = 78,
        Q = 79,
        Cite = 80,
        Abbr = 81,
        Time = 82,
        Mark = 83,
        Wbr = 84,
    }
}

impl NodeKind {
    /// Whether this kind is an element (as opposed to text or generated
    /// content).
    pub const fn is_element(self) -> bool {
        !matches!(self, NodeKind::Text | NodeKind::Marker | NodeKind::Pseudo)
    }

    /// Heading level 1..=6 for `H1`..`H6`, else `None`.
    pub const fn heading_level(self) -> Option<u8> {
        match self {
            NodeKind::H1 => Some(1),
            NodeKind::H2 => Some(2),
            NodeKind::H3 => Some(3),
            NodeKind::H4 => Some(4),
            NodeKind::H5 => Some(5),
            NodeKind::H6 => Some(6),
            _ => None,
        }
    }
}

byte_enum! {
    /// ARIA-ish role, computed by the loader (plan §12).
    Role {
        #[default]
        Generic = 0,
        Document = 1,
        StaticText = 2,
        Paragraph = 3,
        Heading1 = 4,
        Heading2 = 5,
        Heading3 = 6,
        Heading4 = 7,
        Heading5 = 8,
        Heading6 = 9,
        Link = 10,
        Button = 11,
        TextField = 12,
        Checkbox = 13,
        Radio = 14,
        ComboBox = 15,
        Img = 16,
        List = 17,
        ListItem = 18,
        Navigation = 19,
        Main = 20,
        Banner = 21,
        ContentInfo = 22,
        Complementary = 23,
        Article = 24,
        Region = 25,
        Table = 26,
        Row = 27,
        Cell = 28,
        ColumnHeader = 29,
        RowHeader = 30,
        Code = 31,
        Blockquote = 32,
        Emphasis = 33,
        Strong = 34,
        Separator = 35,
        Figure = 36,
        Caption = 37,
        Form = 38,
        Label = 39,
        LineBreak = 40,
        Presentation = 41,
        Group = 42,
    }
}

impl Role {
    /// Whether a landmark or heading that compression must never collapse.
    pub const fn is_structural(self) -> bool {
        !matches!(self, Role::Generic | Role::StaticText | Role::Presentation)
    }
}

byte_enum! {
    /// Attribute keys kept after cascade (plan §4). Values live in the text
    /// blob; `href`/`src`/`action` are absolute URLs.
    AttrKey {
        #[default]
        Href = 0,
        Src = 1,
        Alt = 2,
        Id = 3,
        Name = 4,
        Value = 5,
        Type = 6,
        Action = 7,
        Method = 8,
        Placeholder = 9,
        Checked = 10,
        For = 11,
        Lang = 12,
        Title = 13,
        AriaLabel = 14,
        Colspan = 15,
        Rowspan = 16,
    }
}

byte_enum! {
    /// Encoding of an image blob.
    ImageFormat {
        #[default]
        Unsupported = 0,
        Jpeg = 1,
        Png = 2,
        Svg = 3,
    }
}

byte_enum! {
    /// HTTP method of a form.
    FormMethod {
        #[default]
        Get = 0,
        Post = 1,
    }
}

/// One DOM node. Nodes are stored in pre-order; index 0 is the root.
///
/// Field order is chosen so the archived struct is exactly 32 bytes: the
/// plan's field list sums to 33, so the padding word is a single byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
pub struct Node {
    /// Index into `Page.styles`.
    pub style: u16,
    /// What the node is.
    pub kind: NodeKind,
    /// [`NodeFlags`] bits.
    pub flags: u8,
    /// Accessibility role.
    pub role: Role,
    /// Always zero.
    pub reserved: u8,
    /// Length of the accessible name in the text blob.
    pub name_len: u16,
    /// Text run start in `Page.text` (text nodes; unused for elements).
    pub text_off: u32,
    /// Text run length.
    pub text_len: u32,
    /// Parent index or [`NONE`] for the root.
    pub parent: u32,
    /// First child index (always `self + 1` when present) or [`NONE`].
    pub first_child: u32,
    /// Next sibling index or [`NONE`].
    pub next_sibling: u32,
    /// Accessible name start in `Page.text` (may alias `text_off`).
    pub name_off: u32,
}

impl Node {
    /// A node with no children, no text and the given style.
    pub const fn new(kind: NodeKind, role: Role, style: u16, parent: u32) -> Node {
        Node {
            style,
            kind,
            flags: 0,
            role,
            reserved: 0,
            name_len: 0,
            text_off: 0,
            text_len: 0,
            parent,
            first_child: NONE,
            next_sibling: NONE,
            name_off: 0,
        }
    }

    /// Typed view of `flags`.
    pub fn node_flags(&self) -> NodeFlags {
        NodeFlags::from_bits_retain(self.flags)
    }
}

impl ArchivedNode {
    /// Typed view of `flags`.
    pub fn node_flags(&self) -> NodeFlags {
        NodeFlags::from_bits_retain(self.flags)
    }

    /// Copies the archived node out as a native [`Node`].
    pub fn to_native(&self) -> Node {
        Node {
            style: self.style.to_native(),
            kind: self.kind,
            flags: self.flags,
            role: self.role,
            reserved: self.reserved,
            name_len: self.name_len.to_native(),
            text_off: self.text_off.to_native(),
            text_len: self.text_len.to_native(),
            parent: self.parent.to_native(),
            first_child: self.first_child.to_native(),
            next_sibling: self.next_sibling.to_native(),
            name_off: self.name_off.to_native(),
        }
    }
}

const _: () = assert!(
    size_of::<ArchivedNode>() == 32,
    "ArchivedNode must be 32 bytes"
);

/// A kept attribute. The table is sorted by `node` for binary search.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
pub struct Attr {
    /// Owning node.
    pub node: u32,
    /// Value start in `Page.text`.
    pub val_off: u32,
    /// Value length.
    pub val_len: u32,
    /// Which attribute.
    pub key: AttrKey,
}

/// An image (or inline SVG) whose bytes live in `Page.blobs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
pub struct ImageRef {
    /// Owning node.
    pub node: u32,
    /// Blob start in `Page.blobs`.
    pub blob_off: u32,
    /// Blob length.
    pub blob_len: u32,
    /// Intrinsic width in CSS px.
    pub width: u16,
    /// Intrinsic height in CSS px.
    pub height: u16,
    /// Encoding.
    pub format: ImageFormat,
}

/// A `<form>`; its fields are the node indices `Page.fields[first_field..][..field_count]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug, Clone, Copy), compare(PartialEq))]
pub struct Form {
    /// Owning node.
    pub node: u32,
    /// Absolute action URL start in `Page.text`.
    pub action_off: u32,
    /// Action URL length.
    pub action_len: u32,
    /// First entry in `Page.fields`.
    pub first_field: u32,
    /// Number of entries in `Page.fields`.
    pub field_count: u32,
    /// Submission method.
    pub method: FormMethod,
}

/// The archive root.
#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[rkyv(derive(Debug))]
pub struct Page {
    /// URL after redirects.
    pub final_url: String,
    /// Document title.
    pub title: String,
    /// Viewport-width bucket the styles were cascaded for (CSS px).
    pub viewport_width: u16,
    /// Media-query breakpoints encountered, sorted ascending (CSS px).
    pub breakpoints: Vec<u16>,
    /// Nodes in pre-order; index 0 is the root.
    pub nodes: Vec<Node>,
    /// UTF-8 blob holding text runs, names and attribute values.
    pub text: Vec<u8>,
    /// Interned computed styles.
    pub styles: Vec<ComputedStyle>,
    /// Grid track lists referenced by `ComputedStyle::grid_template_*`.
    pub tracks: Vec<GridTrack>,
    /// Kept attributes, sorted by node.
    pub attrs: Vec<Attr>,
    /// Images and inline SVGs.
    pub images: Vec<ImageRef>,
    /// Concatenated image bytes.
    pub blobs: Vec<u8>,
    /// Forms.
    pub forms: Vec<Form>,
    /// Node indices of form fields, grouped per form.
    pub fields: Vec<u32>,
    /// Node indices of the body's block children (scrollbar units),
    /// strictly increasing.
    pub top_level: Vec<u32>,
}

impl Page {
    /// A minimal valid page: one root node with the initial style.
    pub fn empty(final_url: impl Into<String>, viewport_width: u16) -> Page {
        Page {
            final_url: final_url.into(),
            title: String::new(),
            viewport_width,
            breakpoints: Vec::new(),
            nodes: vec![Node::new(NodeKind::Html, Role::Document, 0, NONE)],
            text: Vec::new(),
            styles: vec![ComputedStyle::INITIAL],
            tracks: Vec::new(),
            attrs: Vec::new(),
            images: Vec::new(),
            blobs: Vec::new(),
            forms: Vec::new(),
            fields: Vec::new(),
            top_level: Vec::new(),
        }
    }

    /// Appends `bytes` to the text blob and returns `(offset, len)`.
    pub fn push_text(&mut self, s: &str) -> (u32, u32) {
        let off = self.text.len() as u32;
        self.text.extend_from_slice(s.as_bytes());
        (off, s.len() as u32)
    }
}

impl ArchivedPage {
    /// The text run of a text node, or the empty string for others.
    pub fn text_of(&self, node: &ArchivedNode) -> &str {
        self.text_slice(node.text_off.to_native(), node.text_len.to_native())
    }

    /// Accessible name of a node.
    pub fn name_of(&self, node: &ArchivedNode) -> &str {
        self.text_slice(
            node.name_off.to_native(),
            u32::from(node.name_len.to_native()),
        )
    }

    /// Value of an attribute.
    pub fn attr_value(&self, attr: &ArchivedAttr) -> &str {
        self.text_slice(attr.val_off.to_native(), attr.val_len.to_native())
    }

    /// Attributes of the node at `index` (empty if none). O(log n).
    pub fn attrs_of(&self, index: u32) -> &[ArchivedAttr] {
        let attrs = self.attrs.as_slice();
        let start = attrs.partition_point(|a| a.node.to_native() < index);
        let end = start + attrs[start..].partition_point(|a| a.node.to_native() == index);
        &attrs[start..end]
    }

    /// Looks up one attribute of a node.
    pub fn attr(&self, index: u32, key: AttrKey) -> Option<&str> {
        self.attrs_of(index)
            .iter()
            .find(|a| a.key == key)
            .map(|a| self.attr_value(a))
    }

    /// Bytes of an image blob.
    pub fn blob_of(&self, image: &ArchivedImageRef) -> &[u8] {
        let off = image.blob_off.to_native() as usize;
        let len = image.blob_len.to_native() as usize;
        &self.blobs[off..off + len]
    }

    /// Iterates over the child indices of `index`.
    pub fn children(&self, index: u32) -> impl Iterator<Item = u32> + '_ {
        let nodes = self.nodes.as_slice();
        let mut next = nodes[index as usize].first_child.to_native();
        std::iter::from_fn(move || {
            if next == NONE {
                return None;
            }
            let cur = next;
            next = nodes[cur as usize].next_sibling.to_native();
            Some(cur)
        })
    }

    /// Slice of the text blob. Ranges were validated at map time.
    fn text_slice(&self, off: u32, len: u32) -> &str {
        let off = off as usize;
        let bytes = &self.text.as_slice()[off..off + len as usize];
        // Validation guarantees the blob is UTF-8 and runs are on char
        // boundaries; fall back to "" rather than panic if ever violated.
        std::str::from_utf8(bytes).unwrap_or("")
    }
}
