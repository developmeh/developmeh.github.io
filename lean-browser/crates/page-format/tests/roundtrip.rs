//! Round trip, validator rejection and bit-flip tests for the page file.

use page_format::css_subset::{ComputedStyle, Display, GridTrack, NodeFlags, TrackListRef};
use page_format::{
    encode, validate, Attr, AttrKey, Form, FormMethod, Header, ImageFormat, ImageRef, Node,
    NodeKind, Page, PageError, PageFile, Role, HEADER_LEN, NONE,
};

/// A small but complete page:
///
/// ```text
/// 0 html
///   1 body
///     2 p            (top-level block, has id)
///       3 text "Hello, "
///       4 a href     (link)
///         5 text "world"
///     6 img          (image blob)
///     7 form
///       8 input      (form field)
/// ```
fn sample_page() -> Page {
    let mut page = Page::empty("https://example.test/", 1280);
    page.title = "Sample".into();
    let mut block = ComputedStyle::INITIAL;
    block.display = Display::Block;
    page.styles.push(block); // style 1
    let mut grid = ComputedStyle::INITIAL;
    grid.display = Display::Grid;
    grid.grid_template_columns = TrackListRef { off: 0, len: 2 };
    page.styles.push(grid); // style 2
    page.tracks = vec![GridTrack::default(), GridTrack::default()];

    let (hello_off, hello_len) = page.push_text("Hello, ");
    let (world_off, world_len) = page.push_text("world");
    let (href_off, href_len) = page.push_text("https://example.test/next");
    let (id_off, id_len) = page.push_text("intro");
    let (alt_off, alt_len) = page.push_text("a picture");
    let (action_off, action_len) = page.push_text("https://example.test/submit");

    let mut nodes = vec![
        Node::new(NodeKind::Html, Role::Document, 1, NONE),
        Node::new(NodeKind::Body, Role::Generic, 1, 0),
        Node::new(NodeKind::P, Role::Paragraph, 1, 1),
        Node::new(NodeKind::Text, Role::StaticText, 0, 2),
        Node::new(NodeKind::A, Role::Link, 0, 2),
        Node::new(NodeKind::Text, Role::StaticText, 0, 4),
        Node::new(NodeKind::Image, Role::Img, 2, 1),
        Node::new(NodeKind::Form, Role::Form, 1, 1),
        Node::new(NodeKind::FormControl, Role::TextField, 0, 7),
    ];
    nodes[0].first_child = 1;
    nodes[1].first_child = 2;
    nodes[2].first_child = 3;
    nodes[2].next_sibling = 6;
    nodes[2].flags = (NodeFlags::BLOCK | NodeFlags::ANCHOR_TARGET | NodeFlags::HAS_ATTRS).bits();
    nodes[3].text_off = hello_off;
    nodes[3].text_len = hello_len;
    nodes[3].name_off = hello_off;
    nodes[3].name_len = hello_len as u16;
    nodes[3].next_sibling = 4;
    nodes[4].first_child = 5;
    nodes[4].flags = (NodeFlags::LINK | NodeFlags::FOCUSABLE | NodeFlags::HAS_ATTRS).bits();
    nodes[4].name_off = world_off;
    nodes[4].name_len = world_len as u16;
    nodes[5].text_off = world_off;
    nodes[5].text_len = world_len;
    nodes[6].next_sibling = 7;
    nodes[6].flags = (NodeFlags::INLINE_REPLACED | NodeFlags::HAS_ATTRS).bits();
    nodes[6].name_off = alt_off;
    nodes[6].name_len = alt_len as u16;
    nodes[7].first_child = 8;
    nodes[7].flags = NodeFlags::BLOCK.bits();
    nodes[8].flags = (NodeFlags::IS_FORM_FIELD | NodeFlags::FOCUSABLE).bits();
    page.nodes = nodes;

    page.attrs = vec![
        Attr {
            node: 2,
            key: AttrKey::Id,
            val_off: id_off,
            val_len: id_len,
        },
        Attr {
            node: 4,
            key: AttrKey::Href,
            val_off: href_off,
            val_len: href_len,
        },
        Attr {
            node: 6,
            key: AttrKey::Alt,
            val_off: alt_off,
            val_len: alt_len,
        },
    ];
    page.blobs = b"\x89PNG\r\n\x1a\nnot really".to_vec();
    page.images = vec![ImageRef {
        node: 6,
        blob_off: 0,
        blob_len: page.blobs.len() as u32,
        width: 4,
        height: 3,
        format: ImageFormat::Png,
    }];
    page.fields = vec![8];
    page.forms = vec![Form {
        node: 7,
        action_off,
        action_len,
        method: FormMethod::Post,
        first_field: 0,
        field_count: 1,
    }];
    page.top_level = vec![2, 6, 7];
    page.breakpoints = vec![600, 900];
    page
}

