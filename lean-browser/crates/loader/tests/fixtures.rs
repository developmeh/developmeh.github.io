//! End-to-end loads of the HTML fixtures under `tests/fixtures/` through
//! the public pipeline (`file://` fetch, cascade, compression, page file
//! validation).

use std::path::PathBuf;

use css_subset::{Display, FontFamily, FontWeight, NodeFlags, WhiteSpace};
use loader::fetch::url_from_arg;
use loader::{load, LoadOptions, LoadResult};
use page_format::{ArchivedPage, AttrKey, ImageFormat, NodeKind, Role};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

/// Loads a fixture, encodes it and validates the bytes as the renderer would.
fn load_fixture(rel: &str) -> (LoadResult, page_format::AlignedVec) {
    let url = url_from_arg(fixture(rel).to_str().unwrap()).unwrap();
    let opts = LoadOptions {
        check: true,
        ..LoadOptions::default()
    };
    let r = load(&url, &opts).expect("load fixture");
    let bytes = page_format::encode(&r.page, r.flags).expect("encode");
    page_format::validate(&bytes).expect("validate");
    (r, bytes)
}

fn archived(bytes: &page_format::AlignedVec) -> &ArchivedPage {
    page_format::validate(bytes).unwrap().1
}

fn text_of(page: &ArchivedPage, i: usize) -> String {
    page.text_of(&page.nodes[i]).to_string()
}

fn all_text(page: &ArchivedPage) -> String {
    let mut s = String::new();
    for n in page.nodes.iter() {
        if matches!(n.kind, NodeKind::Text | NodeKind::Pseudo | NodeKind::Marker) {
            s.push_str(page.text_of(n));
        }
    }
    s
}

#[test]
fn blog_post() {
    let (r, bytes) = load_fixture("blog/post.html");
    let page = archived(&bytes);
    assert_eq!(page.title, "A post with code · developmeh");
    assert!(page.final_url.starts_with("file://"));
    assert_eq!(page.viewport_width.to_native(), 1280);
    // Breakpoints from the inline sheet and the imported theme.
    let bps: Vec<u16> = page.breakpoints.iter().map(|b| b.to_native()).collect();
    assert_eq!(bps, vec![720, 900, 1100]);

    // Compression: the M2 exit criterion is >= 40% node reduction on the
    // corpus median; this small fixture has few transparent wrappers, so
    // it only asserts a floor.
    let total_before = r.frozen_nodes;
    let after = page.nodes.len();
    assert!(
        after * 100 <= total_before * 75,
        "only {total_before} -> {after} nodes"
    );
    assert_eq!(r.stats.len(), 7);

    // Head, script, print stylesheet and the noscript-hidden paragraph.
    let text = all_text(page);
    assert!(!text.contains("dataLayer"));
    assert!(!text.contains("Enable JS"));
    // Code block keeps whitespace and its syntax spans.
    let pre = page
        .nodes
        .iter()
        .position(|n| n.kind == NodeKind::Pre)
        .expect("pre");
    assert!(page.nodes[pre].node_flags().contains(NodeFlags::PRE));
    let pre_style = &page.styles[page.nodes[pre].style.to_native() as usize];
    assert_eq!(pre_style.white_space, WhiteSpace::Pre);
    assert_eq!(pre_style.font_family, FontFamily::Mono);
    let keyword = page
        .nodes
        .iter()
        .enumerate()
        .find(|(_, n)| n.kind == NodeKind::Text && page.text_of(n) == "fn")
        .map(|(i, _)| i)
        .expect("keyword run");
    let ks = &page.styles[page.nodes[keyword].style.to_native() as usize];
    assert_eq!(ks.font_weight, FontWeight::Bold);
    assert_eq!(ks.color, css_subset::Rgba::rgb(0xd7, 0x3a, 0x49));
    assert!(text.contains("    println!(\"hi\");\n}\n"));

    // Wrapper collapse merged "pass three" into the paragraph text run.
    assert!(text.contains("collapsed away by pass three."), "{text}");
    // The <span class=author> had no box props: collapsed; <em> keeps italic.
    let em = page
        .nodes
        .iter()
        .position(|n| n.kind == NodeKind::Em)
        .expect("em");
    assert_eq!(
        page.styles[page.nodes[em].style.to_native() as usize].font_style,
        css_subset::FontStyle::Italic
    );

    // Links: resolved hrefs, LINK flag, name = text.
    let links: Vec<usize> = page
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.node_flags().contains(NodeFlags::LINK))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(links.len(), 6);
    let hrefs: Vec<String> = links
        .iter()
        .map(|&i| page.attr(i as u32, AttrKey::Href).unwrap().to_string())
        .collect();
    assert!(hrefs[0].ends_with("/"));
    assert!(hrefs[1].ends_with("/devex/"));
    assert!(
        hrefs[2].ends_with("/tests/fixtures/blog/../tags/index.html")
            || hrefs[2].ends_with("/tests/fixtures/tags/index.html")
    );
    assert!(hrefs[3].ends_with("#title"));
    assert_eq!(hrefs[4], "https://example.test/");
    assert_eq!(hrefs[5], "mailto:x@example.test");
    assert_eq!(page.name_of(&page.nodes[links[0]]), "Home");
    assert_eq!(page.nodes[links[0]].role, Role::Link);
    // A name equal to the node text aliases it.
    let home_text = page.children(links[0] as u32).next().unwrap() as usize;
    assert_eq!(
        page.nodes[links[0]].name_off,
        page.nodes[home_text].text_off
    );

    // Landmarks and headings survive.
    let roles: Vec<Role> = page.nodes.iter().map(|n| n.role).collect();
    for r in [
        Role::Navigation,
        Role::Banner,
        Role::Main,
        Role::Article,
        Role::Heading1,
        Role::Heading2,
        Role::Complementary,
        Role::ContentInfo,
        Role::List,
        Role::ListItem,
        Role::Figure,
        Role::Caption,
        Role::Table,
        Role::ColumnHeader,
        Role::Cell,
        Role::Blockquote,
        Role::Code,
    ] {
        assert!(roles.contains(&r), "missing role {r:?}");
    }
    let nav = roles.iter().position(|&r| r == Role::Navigation).unwrap();
    assert_eq!(page.name_of(&page.nodes[nav]), "Main");

    // Images: the PNG is stored, the missing GIF is a placeholder of its
    // attribute size.
    assert_eq!(page.images.len(), 2);
    let png = &page.images[0];
    assert_eq!(png.format, ImageFormat::Png);
    assert_eq!((png.width.to_native(), png.height.to_native()), (8, 8));
    assert!(png.blob_len.to_native() > 0);
    let gif = &page.images[1];
    assert_eq!(gif.format, ImageFormat::Unsupported);
    assert_eq!((gif.width.to_native(), gif.height.to_native()), (20, 10));
    assert_eq!(gif.blob_len.to_native(), 0);
    assert_eq!(
        page.nodes[gif.node.to_native() as usize].role,
        Role::Presentation
    );

    // List markers: bullets and the <ol start=3> ordinals.
    let markers: Vec<String> = page
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == NodeKind::Marker)
        .map(|(i, _)| text_of(page, i))
        .collect();
    // The trailing bullet is the <summary> (UA `display: list-item`).
    assert_eq!(markers, vec!["• ", "• ", "• ", "◦ ", "3. ", "4. ", "• "]);

    // Grid container recorded with its tracks (M4 layout, M2 data).
    let grid = page
        .styles
        .iter()
        .find(|s| s.display == Display::Grid)
        .expect("grid style");
    assert_eq!(grid.grid_template_columns.len.to_native(), 2);
    assert_eq!(page.tracks.len(), 2);

    // Colspan kept, class/style attributes gone.
    assert!(page.attrs.iter().any(|a| a.key == AttrKey::Colspan));

    // Every top-level node is a block-level child of body.
    let body = page
        .nodes
        .iter()
        .position(|n| n.kind == NodeKind::Body)
        .unwrap();
    assert!(!page.top_level.is_empty());
    for tl in page.top_level.iter() {
        let n = &page.nodes[tl.to_native() as usize];
        assert_eq!(n.parent.to_native() as usize, body);
        assert!(n.node_flags().contains(NodeFlags::BLOCK));
    }
    assert_eq!(r.flags, 0);
}

