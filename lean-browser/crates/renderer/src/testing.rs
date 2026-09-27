//! Test support: builds page files through `page-format`'s writer so the
//! renderer's tests (and `--write-demo`) never depend on the loader.

use css_subset::{
    BorderStyle, ComputedStyle, Display, Length, ListStyleType, NodeFlags, Rgba, StyleTable,
    TextAlign, TextDecoration, WhiteSpace,
};
use page_format::{
    Attr, AttrKey, ImageFormat, ImageRef, Node, NodeKind, Page, PageFile, Role, NONE,
};

/// Incrementally builds a pre-order node tree.
pub struct PageBuilder {
    page: Page,
    styles: StyleTable,
    /// Open elements (the current parent is the last).
    stack: Vec<u32>,
    last_child: Vec<u32>,
}

/// A block style with the given tweaks applied to the initial style.
pub fn style(f: impl FnOnce(&mut ComputedStyle)) -> ComputedStyle {
    let mut s = ComputedStyle::INITIAL;
    f(&mut s);
    s
}

/// `display: block` with tweaks.
pub fn block(f: impl FnOnce(&mut ComputedStyle)) -> ComputedStyle {
    style(|s| {
        s.display = Display::Block;
        f(s);
    })
}

/// `display: inline` with tweaks.
pub fn inline(f: impl FnOnce(&mut ComputedStyle)) -> ComputedStyle {
    style(f)
}

/// Sets all four margins.
pub fn margins(s: &mut ComputedStyle, top: f32, right: f32, bottom: f32, left: f32) {
    s.margin = [
        Length::px(top),
        Length::px(right),
        Length::px(bottom),
        Length::px(left),
    ];
}

/// Sets all four paddings.
pub fn paddings(s: &mut ComputedStyle, all: f32) {
    s.padding = [Length::px(all); 4];
}

/// Sets a solid border on all sides.
pub fn border(s: &mut ComputedStyle, width: f32, color: Rgba) {
    s.border_width = [width; 4];
    s.border_style = [BorderStyle::Solid; 4];
    s.border_color = [color; 4];
}

impl PageBuilder {
    /// A page with `<html>` and `<body>` open; `body_style` is applied to
    /// the body (the UA default is an 8px margin).
    pub fn new(url: &str, viewport_width: u16, body_style: ComputedStyle) -> PageBuilder {
        let mut page = Page::empty(url, viewport_width);
        page.styles.clear();
        page.nodes.clear();
        let mut b = PageBuilder {
            page,
            styles: StyleTable::new(),
            stack: Vec::new(),
            last_child: Vec::new(),
        };
        b.open(NodeKind::Html, Role::Document, block(|_| {}));
        b.open(NodeKind::Body, Role::Generic, body_style);
        b
    }

    /// The usual UA body: block with 8px margins.
    pub fn ua_body() -> ComputedStyle {
        block(|s| margins(s, 8.0, 8.0, 8.0, 8.0))
    }

    fn intern(&mut self, s: &ComputedStyle) -> u16 {
        self.styles.intern(s).expect("style table")
    }

    fn push(&mut self, mut node: Node) -> u32 {
        let idx = self.page.nodes.len() as u32;
        if let Some(&parent) = self.stack.last() {
            node.parent = parent;
            let p = &mut self.page.nodes[parent as usize];
            if p.first_child == NONE {
                p.first_child = idx;
            } else {
                let last = self.last_child[parent as usize];
                self.page.nodes[last as usize].next_sibling = idx;
            }
            self.last_child[parent as usize] = idx;
        } else {
            node.parent = NONE;
        }
        self.page.nodes.push(node);
        self.last_child.push(NONE);
        idx
    }

    /// Opens an element; children go inside until [`Self::close`].
    pub fn open(&mut self, kind: NodeKind, role: Role, s: ComputedStyle) -> u32 {
        let sid = self.intern(&s);
        let mut node = Node::new(kind, role, sid, NONE);
        if s.display.is_block_level() {
            node.flags |= NodeFlags::BLOCK.bits();
        }
        if s.white_space.preserves_newlines() {
            node.flags |= NodeFlags::PRE.bits();
        }
        let idx = self.push(node);
        self.stack.push(idx);
        idx
    }

    /// Closes the current element.
    pub fn close(&mut self) {
        self.stack.pop();
    }

    /// Adds a childless element (e.g. `<br>`, `<hr>`).
    pub fn leaf(&mut self, kind: NodeKind, role: Role, s: ComputedStyle) -> u32 {
        let idx = self.open(kind, role, s);
        self.close();
        idx
    }

    /// Adds a text run with the given (inherited) style.
    pub fn text(&mut self, text: &str, s: ComputedStyle) -> u32 {
        let sid = self.intern(&s);
        let (off, len) = self.page.push_text(text);
        let mut node = Node::new(NodeKind::Text, Role::StaticText, sid, NONE);
        node.text_off = off;
        node.text_len = len;
        node.name_off = off;
        node.name_len = len.min(u32::from(u16::MAX)) as u16;
        self.push(node)
    }

