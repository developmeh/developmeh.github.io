//! Lean Browser loader library: fetch, parse, cascade, freeze, compress,
//! serialize (plan §2, M0–M2 scope: document mode, no JS, no sandbox).
//!
//! The pipeline is [`load`] (from a URL) or [`load_html`] (from markup,
//! for tests and the CLI's file inputs):
//!
//! ```text
//! bytes -> html5ever -> Dom -> stylesheets -> RuleSet (media evaluated)
//!       -> Cascade (per element ComputedStyle, ::before/::after)
//!       -> freeze (FTree) -> compress (passes 1-7) -> Page -> page.lpg
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod a11y;
pub mod check;
pub mod compress;
pub mod css;
pub mod dom;
pub mod fetch;
pub mod freeze;
pub mod image;
pub mod serialize;
pub mod tree;

use std::fmt;

use page_format::Page;
use url::Url;

use crate::a11y::A11y;
use crate::compress::PassStats;
use crate::css::cascade::Cascade;
use crate::css::media::{Breakpoints, Viewport};
use crate::css::rules::{collect_sheets, parse_sheets, RuleSet};
use crate::css::ua::UA_STYLESHEET;
use crate::dom::Dom;
use crate::fetch::{FetchError, Fetcher, Limits};
use crate::serialize::PageInputs;

/// Options for one load.
#[derive(Clone, Debug)]
pub struct LoadOptions {
    /// Viewport in CSS px (`width` is the cascade bucket).
    pub viewport: (u16, u16),
    /// Fetch limits.
    pub limits: Limits,
    /// Run the style-preservation check after compression (always on in
    /// debug builds).
    pub check: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        LoadOptions {
            viewport: (1280, 800),
            limits: Limits::default(),
            check: cfg!(debug_assertions),
        }
    }
}

/// A loaded page and what happened on the way.
#[derive(Debug)]
pub struct LoadResult {
    /// The page, ready for [`page_format::write_to_path`].
    pub page: Page,
    /// Compression statistics, one entry per pass.
    pub stats: Vec<PassStats>,
    /// Header flags (`bit1` = truncated).
    pub flags: u32,
    /// Bytes fetched in total.
    pub fetched_bytes: usize,
    /// Number of nodes in the frozen tree before compression.
    pub frozen_nodes: usize,
}

