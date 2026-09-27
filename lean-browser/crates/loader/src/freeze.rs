//! Freezing: DOM + computed styles -> [`FTree`]. Text runs get the
//! inherited style of their parent (plus propagated text decoration),
//! whitespace is collapsed according to `white-space`, list markers and
//! `::before`/`::after` boxes are generated, images are fetched and
//! sniffed, and inline SVG subtrees become one blob node. Inline children
//! of `<body>` are wrapped in anonymous blocks ([`wrap_body_inlines`]) so
//! every top-level node is block-level (scrollbar units, plan §4), but only
//! after the compression passes have run, so a collapsed body-level
//! wrapper's inline children end up in one anonymous block (one line, as
//! before the collapse) and whitespace between blocks is dropped rather
//! than wrapped.

use std::rc::Rc;

use css_subset::{ComputedStyle, Display, ListStyleType, NodeFlags, TextDecoration, WhiteSpace};
use page_format::{FormMethod, NodeKind, Role};
use url::Url;

use crate::a11y::{kind_for_tag, A11y};
use crate::css::cascade::Cascade;
use crate::css::select::PseudoKind;
use crate::dom::{Dom, NodeData, NodeId};
use crate::fetch::Fetcher;
use crate::image::{sniff, ImageInfo};
use crate::tree::{anonymous_style, text_style, FNode, FTree, FormData, ImageData};

/// Freezes the document. `base` is the resolved document base URL.
pub fn freeze(
    dom: &Dom,
    cascade: &Cascade<'_, '_>,
    a11y: &A11y,
    fetcher: &mut Fetcher,
    base: &Url,
) -> FTree {
    let mut tree = FTree {
        title: dom
            .find_first("title")
            .map(|t| crate::a11y::normalize(&dom.text_content(t)))
            .unwrap_or_default(),
        ..FTree::default()
    };
    let Some(root) = dom.nodes[0]
        .children
        .iter()
        .copied()
        .find(|&c| dom.is_element(c))
    else {
        tree.push(FNode::new(NodeKind::Html, ComputedStyle::INITIAL), None);
        return tree;
    };
    let mut f = Freezer {
        dom,
        cascade,
        a11y,
        fetcher,
        base,
        tree,
    };
    f.element(root, None);
    f.tree
}

struct Freezer<'x, 'a, 'i> {
    dom: &'x Dom,
    cascade: &'x Cascade<'a, 'i>,
    a11y: &'x A11y,
    fetcher: &'x mut Fetcher,
    base: &'x Url,
    tree: FTree,
}

