//! The seven compression passes (plan §6), run in order on a frozen tree.
//! Each pass is a pure function of the tree and records before/after
//! counts. Pass 3's safety rules are the ones that matter: a collapsed
//! wrapper must leave every descendant's computed style unchanged, which
//! [`crate::check::check_style_preserving`] verifies by re-running the
//! cascade on the compressed tree.

use std::collections::HashMap;

use css_subset::{
    ComputedStyle, Display, Float, Length, NodeFlags, Overflow, Position, Rgba, StyleTable,
    StyleTableFull, VerticalAlign, Z_INDEX_AUTO,
};
use page_format::{AttrKey, NodeKind, Role};
use url::Url;

use crate::tree::FTree;

/// Counts recorded by one pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PassStats {
    /// Pass number (1..=7).
    pub number: u8,
    /// Pass name.
    pub name: &'static str,
    /// What is counted (`nodes`, `styles`, `blobs`, `attrs`).
    pub unit: &'static str,
    /// Count before the pass.
    pub before: usize,
    /// Count after the pass.
    pub after: usize,
}

/// Output of the whole compression: the interned style table and blobs.
#[derive(Debug, Default)]
pub struct Compressed {
    /// Interned styles (`FNode::style_id` indexes here).
    pub styles: Vec<ComputedStyle>,
    /// Concatenated image blobs (`ImageData::blob` ranges point here).
    pub blobs: Vec<u8>,
    /// Per-pass statistics.
    pub stats: Vec<PassStats>,
}

/// Runs passes 1-7. `base` resolves `href`/`src`/`action` in pass 7.
pub fn compress(tree: &mut FTree, base: &Url) -> Compressed {
    let mut out = Compressed::default();
    let record = |tree: &FTree, number: u8, name: &'static str, before: usize| PassStats {
        number,
        name,
        unit: "nodes",
        before,
        after: tree.alive_count(),
    };

    let before = tree.alive_count();
    drop_never_rendered(tree);
    out.stats
        .push(record(tree, 1, "drop never-rendered", before));

    let before = tree.alive_count();
    drop_whitespace_text(tree);
    out.stats
        .push(record(tree, 2, "drop whitespace-only text", before));

    let before = tree.alive_count();
    collapse_wrappers(tree);
    out.stats
        .push(record(tree, 3, "collapse transparent wrappers", before));

    let before = tree.alive_count();
    merge_text_runs(tree);
    out.stats
        .push(record(tree, 4, "merge adjacent text runs", before));

    // Not a pass: now that whitespace between body blocks is gone (2) and
    // body-level wrappers are collapsed (3), wrap what is still inline
    // under <body> so every top-level node is block-level. Doing this
    // before the passes left an empty anonymous block per newline between
    // body blocks and split a collapsed wrapper's inline run into one
    // scrollbar unit per child.
    crate::freeze::wrap_body_inlines(tree);

    let node_styles = tree.alive_count();
    let (styles, _distinct) = intern_styles(tree);
    out.stats.push(PassStats {
        number: 5,
        name: "intern computed styles",
        unit: "styles",
        before: node_styles,
        after: styles.len(),
    });
    out.styles = styles;

    let (blobs, before, after) = dedup_blobs(tree);
    out.stats.push(PassStats {
        number: 6,
        name: "deduplicate blobs",
        unit: "blobs",
        before,
        after,
    });
    out.blobs = blobs;

    let (before, after) = trim_attributes(tree, base);
    out.stats.push(PassStats {
        number: 7,
        name: "trim attributes",
        unit: "attrs",
        before,
        after,
    });
    out
}

// ---------------------------------------------------------------------------
// Pass 1

/// Tags never rendered in document mode (besides `display: none`).
fn never_rendered_tag(tag: &str) -> bool {
    matches!(
        tag,
        "head" | "meta" | "link" | "title" | "script" | "style" | "template" | "base"
    )
}

