//! The frozen tree: what the cascade produced, in a mutable arena that the
//! compression passes rewrite before serialization. Nodes are unlinked
//! (`alive = false`) rather than removed so indices stay stable.

use css_subset::{ComputedStyle, Display, NodeFlags, TextDecoration};
use page_format::{AttrKey, FormMethod, NodeKind, Role};

use crate::css::select::PseudoKind;
use crate::dom::NodeId;
use crate::image::ImageInfo;

/// One frozen node.
#[derive(Clone, Debug)]
pub struct FNode {
    /// Originating DOM node (elements, text, pseudo-elements); `None` for
    /// generated markers and anonymous wrappers.
    pub dom: Option<NodeId>,
    /// For `::before`/`::after` boxes, which one.
    pub pseudo: Option<PseudoKind>,
    /// Anonymous block wrapper introduced around inline body children.
    pub anonymous: bool,
    /// Page-file kind.
    pub kind: NodeKind,
    /// Element tag (empty for non-elements).
    pub tag: String,
    /// Computed style.
    pub style: ComputedStyle,
    /// Interned style id (after pass 5).
    pub style_id: u16,
    /// `line-height: <number>` factor and custom-property scope identity
    /// inherited by children (pass 3 requires them equal to the parent's).
    pub ctx_key: (Option<u32>, usize),
    /// Text decoration accumulated from ancestors *including* this node,
    /// applied to descendant text.
    pub deco: TextDecoration,
    /// Text run (text, pseudo and marker nodes).
    pub text: String,
    /// Raw attributes (elements) until pass 7.
    pub attrs: Vec<(String, String)>,
    /// Kept attributes after pass 7.
    pub kept: Vec<(AttrKey, String)>,
    /// Accessibility role.
    pub role: Role,
    /// Accessible name.
    pub name: String,
    /// Node flags.
    pub flags: NodeFlags,
    /// Parent index.
    pub parent: Option<usize>,
    /// Children in order.
    pub children: Vec<usize>,
    /// Whether the node is still part of the tree.
    pub alive: bool,
    /// Index into [`FTree::images`].
    pub image: Option<usize>,
    /// Index into [`FTree::forms`] for `<form>` elements.
    pub form: Option<usize>,
}

impl FNode {
    /// A blank node of the given kind.
    pub fn new(kind: NodeKind, style: ComputedStyle) -> FNode {
        FNode {
            dom: None,
            pseudo: None,
            anonymous: false,
            kind,
            tag: String::new(),
            style,
            style_id: 0,
            ctx_key: (None, 0),
            deco: TextDecoration::empty(),
            text: String::new(),
            attrs: Vec::new(),
            kept: Vec::new(),
            role: Role::Generic,
            name: String::new(),
            flags: NodeFlags::empty(),
            parent: None,
            children: Vec::new(),
            alive: true,
            image: None,
            form: None,
        }
    }

    /// Whether this node generates a block-level box.
    pub fn is_block(&self) -> bool {
        self.flags.contains(NodeFlags::BLOCK)
    }

    /// Text-like nodes (text runs, generated content, markers).
    pub fn is_textual(&self) -> bool {
        matches!(
            self.kind,
            NodeKind::Text | NodeKind::Pseudo | NodeKind::Marker
        )
    }
}

/// Bytes of an image plus what was sniffed from them.
#[derive(Clone, Debug)]
pub struct ImageData {
    /// Encoded bytes (empty for placeholders).
    pub bytes: Vec<u8>,
    /// Format and intrinsic size.
    pub info: ImageInfo,
    /// Blob range assigned by pass 6.
    pub blob: Option<(u32, u32)>,
}

/// A `<form>`.
#[derive(Clone, Debug)]
pub struct FormData {
    /// Absolute action URL.
    pub action: String,
    /// Submission method.
    pub method: FormMethod,
}

/// The frozen page.
#[derive(Debug, Default)]
pub struct FTree {
    /// Node arena; index 0 is the root (`<html>`).
    pub nodes: Vec<FNode>,
    /// Images referenced by nodes.
    pub images: Vec<ImageData>,
    /// Forms referenced by nodes.
    pub forms: Vec<FormData>,
    /// Document title.
    pub title: String,
    /// Set when limits were hit (style table overflow).
    pub truncated: bool,
}

impl FTree {
    /// Appends a node under `parent` (or as root when `None`).
    pub fn push(&mut self, mut node: FNode, parent: Option<usize>) -> usize {
        node.parent = parent;
        let idx = self.nodes.len();
        self.nodes.push(node);
        if let Some(p) = parent {
            self.nodes[p].children.push(idx);
        }
        idx
    }

    /// Number of live nodes.
    pub fn alive_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.alive).count()
    }

    /// Live nodes in pre-order from the root.
    pub fn pre_order(&self) -> Vec<usize> {
        let mut out = Vec::with_capacity(self.nodes.len());
        if self.nodes.is_empty() {
            return out;
        }
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            if !self.nodes[n].alive {
                continue;
            }
            out.push(n);
            for &c in self.nodes[n].children.iter().rev() {
                stack.push(c);
            }
        }
        out
    }

    /// Unlinks `idx` and its whole subtree.
    pub fn remove(&mut self, idx: usize) {
        if let Some(p) = self.nodes[idx].parent {
            self.nodes[p].children.retain(|&c| c != idx);
        }
        let mut stack = vec![idx];
        while let Some(n) = stack.pop() {
            self.nodes[n].alive = false;
            let children = std::mem::take(&mut self.nodes[n].children);
            stack.extend(children);
        }
    }

    /// Removes `idx`, splicing its children into its parent at its position.
    pub fn replace_with_children(&mut self, idx: usize) {
        let Some(p) = self.nodes[idx].parent else {
            return;
        };
        let children = std::mem::take(&mut self.nodes[idx].children);
        for &c in &children {
            self.nodes[c].parent = Some(p);
        }
        let pos = self.nodes[p]
            .children
            .iter()
            .position(|&c| c == idx)
            .unwrap_or(self.nodes[p].children.len());
        self.nodes[p].children.splice(pos..pos + 1, children);
        self.nodes[idx].alive = false;
        self.nodes[idx].parent = None;
    }

    /// Index of the `<body>` node, if any.
    pub fn body(&self) -> Option<usize> {
        self.nodes[0]
            .children
            .iter()
            .copied()
            .find(|&c| self.nodes[c].alive && self.nodes[c].kind == NodeKind::Body)
    }

    /// Whether `idx`'s children are laid out as flex/grid items.
    pub fn is_item_container(&self, idx: usize) -> bool {
        self.nodes[idx].style.display.is_flex_or_grid_container()
    }
}

/// Style of a text run under `parent`: inherited properties only, plus
/// the decoration propagated from ancestors.
pub fn text_style(parent: &ComputedStyle, deco: TextDecoration) -> ComputedStyle {
    let mut s = ComputedStyle::inherit_from(parent);
    s.text_decoration = deco;
    s
}

/// Style of an anonymous block wrapper under `parent`.
pub fn anonymous_style(parent: &ComputedStyle) -> ComputedStyle {
    let mut s = ComputedStyle::inherit_from(parent);
    s.display = Display::Block;
    s
}