impl Freezer<'_, '_, '_> {
    /// Freezes element `id` under `parent` (recursion depth is bounded by
    /// html5ever's own nesting limits).
    fn element(&mut self, id: NodeId, parent: Option<usize>) {
        let dom = self.dom;
        let Some(ns) = self.cascade.styles[id].as_ref() else {
            return;
        };
        let tag = dom.tag(id).unwrap_or("").to_string();
        let style = ns.style;
        let mut node = FNode::new(kind_for_tag(&tag), style);
        node.dom = Some(id);
        node.tag = tag.clone();
        node.attrs = dom.attrs(id).to_vec();
        node.role = self.a11y.role(dom, id);
        node.name = self.a11y.name(dom, id, node.role);
        if let Some(ctx) = &self.cascade.contexts[id] {
            node.ctx_key = (
                ctx.line_height_factor.map(f32::to_bits),
                Rc::as_ptr(&ctx.custom) as usize,
            );
        }
        let parent_deco = parent
            .map(|p| self.tree.nodes[p].deco)
            .unwrap_or(TextDecoration::empty());
        node.deco = parent_deco | style.text_decoration;
        node.flags = flags_for(dom, id, &tag, &style);

        // Replaced content and forms.
        match tag.as_str() {
            "img" => {
                let img = self.load_image(id);
                self.tree.images.push(img);
                node.image = Some(self.tree.images.len() - 1);
            }
            "svg" => {
                let markup = dom.serialize(id);
                let info = sniff(markup.as_bytes(), Some("image/svg+xml"));
                self.tree.images.push(ImageData {
                    bytes: markup.into_bytes(),
                    info,
                    blob: None,
                });
                node.image = Some(self.tree.images.len() - 1);
            }
            "form" => {
                let action = dom
                    .attr(id, "action")
                    .and_then(|a| self.base.join(a.trim()).ok())
                    .unwrap_or_else(|| self.base.clone());
                let method = if dom
                    .attr(id, "method")
                    .is_some_and(|m| m.trim().eq_ignore_ascii_case("post"))
                {
                    FormMethod::Post
                } else {
                    FormMethod::Get
                };
                self.tree.forms.push(FormData {
                    action: action.to_string(),
                    method,
                });
                node.form = Some(self.tree.forms.len() - 1);
            }
            _ => {}
        }

        let idx = self.tree.push(node, parent);
        if tag == "svg" {
            return; // the subtree lives in the blob
        }

        // List marker, then ::before, children, ::after.
        if style.display == Display::ListItem {
            if let Some(text) = marker_text(dom, id, style.list_style_type) {
                let mut m = FNode::new(
                    NodeKind::Marker,
                    text_style(&style, self.tree.nodes[idx].deco),
                );
                m.text = text;
                m.role = Role::Generic;
                self.tree.push(m, Some(idx));
            }
        }
        if let Some(g) = &ns.before {
            self.pseudo(idx, id, PseudoKind::Before, &g.style, &g.content);
        }
        for &c in &dom.nodes[id].children {
            match &dom.nodes[c].data {
                NodeData::Element { .. } => self.element(c, Some(idx)),
                NodeData::Text(t) => self.text(idx, c, t),
                _ => {}
            }
        }
        if let Some(g) = &ns.after {
            self.pseudo(idx, id, PseudoKind::After, &g.style, &g.content);
        }
    }

    fn pseudo(
        &mut self,
        parent: usize,
        origin: NodeId,
        kind: PseudoKind,
        style: &ComputedStyle,
        content: &str,
    ) {
        let mut n = FNode::new(NodeKind::Pseudo, *style);
        n.dom = Some(origin);
        n.pseudo = Some(kind);
        n.deco = self.tree.nodes[parent].deco | style.text_decoration;
        n.text = collapse_text(content, style.white_space);
        n.flags = if style.display.is_block_level() {
            NodeFlags::BLOCK
        } else {
            NodeFlags::empty()
        };
        if n.text.is_empty() {
            return;
        }
        self.tree.push(n, Some(parent));
    }

    fn text(&mut self, parent: usize, dom_id: NodeId, raw: &str) {
        let pstyle = self.tree.nodes[parent].style;
        let text = collapse_text(raw, pstyle.white_space);
        if text.is_empty() {
            return;
        }
        let mut n = FNode::new(
            NodeKind::Text,
            text_style(&pstyle, self.tree.nodes[parent].deco),
        );
        n.dom = Some(dom_id);
        n.text = text;
        n.role = Role::StaticText;
        if matches!(pstyle.white_space, WhiteSpace::Pre | WhiteSpace::PreWrap) {
            n.flags |= NodeFlags::PRE;
        }
        self.tree.push(n, Some(parent));
    }

    fn load_image(&mut self, id: NodeId) -> ImageData {
        let dom = self.dom;
        let attr_dim = |name: &str| -> u16 {
            dom.attr(id, name)
                .and_then(|v| v.trim().trim_end_matches("px").parse::<f32>().ok())
                .map(|v| v.round().clamp(0.0, 65535.0) as u16)
                .unwrap_or(0)
        };
        let fallback = ImageData {
            bytes: Vec::new(),
            info: ImageInfo {
                format: page_format::ImageFormat::Unsupported,
                width: attr_dim("width"),
                height: attr_dim("height"),
                store: false,
            },
            blob: None,
        };
        let Some(src) = dom.attr(id, "src").map(str::trim).filter(|s| !s.is_empty()) else {
            return fallback;
        };
        let Ok(url) = self.base.join(src) else {
            return fallback;
        };
        let Ok(res) = self.fetcher.fetch_subresource(&url) else {
            return fallback;
        };
        let mut info = sniff(&res.bytes, res.mime.as_deref());
        if info.width == 0 || info.height == 0 {
            info.width = attr_dim("width");
            info.height = attr_dim("height");
        }
        ImageData {
            bytes: if info.store { res.bytes } else { Vec::new() },
            info,
            blob: None,
        }
    }
}