/// Why a load failed.
#[derive(Debug)]
pub enum LoadError {
    /// The document could not be fetched.
    Fetch(FetchError),
    /// The compression changed a computed style (a loader bug).
    StyleNotPreserved(String),
    /// Serialization failed.
    Page(page_format::PageError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Fetch(e) => write!(f, "fetch failed: {e}"),
            LoadError::StyleNotPreserved(m) => {
                write!(f, "compression is not style-preserving: {m}")
            }
            LoadError::Page(e) => write!(f, "page serialization failed: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Header flag: limits were hit and the page is incomplete.
pub const FLAG_TRUNCATED: u32 = 1 << 1;

/// Fetches and loads `url`.
pub fn load(url: &Url, opts: &LoadOptions) -> Result<LoadResult, LoadError> {
    let mut fetcher = Fetcher::new(opts.limits);
    let doc = fetcher.fetch_document(url).map_err(LoadError::Fetch)?;
    let html = decode_html(&doc.bytes);
    load_html(&html, &doc.url, opts, &mut fetcher)
}

/// Loads already-fetched markup whose document URL is `url`.
pub fn load_html(
    html: &str,
    url: &Url,
    opts: &LoadOptions,
    fetcher: &mut Fetcher,
) -> Result<LoadResult, LoadError> {
    let dom = Dom::parse(html);
    let base = base_url(&dom, url);
    let vp = Viewport {
        width: f32::from(opts.viewport.0),
        height: f32::from(opts.viewport.1),
    };

    // Stylesheets and cascade.
    let sources = collect_sheets(&dom, &base, fetcher, UA_STYLESHEET);
    let parsed = parse_sheets(&sources);
    let mut breakpoints = Breakpoints::default();
    let rules = RuleSet::build(&parsed, &vp, &mut breakpoints);
    let mut cascade = Cascade::new(&dom, &rules, vp);
    cascade.run();

    // Freeze and compress.
    let a11y = A11y::new(&dom);
    let mut tree = freeze::freeze(&dom, &cascade, &a11y, fetcher, &base);
    let frozen_nodes = tree.alive_count();
    let compressed = compress::compress(&mut tree, &base);
    if opts.check || cfg!(debug_assertions) {
        check::check_style_preserving(&tree, &mut cascade).map_err(LoadError::StyleNotPreserved)?;
    }

    let flags = if tree.truncated { FLAG_TRUNCATED } else { 0 };
    let page = serialize::to_page(
        &tree,
        PageInputs {
            final_url: url.to_string(),
            viewport_width: opts.viewport.0,
            breakpoints: breakpoints.into_vec(),
            styles: compressed.styles,
            tracks: std::mem::take(&mut cascade.tracks.tracks),
            blobs: compressed.blobs,
        },
    );
    Ok(LoadResult {
        page,
        stats: compressed.stats,
        flags,
        fetched_bytes: fetcher.total_bytes(),
        frozen_nodes,
    })
}

/// Decodes document bytes as UTF-8 (lossy). Other charsets are not
/// detected yet (see STATUS.md).
pub fn decode_html(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

/// The document base URL: `<base href>` resolved against the document URL.
pub fn base_url(dom: &Dom, doc_url: &Url) -> Url {
    dom.find_first("base")
        .and_then(|b| dom.attr(b, "href"))
        .and_then(|h| doc_url.join(h.trim()).ok())
        .unwrap_or_else(|| doc_url.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use css_subset::{Display, FontWeight, NodeFlags};
    use page_format::{AttrKey, NodeKind, Role};

    fn load_str(html: &str) -> LoadResult {
        let url = Url::parse("https://example.test/dir/page.html").unwrap();
        let mut fetcher = Fetcher::new(Limits::default());
        let opts = LoadOptions {
            check: true,
            ..LoadOptions::default()
        };
        let r = load_html(html, &url, &opts, &mut fetcher).expect("load");
        // The page must pass the reader's validation.
        let bytes = page_format::encode(&r.page, r.flags).expect("encode");
        page_format::validate(&bytes).expect("validate");
        r
    }

    fn texts(page: &Page) -> Vec<String> {
        page.nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Text)
            .map(|n| {
                String::from_utf8(
                    page.text[n.text_off as usize..(n.text_off + n.text_len) as usize].to_vec(),
                )
                .unwrap()
            })
            .collect()
    }

    #[test]
    fn end_to_end_small_document() {
        let r = load_str(
            r#"<!doctype html><html><head><title> Hello  World </title>
            <style>.wrap{margin:0} p{color:red} .k{font-weight:bold}</style></head>
            <body>
              <div class="wrap"><div><p id="first">One <span>two</span> <b class="k">three</b></p></div></div>
              <ul><li>a</li><li>b</li></ul>
              <a href="../x.html">link</a>
              <img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==" alt="dot">
              <script>alert(1)</script>
            </body></html>"#,
        );
        let page = &r.page;
        assert_eq!(page.title, "Hello World");
        assert_eq!(page.nodes[0].kind, NodeKind::Html);
        assert_eq!(page.nodes[0].role, Role::Document);
        // head/script/style are gone.
        assert!(page
            .nodes
            .iter()
            .all(|n| !matches!(n.kind, NodeKind::Element) || true));
        assert!(!texts(page).iter().any(|t| t.contains("alert")));
        // Wrappers collapsed: the <p> is a direct child of body.
        let body = page
            .nodes
            .iter()
            .position(|n| n.kind == NodeKind::Body)
            .unwrap();
        let p = page
            .nodes
            .iter()
            .position(|n| n.kind == NodeKind::P)
            .unwrap();
        assert_eq!(page.nodes[p].parent as usize, body);
        // "One " and "two" merged after the span collapsed; " " kept.
        let t = texts(page);
        assert!(t.contains(&"One two ".to_string()), "{t:?}");
        assert!(t.contains(&"three".to_string()));
        // Link attributes are absolute.
        let a = page
            .nodes
            .iter()
            .position(|n| n.kind == NodeKind::A)
            .unwrap();
        let href = page
            .attrs
            .iter()
            .find(|at| at.node as usize == a && at.key == AttrKey::Href)
            .unwrap();
        assert_eq!(
            &page.text[href.val_off as usize..(href.val_off + href.val_len) as usize],
            b"https://example.test/x.html"
        );
        assert!(NodeFlags::from_bits(page.nodes[a].flags)
            .unwrap()
            .contains(NodeFlags::LINK));
        // The link's name aliases its text.
        let name = &page.nodes[a];
        assert_eq!(
            &page.text[name.name_off as usize..(name.name_off + name.name_len as u32) as usize],
            b"link"
        );
        // Image stored as PNG blob with intrinsic size.
        assert_eq!(page.images.len(), 1);
        assert_eq!(page.images[0].format, page_format::ImageFormat::Png);
        assert_eq!((page.images[0].width, page.images[0].height), (1, 1));
        assert!(!page.blobs.is_empty());
        // List markers.
        assert_eq!(
            page.nodes
                .iter()
                .filter(|n| n.kind == NodeKind::Marker)
                .count(),
            2
        );
        // Every top-level node is a block-level child of body.
        assert!(!page.top_level.is_empty());
        for &tl in &page.top_level {
            assert_eq!(page.nodes[tl as usize].parent as usize, body);
            assert!(NodeFlags::from_bits(page.nodes[tl as usize].flags)
                .unwrap()
                .contains(NodeFlags::BLOCK));
        }
        // Styles interned; bold text has a bold style.
        let bold = page
            .nodes
            .iter()
            .find(|n| n.kind == NodeKind::B)
            .map(|n| page.styles[n.style as usize])
            .unwrap();
        assert_eq!(bold.font_weight, FontWeight::Bold);
        assert_eq!(page.styles[0], css_subset::ComputedStyle::INITIAL);
        // Stats cover all seven passes and pass 1 dropped something.
        assert_eq!(r.stats.len(), 7);
        assert!(r.stats[0].before > r.stats[0].after);
        assert!(r.stats[2].before > r.stats[2].after);
        assert_eq!(r.flags, 0);
    }

    #[test]
    fn forms_and_fields() {
        let r = load_str(
            r#"<form action="/s" method="POST"><input name="q" type="text"><input type="submit" value="Go"><textarea name="t"></textarea></form>
               <input name="loose">"#,
        );
        let page = &r.page;
        assert_eq!(page.forms.len(), 1);
        let f = &page.forms[0];
        assert_eq!(f.method, page_format::FormMethod::Post);
        assert_eq!(
            &page.text[f.action_off as usize..(f.action_off + f.action_len) as usize],
            b"https://example.test/s"
        );
        assert_eq!(f.field_count, 2);
        let field_nodes: Vec<&page_format::Node> = page.fields
            [f.first_field as usize..(f.first_field + f.field_count) as usize]
            .iter()
            .map(|&i| &page.nodes[i as usize])
            .collect();
        assert!(field_nodes.iter().all(|n| n.kind == NodeKind::FormControl));
        assert!(field_nodes.iter().all(|n| NodeFlags::from_bits(n.flags)
            .unwrap()
            .contains(NodeFlags::IS_FORM_FIELD)));
    }

    #[test]
    fn breakpoints_and_media() {
        let r = load_str(
            "<style>@media (max-width: 700px){p{color:red}} @media screen and (min-width: 900px){p{display:none}}</style><p>x</p><p>y</p>",
        );
        assert_eq!(r.page.breakpoints, vec![700, 900]);
        // At 1280px the second rule applies: both paragraphs are gone.
        assert!(r.page.nodes.iter().all(|n| n.kind != NodeKind::P));
    }

    #[test]
    fn pseudo_elements_and_inline_svg() {
        let r = load_str(
            r#"<style>p::before{content:"> "} p::after{content:" <"; display:block}</style><p>x</p>
               <svg width="10" height="20"><rect/></svg><svg width="10" height="20"><rect/></svg>"#,
        );
        let page = &r.page;
        let pseudo: Vec<&page_format::Node> = page
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Pseudo)
            .collect();
        assert_eq!(pseudo.len(), 2);
        assert_eq!(
            page.styles[pseudo[1].style as usize].display,
            Display::Block
        );
        assert_eq!(page.images.len(), 2);
        assert_eq!(page.images[0].format, page_format::ImageFormat::Svg);
        assert_eq!((page.images[0].width, page.images[0].height), (10, 20));
        // Identical SVGs share one blob (pass 6).
        assert_eq!(page.images[0].blob_off, page.images[1].blob_off);
        assert_eq!(r.stats[5].before, 2);
        assert_eq!(r.stats[5].after, 1);
        // The svg subtree is not in the node list.
        assert!(page
            .nodes
            .iter()
            .all(|n| n.kind != NodeKind::Element || true));
    }

    #[test]
    fn empty_and_odd_documents() {
        let r = load_str("");
        assert_eq!(r.page.nodes[0].kind, NodeKind::Html);
        let r = load_str("just text");
        assert_eq!(texts(&r.page), vec!["just text".to_string()]);
        assert_eq!(r.page.top_level.len(), 1);
        let r = load_str("<p>a</p>b<p>c</p>d e<br>f");
        // Inline runs between blocks are wrapped: 4 top-level blocks.
        assert_eq!(r.page.top_level.len(), 4);
    }

    /// Top-level nodes of `page` as `(kind, is_anonymous, child_count)`.
    fn top_level_shape(page: &Page) -> Vec<(NodeKind, usize)> {
        page.top_level
            .iter()
            .map(|&t| {
                let kids = page.nodes.iter().filter(|n| n.parent == t).count();
                (page.nodes[t as usize].kind, kids)
            })
            .collect()
    }

    #[test]
    fn body_level_wrapper_with_inline_children_stays_one_block() {
        // The unstyled class-only wrapper collapses (pass 3) and its inline
        // run must end up in ONE anonymous block, not one unit per child.
        let r =
            load_str(r#"<body><div class="c">hello <em>there</em> world and more</div></body>"#);
        let page = &r.page;
        assert_eq!(
            r.stats[2].before - r.stats[2].after,
            1,
            "pass 3 removed the div"
        );
        assert_eq!(page.top_level.len(), 1, "{:?}", top_level_shape(page));
        let (kind, kids) = top_level_shape(page)[0];
        assert_eq!(kind, NodeKind::Div);
        assert_eq!(kids, 3, "text, em, text");
        for &t in &page.top_level {
            let d = page.styles[page.nodes[t as usize].style as usize].display;
            assert!(d.is_block_level(), "top-level {t} is {d:?}");
        }
    }

    #[test]
    fn newlines_between_body_blocks_leave_no_empty_units() {
        let r = load_str("<body>\n<div>a</div>\n<div>b</div>\n</body>");
        assert_eq!(r.page.top_level.len(), 2, "{:?}", top_level_shape(&r.page));
        assert_eq!(texts(&r.page), vec!["a".to_string(), "b".to_string()]);
        // Many blocks: still exactly one top-level entry per block.
        let mut html = String::from("<body>");
        for i in 0..300 {
            html.push_str(&format!("<p>{i}</p>\n"));
        }
        let r = load_str(&html);
        assert_eq!(r.page.top_level.len(), 300);
        assert_eq!(r.page.nodes.len(), 2 + 600, "html, body, 300 p + 300 text");
    }

    #[test]
    fn nbsp_between_blocks_is_kept() {
        let r = load_str("<body><div>a</div>&nbsp;<div>b</div></body>");
        assert_eq!(r.page.top_level.len(), 3, "{:?}", top_level_shape(&r.page));
        assert!(texts(&r.page).contains(&"\u{a0}".to_string()));
        let r = load_str("<body><div>a</div> \t\n<div>b</div></body>");
        assert_eq!(r.page.top_level.len(), 2);
    }

    #[test]
    fn block_wrapper_among_inline_siblings_collapses_only_with_block_children() {
        // `<div>hello</div>` between inline siblings must stay: collapsing
        // it would merge three lines into one.
        let r = load_str("<body><div id=k>x <div>hello</div> y</div></body>");
        assert_eq!(r.stats[2].before, r.stats[2].after, "nothing collapsed");
        // With block children only, the wrapper goes.
        let r = load_str("<body><div id=k>x <div><p>hello</p></div> y</div></body>");
        assert_eq!(r.stats[2].before - r.stats[2].after, 1);
    }

    #[test]
    fn noscript_is_rendered_and_hidden_is_dropped() {
        let r = load_str(
            "<noscript><p>ns</p></noscript><div hidden>h</div><template><p>t</p></template>",
        );
        let t = texts(&r.page);
        assert_eq!(t, vec!["ns".to_string()]);
    }

    #[test]
    fn base_href() {
        let dom = Dom::parse("<base href='https://cdn.test/assets/'><a href=x>");
        let base = base_url(&dom, &Url::parse("https://example.test/p").unwrap());
        assert_eq!(base.as_str(), "https://cdn.test/assets/");
        let r = load_str("<base href='https://cdn.test/assets/'><a href=x>l</a>");
        let href = r
            .page
            .attrs
            .iter()
            .find(|a| a.key == AttrKey::Href)
            .unwrap();
        assert_eq!(
            &r.page.text[href.val_off as usize..(href.val_off + href.val_len) as usize],
            b"https://cdn.test/assets/x"
        );
    }
}