fn expect_semantic(page: &Page) -> String {
    let bytes = encode(page, 0).unwrap();
    match validate(&bytes) {
        Err(PageError::Semantic(msg)) => msg,
        other => panic!(
            "expected semantic rejection, got {:?}",
            other.map(|(h, _)| h)
        ),
    }
}

#[test]
fn empty_page_round_trips() {
    let page = Page::empty("about:blank", 800);
    let bytes = encode(&page, 0).unwrap();
    let file = PageFile::from_bytes(bytes).unwrap();
    assert_eq!(file.header().archive_len as usize, file.len() - HEADER_LEN);
    assert_eq!(file.page().nodes.len(), 1);
    assert_eq!(file.page().final_url.as_str(), "about:blank");
}

#[test]
fn sample_page_round_trips() {
    let page = sample_page();
    let bytes = encode(&page, Header::FLAG_JS_RAN).unwrap();
    let file = PageFile::from_bytes(bytes).unwrap();
    assert_eq!(file.header().flags, Header::FLAG_JS_RAN);
    let a = file.page();
    assert_eq!(a.title.as_str(), "Sample");
    assert_eq!(a.nodes.len(), 9);
    assert_eq!(a.text_of(&a.nodes[3]), "Hello, ");
    assert_eq!(a.text_of(&a.nodes[5]), "world");
    assert_eq!(a.name_of(&a.nodes[6]), "a picture");
    assert_eq!(a.attr(4, AttrKey::Href), Some("https://example.test/next"));
    assert_eq!(a.attr(4, AttrKey::Id), None);
    assert_eq!(a.attrs_of(3).len(), 0);
    assert_eq!(a.attrs_of(6).len(), 1);
    assert_eq!(a.children(1).collect::<Vec<_>>(), vec![2, 6, 7]);
    assert_eq!(a.children(2).collect::<Vec<_>>(), vec![3, 4]);
    assert_eq!(a.children(5).count(), 0);
    assert!(a.nodes[4].node_flags().contains(NodeFlags::LINK));
    assert_eq!(a.blob_of(&a.images[0]), page.blobs.as_slice());
    assert!(a.styles[2].display == Display::Grid);
    assert_eq!(a.nodes[2].to_native(), page.nodes[2]);
    // Deserialize the whole thing back and compare.
    let back: Page = rkyv::deserialize::<Page, rkyv::rancor::Error>(a).unwrap();
    assert_eq!(back, page);
}