    /// Adds an `<a href>` with a text child.
    pub fn link(&mut self, href: &str, text: &str, s: ComputedStyle) -> u32 {
        let a = self.open(NodeKind::A, Role::Link, s);
        self.page.nodes[a as usize].flags |=
            (NodeFlags::LINK | NodeFlags::FOCUSABLE | NodeFlags::HAS_ATTRS).bits();
        self.attr(a, AttrKey::Href, href);
        self.text(text, s);
        self.close();
        a
    }

    /// Adds an attribute.
    pub fn attr(&mut self, node: u32, key: AttrKey, value: &str) {
        let (val_off, val_len) = self.page.push_text(value);
        self.page.attrs.push(Attr {
            node,
            key,
            val_off,
            val_len,
        });
        self.page.nodes[node as usize].flags |= NodeFlags::HAS_ATTRS.bits();
    }

    /// Adds an `<img>` whose bytes go into the blob table.
    pub fn image(
        &mut self,
        s: ComputedStyle,
        bytes: &[u8],
        format: ImageFormat,
        width: u16,
        height: u16,
    ) -> u32 {
        let idx = self.leaf(NodeKind::Image, Role::Img, s);
        self.page.nodes[idx as usize].flags |= NodeFlags::INLINE_REPLACED.bits();
        let blob_off = self.page.blobs.len() as u32;
        self.page.blobs.extend_from_slice(bytes);
        self.page.images.push(ImageRef {
            node: idx,
            blob_off,
            blob_len: bytes.len() as u32,
            width,
            height,
            format,
        });
        idx
    }

    /// Sets the document title.
    pub fn title(&mut self, title: &str) {
        self.page.title = title.to_string();
    }

    /// Closes everything and finalizes the tables.
    pub fn finish(mut self) -> Page {
        self.stack.clear();
        self.page.styles = self.styles.into_vec();
        self.page.attrs.sort_by_key(|a| a.node);
        // Top-level units: the body's block children.
        let body = 1u32;
        let mut top = Vec::new();
        let mut c = self.page.nodes[body as usize].first_child;
        while c != NONE {
            let sid = self.page.nodes[c as usize].style as usize;
            if self.page.styles[sid].display.is_block_level() {
                top.push(c);
            }
            c = self.page.nodes[c as usize].next_sibling;
        }
        self.page.top_level = top;
        self.page
    }

    /// Finishes and validates in memory.
    pub fn build(self) -> PageFile {
        let page = self.finish();
        let bytes = page_format::encode(&page, 0).expect("encode");
        PageFile::from_bytes(bytes).expect("valid page")
    }

    /// Finishes and writes to `path`.
    pub fn write(self, path: &std::path::Path) -> Result<(), page_format::PageError> {
        let page = self.finish();
        page_format::write_to_path(&page, 0, path)
    }
}