#[test]
fn forms() {
    let (_r, bytes) = load_fixture("forms.html");
    let page = archived(&bytes);
    assert_eq!(page.forms.len(), 2);
    let signup = &page.forms[0];
    assert_eq!(signup.method, page_format::FormMethod::Post);
    let action = std::str::from_utf8(
        &page.text[signup.action_off.to_native() as usize
            ..(signup.action_off.to_native() + signup.action_len.to_native()) as usize],
    )
    .unwrap();
    assert!(action.ends_with("/tests/fixtures/submit.php"));
    // name, email, news, plan x2, tz, bio: 7 fields. The disabled input and
    // the input inside a display:none div are not fields; the hidden input
    // is dropped by pass 1 (UA `display: none`, see STATUS.md); buttons
    // have no name.
    let fields: Vec<usize> = page.fields[signup.first_field.to_native() as usize..]
        [..signup.field_count.to_native() as usize]
        .iter()
        .map(|f| f.to_native() as usize)
        .collect();
    let names: Vec<String> = fields
        .iter()
        .map(|&i| page.attr(i as u32, AttrKey::Name).unwrap().to_string())
        .collect();
    assert_eq!(
        names,
        vec!["name", "email", "news", "plan", "plan", "tz", "bio"]
    );
    // Roles and names.
    let role_name = |name: &str| {
        let i = fields[names.iter().position(|n| n == name).unwrap()];
        (page.nodes[i].role, page.name_of(&page.nodes[i]).to_string())
    };
    assert_eq!(role_name("name"), (Role::TextField, "Name".to_string()));
    assert_eq!(role_name("email"), (Role::TextField, "Email".to_string()));
    assert_eq!(role_name("news"), (Role::Checkbox, "News".to_string()));
    assert_eq!(role_name("plan").0, Role::Radio);
    assert_eq!(role_name("tz").0, Role::ComboBox);
    // Checked and value attributes are kept.
    let news = fields[2];
    assert!(page.attr(news as u32, AttrKey::Checked).is_some());
    let free = fields[3];
    assert_eq!(page.attr(free as u32, AttrKey::Value), Some("free"));
    // Buttons.
    let buttons: Vec<&page_format::ArchivedNode> = page
        .nodes
        .iter()
        .filter(|n| n.role == Role::Button)
        .collect();
    assert_eq!(buttons.len(), 3);
    assert_eq!(page.name_of(buttons[0]), "Create");
    assert_eq!(page.name_of(buttons[1]), "Help");
    assert_eq!(page.name_of(buttons[2]), "Go");
    // The second form is GET with one field.
    assert_eq!(page.forms[1].method, page_format::FormMethod::Get);
    assert_eq!(page.forms[1].field_count.to_native(), 1);
    // Focusable controls.
    assert!(page.nodes[fields[0]]
        .node_flags()
        .contains(NodeFlags::FOCUSABLE));
}