fn flags_for(dom: &Dom, id: NodeId, tag: &str, style: &ComputedStyle) -> NodeFlags {
    let mut flags = NodeFlags::empty();
    let is_link = matches!(tag, "a" | "area") && dom.attr(id, "href").is_some();
    if is_link {
        flags |= NodeFlags::LINK | NodeFlags::FOCUSABLE;
    }
    if dom.attr(id, "id").is_some_and(|v| !v.is_empty()) {
        flags |= NodeFlags::ANCHOR_TARGET;
    }
    if style.display.is_block_level() {
        flags |= NodeFlags::BLOCK;
    }
    if matches!(
        tag,
        "img"
            | "svg"
            | "input"
            | "textarea"
            | "select"
            | "button"
            | "iframe"
            | "video"
            | "audio"
            | "canvas"
            | "object"
            | "embed"
    ) && !style.display.is_block_level()
    {
        flags |= NodeFlags::INLINE_REPLACED;
    }
    if matches!(style.white_space, WhiteSpace::Pre | WhiteSpace::PreWrap) {
        flags |= NodeFlags::PRE;
    }
    let is_control = matches!(tag, "input" | "textarea" | "select" | "button");
    if is_control {
        let disabled = dom.attr(id, "disabled").is_some();
        let hidden_input = tag == "input"
            && dom
                .attr(id, "type")
                .is_some_and(|t| t.trim().eq_ignore_ascii_case("hidden"));
        if !disabled && !hidden_input {
            flags |= NodeFlags::FOCUSABLE;
        }
        if dom.attr(id, "name").is_some_and(|n| !n.is_empty()) && !disabled {
            flags |= NodeFlags::IS_FORM_FIELD;
        }
    }
    if dom
        .attr(id, "tabindex")
        .and_then(|t| t.trim().parse::<i32>().ok())
        .is_some_and(|t| t >= 0)
    {
        flags |= NodeFlags::FOCUSABLE;
    }
    flags
}

/// Applies the `white-space` collapsing rules to a text run: collapsible
/// whitespace sequences become one space, newlines are kept only for
/// `pre*` values, CRLF is normalized.
pub fn collapse_text(raw: &str, ws: WhiteSpace) -> String {
    let raw = if raw.contains('\r') {
        raw.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        raw.to_string()
    };
    if !ws.collapses() {
        return raw;
    }
    let mut out = String::with_capacity(raw.len());
    let mut in_space = false;
    let mut pending_newline = false;
    for c in raw.chars() {
        let collapsible = c == ' ' || c == '\t' || c == '\n' || c == '\u{c}';
        if collapsible {
            if c == '\n' && ws.preserves_newlines() {
                pending_newline = true;
            }
            in_space = true;
            continue;
        }
        if in_space {
            if pending_newline {
                out.push('\n');
            } else {
                out.push(' ');
            }
            in_space = false;
            pending_newline = false;
        }
        out.push(c);
    }
    if in_space {
        out.push(if pending_newline { '\n' } else { ' ' });
    }
    out
}

/// Marker text for a list item: bullet glyphs or the ordinal.
fn marker_text(dom: &Dom, id: NodeId, kind: ListStyleType) -> Option<String> {
    match kind {
        ListStyleType::None => None,
        ListStyleType::Disc => Some("\u{2022} ".to_string()),
        ListStyleType::Circle => Some("\u{25E6} ".to_string()),
        ListStyleType::Square => Some("\u{25AA} ".to_string()),
        ListStyleType::Decimal => Some(format!("{}. ", ordinal(dom, id))),
    }
}