#[test]
fn writes_and_maps_a_file() {
    let dir = std::env::temp_dir().join(format!("lean-page-format-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("page.lpg");
    page_format::write_to_path(&sample_page(), 0, &path).unwrap();
    let file = PageFile::open(&path).unwrap();
    assert_eq!(file.page().nodes.len(), 9);
    assert!(!dir.join("page.lpg.tmp").exists());
    drop(file);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_header_problems() {
    let good = encode(&sample_page(), 0).unwrap();

    let mut bad = good.clone();
    bad[0] = b'X';
    assert_eq!(validate(&bad).unwrap_err(), PageError::BadMagic);

    let mut bad = good.clone();
    bad[8] = 0xFF; // format_version low byte
    assert!(matches!(
        validate(&bad),
        Err(PageError::UnsupportedVersion { .. })
    ));

    let mut bad = good.clone();
    bad[10] = 0xFF; // min_reader_version
    assert!(matches!(
        validate(&bad),
        Err(PageError::ReaderTooOld { .. })
    ));

    let mut truncated = good.clone();
    truncated.resize(good.len() - 1, 0);
    assert!(matches!(
        validate(&truncated),
        Err(PageError::LengthMismatch { .. })
    ));

    assert_eq!(
        validate(&good[..HEADER_LEN - 1]).unwrap_err(),
        PageError::TooShort
    );
}

/// Every single-bit flip in the archive region is caught by the CRC, and
/// every flip in the header's checked fields is caught by step 1 or 2.
#[test]
fn every_bit_flip_is_rejected() {
    let good = encode(&sample_page(), 0).unwrap();
    assert!(validate(&good).is_ok());
    let mut rejected = 0usize;
    let mut asserted = 0usize;
    for byte in 0..good.len() {
        // Bytes 12..16 (flags) and 28..32 (reserved) are not checked; a
        // flip there is legitimately accepted. Bytes 10..12
        // (min_reader_version) may flip *down* to a version this reader
        // still satisfies, so only their upward flips are asserted.
        let unchecked_header_byte = (12..16).contains(&byte) || (28..32).contains(&byte);
        let min_reader_byte = (10..12).contains(&byte);
        for bit in 0..8 {
            let mut bad = good.clone();
            bad[byte] ^= 1 << bit;
            let result = validate(&bad);
            if unchecked_header_byte {
                assert!(result.is_ok(), "flip at {byte}:{bit} should be accepted");
            } else if min_reader_byte && bad[byte] < good[byte] {
                assert!(
                    result.is_ok(),
                    "flip at {byte}:{bit} lowers min_reader_version"
                );
            } else {
                assert!(result.is_err(), "flip at {byte}:{bit} was accepted");
                rejected += 1;
                asserted += 1;
            }
        }
    }
    assert_eq!(rejected, asserted);
    assert!(rejected >= (good.len() - 12) * 8);
}

/// Bypass the CRC (recompute it after corrupting) so the rkyv structural
/// check and the semantic pass are exercised on every byte of the archive.
/// Nothing may panic, and the outcome must be either a clean rejection or a
/// page that still satisfies every invariant.
#[test]
fn corrupted_archives_never_panic() {
    let good = encode(&sample_page(), 0).unwrap();
    let mut rejected_by_rkyv = 0usize;
    let mut rejected_by_semantics = 0usize;
    for byte in HEADER_LEN..good.len() {
        for bit in 0..8 {
            let mut bad = good.clone();
            bad[byte] ^= 1 << bit;
            let crc = page_format::crc32(&bad[HEADER_LEN..]);
            bad[24..28].copy_from_slice(&crc.to_le_bytes());
            match validate(&bad) {
                Ok((_, page)) => {
                    // Exercise every accessor on the accepted page; none may panic.
                    for (i, node) in page.nodes.iter().enumerate() {
                        let _ = page.text_of(node);
                        let _ = page.name_of(node);
                        let _ = page.attrs_of(i as u32);
                        let _ = page.children(i as u32).count();
                    }
                    for img in page.images.iter() {
                        let _ = page.blob_of(img);
                    }
                }
                Err(PageError::Structural(_)) => rejected_by_rkyv += 1,
                Err(PageError::Semantic(_)) => rejected_by_semantics += 1,
                Err(other) => panic!("unexpected error at {byte}:{bit}: {other}"),
            }
        }
    }
    assert!(rejected_by_rkyv > 0, "rkyv never rejected anything");
    assert!(
        rejected_by_semantics > 0,
        "semantic pass never rejected anything"
    );
}

#[test]
fn semantic_pass_catches_each_invariant() {
    let base = sample_page();

    let mut p = base.clone();
    p.nodes.clear();
    assert!(expect_semantic(&p).contains("no nodes"));

    let mut p = base.clone();
    p.nodes[0].parent = 3;
    assert!(expect_semantic(&p).contains("root node has a parent"));

    let mut p = base.clone();
    p.nodes[5].parent = 99;
    let msg = expect_semantic(&p); // node 4's back-pointer check fires first
    assert!(msg.contains("node 4") || msg.contains("99"), "{msg}");

    let mut p = base.clone();
    p.nodes[5].parent = 6; // parent must precede (node 4's back-pointer check fires first)
    expect_semantic(&p);

    let mut p = base.clone();
    p.nodes[2].first_child = 4; // must be i+1
    assert!(expect_semantic(&p).contains("breaks pre-order"));

    let mut p = base.clone();
    p.nodes[2].next_sibling = 1;
    assert!(expect_semantic(&p).contains("does not follow"));

    let mut p = base.clone();
    p.nodes[2].next_sibling = 5; // different parent
    assert!(expect_semantic(&p).contains("different parent"));

    let mut p = base.clone();
    // Node 6 claims node 3 (a text node with no children) as its parent,
    // with sibling links rewired so no other check fires first.
    p.nodes[6].parent = 3;
    p.nodes[6].next_sibling = NONE;
    p.nodes[2].next_sibling = 7;
    assert!(expect_semantic(&p).contains("not an ancestor"));

    let mut p = base.clone();
    p.nodes[1].style = 7;
    assert!(expect_semantic(&p).contains("style 7 out of range"));

    let mut p = base.clone();
    p.nodes[1].style = 0xFFFF;
    assert!(expect_semantic(&p).contains("style 65535 out of range"));

    let mut p = base.clone();
    p.nodes[1].reserved = 1;
    assert!(expect_semantic(&p).contains("reserved"));

    let mut p = base.clone();
    p.nodes[3].text_len = 10_000;
    assert!(expect_semantic(&p).contains("node text range"));

    let mut p = base.clone();
    p.nodes[3].text_off = u32::MAX;
    p.nodes[3].text_len = 1; // overflow-safe
    assert!(expect_semantic(&p).contains("node text range"));

    let mut p = base.clone();
    p.text = "é".repeat(20).into_bytes();
    p.nodes[3].text_off = 1;
    p.nodes[3].text_len = 2;
    assert!(expect_semantic(&p).contains("splits a UTF-8"));

    let mut p = base.clone();
    p.text[0] = 0xFF;
    assert!(expect_semantic(&p).contains("not UTF-8"));

    let mut p = base.clone();
    p.nodes[4].name_len = 60_000;
    assert!(expect_semantic(&p).contains("node name range"));

    let mut p = base.clone();
    p.styles[2].grid_template_columns = TrackListRef { off: 1, len: 2 };
    assert!(expect_semantic(&p).contains("track list"));

    let mut p = base.clone();
    p.attrs.swap(0, 1);
    assert!(expect_semantic(&p).contains("not sorted"));

    let mut p = base.clone();
    p.attrs[0].node = 50;
    assert!(expect_semantic(&p).contains("attr 0: node 50"));

    let mut p = base.clone();
    p.attrs[0].val_len = 1 << 20;
    assert!(expect_semantic(&p).contains("attr value"));

    let mut p = base.clone();
    p.images[0].blob_len += 1;
    assert!(expect_semantic(&p).contains("image blob"));

    let mut p = base.clone();
    p.images[0].node = 9;
    assert!(expect_semantic(&p).contains("image 0: node 9"));

    let mut p = base.clone();
    p.forms[0].field_count = 2;
    assert!(expect_semantic(&p).contains("form fields"));

    let mut p = base.clone();
    p.forms[0].node = NONE;
    assert!(expect_semantic(&p).contains("form 0"));

    let mut p = base.clone();
    p.forms[0].action_len = 1 << 30;
    assert!(expect_semantic(&p).contains("form action"));

    let mut p = base.clone();
    p.fields[0] = 9;
    assert!(expect_semantic(&p).contains("field 0"));

    let mut p = base.clone();
    p.top_level = vec![2, 2];
    assert!(expect_semantic(&p).contains("strictly increasing"));

    let mut p = base.clone();
    p.top_level = vec![42];
    assert!(expect_semantic(&p).contains("top_level 0"));

    let mut p = base.clone();
    p.breakpoints = vec![900, 600];
    assert!(expect_semantic(&p).contains("breakpoints"));
}

#[test]
fn unknown_flag_bits_are_rejected_by_bytecheck_or_semantics() {
    // NodeFlags covers all 8 bits, so no u8 is "unknown"; the enum bytes
    // (kind/role) are the ones bytecheck polices.
    let mut bytes = encode(&sample_page(), 0).unwrap();
    // Find node 0's `kind` byte: it is the only 0x10 (Html) followed by 0x00
    // flags and 0x01 (Document) role at a 32-byte aligned node start in the
    // nodes table. Simpler: brute force over the archive and check that
    // setting a byte to 0xEE either fails structurally or leaves enums valid.
    let mut structural = 0;
    for i in HEADER_LEN..bytes.len() {
        let orig = bytes[i];
        bytes[i] = 0xEE;
        let crc = page_format::crc32(&bytes[HEADER_LEN..]);
        bytes[24..28].copy_from_slice(&crc.to_le_bytes());
        if let Ok((_, page)) = validate(&bytes) {
            for node in page.nodes.iter() {
                assert!(NodeKind::from_u8(node.kind.as_u8()).is_some());
                assert!(Role::from_u8(node.role.as_u8()).is_some());
            }
        } else if matches!(validate(&bytes), Err(PageError::Structural(_))) {
            structural += 1;
        }
        bytes[i] = orig;
    }
    assert!(
        structural >= 9,
        "each node's kind byte must be policed, saw {structural}"
    );
}