/// A demonstration page exercising headings, paragraphs, links, lists,
/// `<pre>`, inline styling, borders, backgrounds, an inline-block, an
/// image and a placeholder.
pub fn demo_page(viewport_width: u16) -> PageBuilder {
    let base = inline(|_| {});
    let mut b = PageBuilder::new(
        "https://example.test/demo",
        viewport_width,
        PageBuilder::ua_body(),
    );
    b.title("Lean Browser demo");

    let h1 = block(|s| {
        s.font_size = 32.0;
        s.font_weight = css_subset::FontWeight::Bold;
        margins(s, 21.44, 0.0, 21.44, 0.0);
    });
    b.open(NodeKind::H1, Role::Heading1, h1);
    b.text("Lean Browser renderer", h1);
    b.close();

    let p = block(|s| margins(s, 16.0, 0.0, 16.0, 0.0));
    b.open(NodeKind::P, Role::Paragraph, p);
    b.text(
        "This paragraph is laid out by the renderer's own block and inline formatting: \
         UAX #14 line breaking, collapsed white space, ",
        base,
    );
    let em = inline(|s| s.font_style = css_subset::FontStyle::Italic);
    b.open(NodeKind::Em, Role::Emphasis, em);
    b.text("italic", em);
    b.close();
    b.text(" and ", base);
    let strong = inline(|s| s.font_weight = css_subset::FontWeight::Bold);
    b.open(NodeKind::Strong, Role::Strong, strong);
    b.text("bold", strong);
    b.close();
    b.text(" runs, and a ", base);
    let link = inline(|s| {
        s.color = Rgba::rgb(0, 0, 238);
        s.text_decoration = TextDecoration::UNDERLINE;
    });
    b.link("https://example.test/next", "link with an underline", link);
    b.text(
        ". Text wraps at the viewport width and the glyph cache is capped by bytes.",
        base,
    );
    b.close();

    let h2 = block(|s| {
        s.font_size = 24.0;
        s.font_weight = css_subset::FontWeight::Bold;
        margins(s, 19.92, 0.0, 19.92, 0.0);
    });
    b.open(NodeKind::H2, Role::Heading2, h2);
    b.text("Lists, code and boxes", h2);
    b.close();

    let ul = block(|s| {
        margins(s, 16.0, 0.0, 16.0, 0.0);
        s.padding[3] = Length::px(40.0);
    });
    b.open(NodeKind::Ul, Role::List, ul);
    for (i, item) in ["first item", "second item with a longer label", "third"]
        .iter()
        .enumerate()
    {
        let li = style(|s| {
            s.display = Display::ListItem;
            s.list_style_type = if i == 1 {
                ListStyleType::Circle
            } else {
                ListStyleType::Disc
            };
        });
        b.open(NodeKind::Li, Role::ListItem, li);
        b.text(item, base);
        b.close();
    }
    b.close();

    let ol = block(|s| {
        margins(s, 16.0, 0.0, 16.0, 0.0);
        s.padding[3] = Length::px(40.0);
    });
    b.open(NodeKind::Ol, Role::List, ol);
    for item in ["one", "two"] {
        let li = style(|s| {
            s.display = Display::ListItem;
            s.list_style_type = ListStyleType::Decimal;
        });
        b.open(NodeKind::Li, Role::ListItem, li);
        b.text(item, base);
        b.close();
    }
    b.close();

    let pre = block(|s| {
        s.font_family = css_subset::FontFamily::Mono;
        s.font_size = 13.0;
        s.white_space = WhiteSpace::Pre;
        s.background_color = Rgba::rgb(0xF4, 0xF4, 0xF4);
        paddings(s, 8.0);
        border(s, 1.0, Rgba::rgb(0xCC, 0xCC, 0xCC));
        s.border_radius = [Length::px(4.0); 4];
        margins(s, 16.0, 0.0, 16.0, 0.0);
    });
    let code = inline(|s| {
        s.font_family = css_subset::FontFamily::Mono;
        s.font_size = 13.0;
        s.white_space = WhiteSpace::Pre;
    });
    b.open(NodeKind::Pre, Role::Code, pre);
    b.text(
        "fn main() {\n\tprintln!(\"hello, strip buffer\");\n}\n",
        code,
    );
    b.close();

    let boxed = block(|s| {
        s.background_color = Rgba::rgb(0xE8, 0xF0, 0xFE);
        border(s, 2.0, Rgba::rgb(0x1A, 0x73, 0xE8));
        paddings(s, 12.0);
        s.width = Length::px(360.0);
        s.margin = [
            Length::px(16.0),
            Length::AUTO,
            Length::px(16.0),
            Length::AUTO,
        ];
        s.text_align = TextAlign::Center;
    });
    b.open(NodeKind::Div, Role::Generic, boxed);
    b.text("A centered, bordered box with centered text and an ", base);
    let ib = style(|s| {
        s.display = Display::InlineBlock;
        s.background_color = Rgba::rgb(0xFF, 0xE0, 0x80);
        paddings(s, 4.0);
        border(s, 1.0, Rgba::rgb(0x80, 0x60, 0x00));
    });
    b.open(NodeKind::Span, Role::Generic, ib);
    b.text("inline-block", base);
    b.close();
    b.text(" inside it.", base);
    b.close();

    // A 2x2 PNG scaled up, and an unsupported image as a placeholder.
    let png = {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, 2, 2);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().unwrap();
        w.write_image_data(&[
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ])
        .unwrap();
        drop(w);
        out
    };
    b.open(NodeKind::P, Role::Paragraph, p);
    b.text("Images: ", base);
    let img = inline(|s| {
        s.width = Length::px(48.0);
        s.height = Length::px(48.0);
        s.vertical_align = css_subset::VerticalAlign::Middle;
    });
    b.image(img, &png, ImageFormat::Png, 2, 2);
    b.text(" a PNG scaled at paint time, and ", base);
    let unsupported = inline(|s| s.vertical_align = css_subset::VerticalAlign::Middle);
    b.image(unsupported, b"GIF89a", ImageFormat::Unsupported, 64, 32);
    b.text(
        " a placeholder for a format the renderer does not decode.",
        base,
    );
    b.close();

    let footer = block(|s| {
        s.font_size = 12.0;
        s.color = Rgba::rgb(0x66, 0x66, 0x66);
        s.text_align = TextAlign::Right;
        margins(s, 32.0, 0.0, 0.0, 0.0);
        s.border_width[0] = 1.0;
        s.border_style[0] = BorderStyle::Solid;
        s.border_color[0] = Rgba::rgb(0xDD, 0xDD, 0xDD);
        s.padding[0] = Length::px(8.0);
    });
    b.open(NodeKind::Footer, Role::ContentInfo, footer);
    b.text("right-aligned footer text", footer);
    b.close();
    b
}