/// 1-based position among list-item siblings, honouring `<ol start>`,
/// `<ol reversed>` and `<li value>`.
fn ordinal(dom: &Dom, id: NodeId) -> i64 {
    let Some(parent) = dom.nodes[id].parent else {
        return 1;
    };
    let reversed = dom.tag(parent) == Some("ol") && dom.attr(parent, "reversed").is_some();
    let items: Vec<NodeId> = dom.nodes[parent]
        .children
        .iter()
        .copied()
        .filter(|&c| dom.tag(c) == Some("li"))
        .collect();
    let start = dom
        .attr(parent, "start")
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(if reversed { items.len() as i64 } else { 1 });
    let mut n = start;
    let step = if reversed { -1 } else { 1 };
    for (i, &li) in items.iter().enumerate() {
        if let Some(v) = dom
            .attr(li, "value")
            .and_then(|v| v.trim().parse::<i64>().ok())
        {
            n = v;
        } else if i > 0 {
            n += step;
        }
        if li == id {
            return n;
        }
    }
    n
}

/// Wraps runs of inline-level children of `<body>` in anonymous block
/// boxes so every top-level node is a scrollbar unit. Skipped when the
/// body is a flex/grid container (its children are already items). Runs
/// from [`crate::compress::compress`] after passes 1-4.
pub(crate) fn wrap_body_inlines(tree: &mut FTree) {
    let Some(body) = tree.body() else {
        return;
    };
    if tree.is_item_container(body) || !tree.nodes[body].style.display.is_block_level() {
        return;
    }
    let children: Vec<usize> = tree.nodes[body]
        .children
        .iter()
        .copied()
        .filter(|&c| tree.nodes[c].alive)
        .collect();
    if children.iter().all(|&c| tree.nodes[c].is_block()) {
        return;
    }
    let body_style = tree.nodes[body].style;
    let body_deco = tree.nodes[body].deco;
    let mut new_children: Vec<usize> = Vec::new();
    let mut run: Vec<usize> = Vec::new();
    let flush = |tree: &mut FTree, run: &mut Vec<usize>, out: &mut Vec<usize>| {
        if run.is_empty() {
            return;
        }
        let mut wrapper = FNode::new(NodeKind::Div, anonymous_style(&body_style));
        wrapper.anonymous = true;
        wrapper.tag = "div".to_string();
        wrapper.deco = body_deco;
        wrapper.flags = NodeFlags::BLOCK;
        wrapper.parent = Some(body);
        wrapper.children = std::mem::take(run);
        let idx = tree.nodes.len();
        let kids = wrapper.children.clone();
        tree.nodes.push(wrapper);
        for c in kids {
            tree.nodes[c].parent = Some(idx);
        }
        out.push(idx);
    };
    for c in children {
        if tree.nodes[c].is_block() {
            flush(tree, &mut run, &mut new_children);
            new_children.push(c);
        } else {
            run.push(c);
        }
    }
    flush(tree, &mut run, &mut new_children);
    tree.nodes[body].children = new_children;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_rules() {
        assert_eq!(collapse_text("  a \n\t b  ", WhiteSpace::Normal), " a b ");
        assert_eq!(collapse_text("a\r\nb", WhiteSpace::Pre), "a\nb");
        assert_eq!(collapse_text("a \n  b", WhiteSpace::PreLine), "a\nb");
        assert_eq!(collapse_text("   ", WhiteSpace::Nowrap), " ");
        assert_eq!(collapse_text("x\u{a0}y", WhiteSpace::Normal), "x\u{a0}y");
    }

    #[test]
    fn ordinals() {
        let dom = Dom::parse("<ol start=3><li>a</li><li value=10>b</li><li>c</li></ol><ol reversed><li>x</li><li>y</li></ol>");
        let lis: Vec<_> = dom
            .descendants(0)
            .filter(|&n| dom.tag(n) == Some("li"))
            .collect();
        assert_eq!(ordinal(&dom, lis[0]), 3);
        assert_eq!(ordinal(&dom, lis[1]), 10);
        assert_eq!(ordinal(&dom, lis[2]), 11);
        assert_eq!(ordinal(&dom, lis[3]), 2);
        assert_eq!(ordinal(&dom, lis[4]), 1);
    }
}