/// Drops `head` & co., `display: none` subtrees. `<noscript>` is kept
/// because JS did not run (plan §6.1).
pub fn drop_never_rendered(tree: &mut FTree) {
    for idx in tree.pre_order() {
        let n = &tree.nodes[idx];
        if !n.alive {
            continue;
        }
        if never_rendered_tag(&n.tag) || n.style.display == Display::None {
            tree.remove(idx);
        }
    }
}

// ---------------------------------------------------------------------------
// Pass 2

/// CSS collapsible whitespace (space, tab, LF, CR, FF). `U+00A0` is
/// deliberately not in the set: `&nbsp;` between two blocks is a real
/// (empty-looking) line.
fn is_collapsible(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

/// Drops whitespace-only text nodes that sit between block boxes (or a
/// block edge) under collapsing `white-space`.
pub fn drop_whitespace_text(tree: &mut FTree) {
    for idx in tree.pre_order() {
        let n = &tree.nodes[idx];
        if n.kind != NodeKind::Text || !n.alive {
            continue;
        }
        if !n.text.chars().all(is_collapsible) {
            continue;
        }
        let Some(p) = n.parent else {
            continue;
        };
        let parent = &tree.nodes[p];
        if !parent.style.white_space.collapses() {
            continue;
        }
        let siblings = &parent.children;
        let pos = siblings.iter().position(|&c| c == idx).unwrap_or(0);
        let parent_is_block = parent.is_block()
            || parent.style.display == Display::InlineBlock
            || tree.is_item_container(p)
            || parent.style.display == Display::TableCell;
        let prev_block = match pos.checked_sub(1).map(|i| siblings[i]) {
            Some(prev) => tree.nodes[prev].is_block(),
            None => parent_is_block,
        };
        let next_block = match siblings.get(pos + 1) {
            Some(&next) => tree.nodes[next].is_block(),
            None => parent_is_block,
        };
        if (prev_block && next_block) || tree.is_item_container(p) {
            tree.remove(idx);
        }
    }
}

// ---------------------------------------------------------------------------
// Pass 3

/// Attribute keys that pass 7 keeps; a wrapper carrying any is not
/// transparent.
fn is_kept_attr(key: &str) -> bool {
    attr_key(key).is_some() || key == "role" || key.starts_with("aria-") || key == "tabindex"
}

/// Whether `E` (a live element) may be removed with its children reparented.
fn is_transparent(tree: &FTree, e: usize) -> bool {
    let n = &tree.nodes[e];
    if n.anonymous || n.dom.is_none() || n.pseudo.is_some() || !n.kind.is_element() {
        return false;
    }
    if matches!(
        n.kind,
        NodeKind::Html | NodeKind::Body | NodeKind::Form | NodeKind::Li
    ) || n.image.is_some()
        || n.form.is_some()
    {
        return false;
    }
    let Some(p) = n.parent else {
        return false;
    };
    let parent = &tree.nodes[p];
    if tree.is_item_container(p) {
        return false;
    }
    let s = &n.style;
    // Box-generating properties must be at their no-op values.
    let zero = |l: &Length| *l == Length::ZERO;
    if !(s.display == Display::Block || s.display == Display::Inline)
        || !s.margin.iter().all(zero)
        || !s.padding.iter().all(zero)
        || s.border_width.iter().any(|w| *w != 0.0)
        || s.background_color != Rgba::TRANSPARENT
        || s.opacity != 1.0
        || s.overflow_x != Overflow::Visible
        || s.overflow_y != Overflow::Visible
        || s.position != Position::Static
        || s.float != Float::None
        || s.clear != css_subset::Clear::None
        || !s.width.is_auto()
        || !s.height.is_auto()
        || !s.min_width.is_auto()
        || !s.min_height.is_auto()
        || !s.max_width.is_auto()
        || !s.max_height.is_auto()
        || s.z_index != Z_INDEX_AUTO
        || !s.text_decoration.is_empty()
        || s.vertical_align != VerticalAlign::Baseline
    {
        return false;
    }
    // Inherited values seen by children must not change.
    if ComputedStyle::inherit_from(s) != ComputedStyle::inherit_from(&parent.style)
        || n.ctx_key != parent.ctx_key
        || n.deco != parent.deco
    {
        return false;
    }
    // Accessibility and interaction structure survives.
    if n.role != Role::Generic
        || !n.name.is_empty()
        || n.flags.intersects(
            NodeFlags::LINK
                | NodeFlags::ANCHOR_TARGET
                | NodeFlags::FOCUSABLE
                | NodeFlags::IS_FORM_FIELD,
        )
        || n.attrs.iter().any(|(k, _)| is_kept_attr(k))
    {
        return false;
    }
    // Generated content belongs to E.
    if n.children
        .iter()
        .any(|&c| tree.nodes[c].pseudo.is_some() || tree.nodes[c].kind == NodeKind::Marker)
    {
        return false;
    }
    if s.display == Display::Block {
        // Removing a block wrapper is layout-neutral only when its parent
        // is a block container and either E's children are all blocks
        // (they take E's place between the same neighbours) or E's
        // siblings are all blocks (E's inline runs form the same
        // anonymous blocks in the parent as they did inside E; under
        // <body> `wrap_body_inlines` runs after this pass and makes that
        // one anonymous block explicit).
        let parent_is_block_container = matches!(
            parent.style.display,
            Display::Block
                | Display::ListItem
                | Display::TableCell
                | Display::TableCaption
                | Display::InlineBlock
        );
        let is_block = |c: &usize| tree.nodes[*c].is_block();
        parent_is_block_container
            && (n.children.iter().all(is_block) || parent.children.iter().all(is_block))
    } else {
        // An inline wrapper must contain only inline-level content.
        n.children.iter().all(|&c| !tree.nodes[c].is_block())
    }
}

/// Removes transparent wrappers, reparenting their children.
pub fn collapse_wrappers(tree: &mut FTree) {
    // Post-order so a wrapper's own wrapper children are handled first.
    let mut order = tree.pre_order();
    order.reverse();
    for idx in order {
        if tree.nodes[idx].alive && is_transparent(tree, idx) {
            tree.replace_with_children(idx);
        }
    }
}

// ---------------------------------------------------------------------------
// Pass 4

/// Merges adjacent text runs with identical style under the same parent.
/// `<br>`, anchors and generated content are nodes of other kinds, so runs
/// never merge across them; a `white-space` change means a different style.
pub fn merge_text_runs(tree: &mut FTree) {
    for idx in tree.pre_order() {
        merge_text_runs_at(tree, idx);
    }
}

/// Re-runs the merge for one parent after a merge changed its child list.
fn merge_text_runs_at(tree: &mut FTree, parent: usize) {
    loop {
        let children = tree.nodes[parent].children.clone();
        let mut merged_any = false;
        for i in 0..children.len().saturating_sub(1) {
            let (a, b) = (children[i], children[i + 1]);
            let (na, nb) = (&tree.nodes[a], &tree.nodes[b]);
            if na.alive
                && nb.alive
                && na.kind == NodeKind::Text
                && nb.kind == NodeKind::Text
                && na.style == nb.style
            {
                let mut text = std::mem::take(&mut tree.nodes[b].text);
                if tree.nodes[a].style.white_space.collapses()
                    && tree.nodes[a].text.ends_with(' ')
                    && text.starts_with(' ')
                {
                    text.remove(0);
                }
                tree.nodes[a].text.push_str(&text);
                tree.remove(b);
                merged_any = true;
                break;
            }
        }
        if !merged_any {
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// Pass 5

/// Interns every live node's style. On overflow (> 65 000 distinct
/// styles) lengths are quantized to 0.5 px and interning retried; if that
/// still overflows the page is flagged truncated and the overflow maps to
/// the nearest existing style.
pub fn intern_styles(tree: &mut FTree) -> (Vec<ComputedStyle>, usize) {
    let order = tree.pre_order();
    let distinct_before = {
        let mut set = std::collections::HashSet::new();
        for &i in &order {
            set.insert(tree.nodes[i].style);
        }
        set.len()
    };
    // Style 0 is always the initial style so an empty page stays valid.
    let attempt = |tree: &FTree| -> Result<(StyleTable, Vec<u16>), StyleTableFull> {
        let mut table = StyleTable::new();
        table.intern(&ComputedStyle::INITIAL)?;
        let mut ids = vec![0u16; tree.nodes.len()];
        for &i in &order {
            ids[i] = table.intern(&tree.nodes[i].style)?;
        }
        Ok((table, ids))
    };
    let (table, ids) = match attempt(tree) {
        Ok(r) => r,
        Err(StyleTableFull) => {
            for &i in &order {
                tree.nodes[i].style.quantize_lengths(0.5);
            }
            match attempt(tree) {
                Ok(r) => r,
                Err(StyleTableFull) => {
                    tree.truncated = true;
                    let mut table = StyleTable::new();
                    let _ = table.intern(&ComputedStyle::INITIAL);
                    let mut ids = vec![0u16; tree.nodes.len()];
                    for &i in &order {
                        ids[i] = match table.intern(&tree.nodes[i].style) {
                            Ok(id) => id,
                            Err(StyleTableFull) => nearest_style(&table, &tree.nodes[i].style),
                        };
                    }
                    (table, ids)
                }
            }
        }
    };
    for &i in &order {
        tree.nodes[i].style_id = ids[i];
    }
    (table.into_vec(), distinct_before)
}

/// The id of the interned style with the fewest differing key properties.
fn nearest_style(table: &StyleTable, s: &ComputedStyle) -> u16 {
    let dist = |a: &ComputedStyle| -> u32 {
        let mut d = 0;
        d += (a.display != s.display) as u32 * 8;
        d += (a.font_size != s.font_size) as u32 * 4;
        d += (a.font_family != s.font_family) as u32 * 4;
        d += (a.font_weight != s.font_weight) as u32 * 2;
        d += (a.color != s.color) as u32 * 2;
        d += (a.background_color != s.background_color) as u32 * 2;
        d += (a.margin != s.margin) as u32;
        d += (a.padding != s.padding) as u32;
        d += (a.white_space != s.white_space) as u32 * 2;
        d += (a.text_decoration != s.text_decoration) as u32;
        d
    };
    let mut best = (u32::MAX, 0u16);
    // Scan a bounded prefix: the first styles are the most common ones.
    for (i, st) in table.styles().iter().enumerate().take(4096) {
        let d = dist(st);
        if d < best.0 {
            best = (d, i as u16);
            if d == 0 {
                break;
            }
        }
    }
    best.1
}

// ---------------------------------------------------------------------------
// Pass 6

/// Assigns blob ranges, sharing bytes between identical images. Returns
/// `(blobs, stored images, distinct blobs)`.
pub fn dedup_blobs(tree: &mut FTree) -> (Vec<u8>, usize, usize) {
    let mut blobs = Vec::new();
    let mut seen: HashMap<u64, (u32, u32)> = HashMap::new();
    let live_images: std::collections::HashSet<usize> = tree
        .pre_order()
        .into_iter()
        .filter_map(|i| tree.nodes[i].image)
        .collect();
    let mut stored = 0;
    for (i, img) in tree.images.iter_mut().enumerate() {
        if !live_images.contains(&i) || !img.info.store || img.bytes.is_empty() {
            img.blob = None;
            continue;
        }
        stored += 1;
        let hash = fnv1a(&img.bytes);
        let range = match seen.get(&hash) {
            Some(&(off, len)) if blobs[off as usize..(off + len) as usize] == img.bytes[..] => {
                (off, len)
            }
            _ => {
                let off = blobs.len() as u32;
                blobs.extend_from_slice(&img.bytes);
                let r = (off, img.bytes.len() as u32);
                seen.insert(hash, r);
                r
            }
        };
        img.blob = Some(range);
        img.bytes = Vec::new();
    }
    let distinct = seen.len();
    (blobs, stored, distinct)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// ---------------------------------------------------------------------------
// Pass 7

/// Maps an attribute name to a kept key.
pub fn attr_key(name: &str) -> Option<AttrKey> {
    Some(match name {
        "href" => AttrKey::Href,
        "src" => AttrKey::Src,
        "alt" => AttrKey::Alt,
        "id" => AttrKey::Id,
        "name" => AttrKey::Name,
        "value" => AttrKey::Value,
        "type" => AttrKey::Type,
        "action" => AttrKey::Action,
        "method" => AttrKey::Method,
        "placeholder" => AttrKey::Placeholder,
        "checked" => AttrKey::Checked,
        "for" => AttrKey::For,
        "lang" => AttrKey::Lang,
        "title" => AttrKey::Title,
        "aria-label" => AttrKey::AriaLabel,
        "colspan" => AttrKey::Colspan,
        "rowspan" => AttrKey::Rowspan,
        _ => return None,
    })
}

/// Keeps only [`AttrKey`] attributes, resolving URLs. Returns
/// `(attrs before, attrs after)`.
pub fn trim_attributes(tree: &mut FTree, base: &Url) -> (usize, usize) {
    let mut before = 0;
    let mut after = 0;
    for idx in tree.pre_order() {
        let n = &mut tree.nodes[idx];
        let raw = std::mem::take(&mut n.attrs);
        before += raw.len();
        let mut kept: Vec<(AttrKey, String)> = Vec::new();
        for (k, v) in raw {
            let Some(key) = attr_key(&k) else {
                continue;
            };
            if kept.iter().any(|(existing, _)| *existing == key) {
                continue;
            }
            let value = match key {
                AttrKey::Href | AttrKey::Src | AttrKey::Action => {
                    let t = v.trim();
                    if t.is_empty() {
                        continue;
                    }
                    match base.join(t) {
                        Ok(u) => u.to_string(),
                        Err(_) => continue,
                    }
                }
                AttrKey::Type | AttrKey::Method => v.trim().to_ascii_lowercase(),
                _ => v,
            };
            kept.push((key, value));
        }
        after += kept.len();
        if kept.is_empty() {
            n.flags.remove(NodeFlags::HAS_ATTRS);
        } else {
            n.flags |= NodeFlags::HAS_ATTRS;
        }
        n.kept = kept;
    }
    (before, after)
}

/// Convenience for tests: a block-level element node.
#[cfg(test)]
pub(crate) fn block(kind: NodeKind, tag: &str) -> crate::tree::FNode {
    use crate::tree::FNode;
    let mut s = ComputedStyle::INITIAL;
    s.display = Display::Block;
    let mut n = FNode::new(kind, s);
    n.tag = tag.to_string();
    n.dom = Some(0);
    n.flags = NodeFlags::BLOCK;
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{text_style, FNode};
    use css_subset::TextDecoration;

    fn text(parent_style: &ComputedStyle, s: &str) -> FNode {
        let mut n = FNode::new(
            NodeKind::Text,
            text_style(parent_style, TextDecoration::empty()),
        );
        n.text = s.to_string();
        n.dom = Some(0);
        n
    }

    fn inline(tag: &str) -> FNode {
        let mut n = FNode::new(NodeKind::Span, ComputedStyle::INITIAL);
        n.tag = tag.to_string();
        n.dom = Some(0);
        n
    }

    #[test]
    fn pass1_and_pass2() {
        let mut t = FTree::default();
        let html = t.push(block(NodeKind::Html, "html"), None);
        let head = t.push(block(NodeKind::Element, "head"), Some(html));
        t.push(block(NodeKind::Element, "title"), Some(head));
        let body = t.push(block(NodeKind::Body, "body"), Some(html));
        let bs = t.nodes[body].style;
        t.push(text(&bs, "\n  "), Some(body));
        let p = t.push(block(NodeKind::P, "p"), Some(body));
        t.push(text(&bs, " "), Some(p)); // between edge and text: kept
        t.push(text(&bs, "x"), Some(p));
        t.push(text(&bs, "\n"), Some(body));
        let mut hidden = block(NodeKind::Div, "div");
        hidden.style.display = Display::None;
        let h = t.push(hidden, Some(body));
        t.push(text(&bs, "gone"), Some(h));
        drop_never_rendered(&mut t);
        assert!(!t.nodes[head].alive && !t.nodes[h].alive);
        // html, body, "\n  ", p, " ", "x", "\n"
        assert_eq!(t.alive_count(), 7);
        drop_whitespace_text(&mut t);
        // Both body-level whitespace runs sit between block edges; the one
        // inside <p> precedes inline text and is a rendered space.
        assert_eq!(t.alive_count(), 5);
        assert_eq!(t.nodes[p].children.len(), 2);
    }

    #[test]
    fn pass3_collapses_only_transparent_wrappers() {
        let mut t = FTree::default();
        let html = t.push(block(NodeKind::Html, "html"), None);
        let body = t.push(block(NodeKind::Body, "body"), Some(html));
        let bs = t.nodes[body].style;
        // <div><p>x</p></div>: div is transparent.
        let div = t.push(block(NodeKind::Div, "div"), Some(body));
        let p = t.push(block(NodeKind::P, "p"), Some(div));
        t.push(text(&bs, "x"), Some(p));
        // <div id=k><p>y</p></div>: keeps its id.
        let mut keep = block(NodeKind::Div, "div");
        keep.attrs.push(("id".into(), "k".into()));
        keep.flags |= NodeFlags::ANCHOR_TARGET;
        let keep = t.push(keep, Some(body));
        t.push(block(NodeKind::P, "p"), Some(keep));
        // <div style="margin:1px"><p>z</p></div>: not transparent.
        let mut boxed = block(NodeKind::Div, "div");
        boxed.style.margin[0] = Length::px(1.0);
        let boxed = t.push(boxed, Some(body));
        t.push(block(NodeKind::P, "p"), Some(boxed));
        // <p><span>a</span></p>: inline span is transparent.
        let p2 = t.push(block(NodeKind::P, "p"), Some(body));
        let span = t.push(inline("span"), Some(p2));
        let sa = t.push(text(&bs, "a"), Some(span));
        // <p><span style="font-weight:bold">b</span></p>: changes inheritance.
        let p3 = t.push(block(NodeKind::P, "p"), Some(body));
        let mut bold = inline("span");
        bold.style.font_weight = css_subset::FontWeight::Bold;
        let bold = t.push(bold, Some(p3));
        let bold_style = t.nodes[bold].style;
        t.push(text(&bold_style, "b"), Some(bold));
        // <nav><p>n</p></nav>: landmark role.
        let mut nav = block(NodeKind::Nav, "nav");
        nav.role = Role::Navigation;
        let nav = t.push(nav, Some(body));
        t.push(block(NodeKind::P, "p"), Some(nav));

        for i in 0..t.nodes.len() {
            if t.nodes[i].kind == NodeKind::P {
                t.nodes[i].role = Role::Paragraph;
            }
        }
        collapse_wrappers(&mut t);
        assert!(!t.nodes[div].alive);
        assert_eq!(t.nodes[p].parent, Some(body));
        assert!(t.nodes[keep].alive);
        assert!(t.nodes[boxed].alive);
        assert!(!t.nodes[span].alive);
        assert_eq!(t.nodes[sa].parent, Some(p2));
        assert!(t.nodes[bold].alive);
        assert!(t.nodes[nav].alive);
        assert!(t.nodes[body].alive && t.nodes[html].alive);
        let order: Vec<usize> = t.nodes[body].children.clone();
        assert_eq!(order[0], p);
    }

    #[test]
    fn pass4_merges_runs() {
        let mut t = FTree::default();
        let html = t.push(block(NodeKind::Html, "html"), None);
        let p = t.push(block(NodeKind::P, "p"), Some(html));
        let ps = t.nodes[p].style;
        let a = t.push(text(&ps, "hello "), Some(p));
        t.push(text(&ps, " world"), Some(p));
        t.push(text(&ps, "!"), Some(p));
        let mut other = text(&ps, "X");
        other.style.font_weight = css_subset::FontWeight::Bold;
        let o = t.push(other, Some(p));
        t.push(text(&ps, "y"), Some(p));
        merge_text_runs(&mut t);
        assert_eq!(t.nodes[a].text, "hello world!");
        assert_eq!(t.nodes[p].children.len(), 3);
        assert_eq!(t.nodes[p].children[1], o);
    }

    #[test]
    fn pass5_interns_and_pass7_trims() {
        let mut t = FTree::default();
        let html = t.push(block(NodeKind::Html, "html"), None);
        let mut a = inline("a");
        a.attrs = vec![
            ("href".into(), "../x?y#z".into()),
            ("class".into(), "c".into()),
            ("data-q".into(), "1".into()),
            ("TYPE".into(), "X".into()),
            ("type".into(), "Sub".into()),
            ("id".into(), "i".into()),
        ];
        let a = t.push(a, Some(html));
        let b = t.push(inline("b"), Some(html));
        let (styles, distinct) = intern_styles(&mut t);
        assert_eq!(distinct, 2);
        assert_eq!(styles.len(), 2); // initial + block
        assert_eq!(t.nodes[a].style_id, 0);
        assert_eq!(t.nodes[html].style_id, 1);
        let base = Url::parse("https://example.test/a/b/c.html").unwrap();
        let (before, after) = trim_attributes(&mut t, &base);
        assert_eq!((before, after), (6, 3));
        assert_eq!(
            t.nodes[a].kept,
            vec![
                (AttrKey::Href, "https://example.test/a/x?y#z".to_string()),
                (AttrKey::Type, "sub".to_string()),
                (AttrKey::Id, "i".to_string()),
            ]
        );
        assert!(t.nodes[a].flags.contains(NodeFlags::HAS_ATTRS));
        assert!(!t.nodes[b].flags.contains(NodeFlags::HAS_ATTRS));
    }

    #[test]
    fn pass6_shares_identical_blobs() {
        use crate::image::ImageInfo;
        use crate::tree::ImageData;
        let mut t = FTree::default();
        let html = t.push(block(NodeKind::Html, "html"), None);
        let info = ImageInfo {
            format: page_format::ImageFormat::Png,
            width: 1,
            height: 1,
            store: true,
        };
        for bytes in [b"AAA".to_vec(), b"BBB".to_vec(), b"AAA".to_vec()] {
            t.images.push(ImageData {
                bytes,
                info: info.clone(),
                blob: None,
            });
            let mut n = FNode::new(NodeKind::Image, ComputedStyle::INITIAL);
            n.image = Some(t.images.len() - 1);
            n.dom = Some(0);
            t.push(n, Some(html));
        }
        let (blobs, stored, distinct) = dedup_blobs(&mut t);
        assert_eq!(blobs, b"AAABBB");
        assert_eq!((stored, distinct), (3, 2));
        assert_eq!(t.images[0].blob, Some((0, 3)));
        assert_eq!(t.images[1].blob, Some((3, 3)));
        assert_eq!(t.images[2].blob, Some((0, 3)));
    }
}
