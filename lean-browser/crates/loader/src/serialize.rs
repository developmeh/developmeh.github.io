//! Frozen tree -> [`page_format::Page`]: pre-order numbering, one text
//! blob shared by runs, names and attribute values (identical strings
//! alias, so a name equal to a node's text costs no extra bytes), sorted
//! attribute table, image/form tables and the top-level block list.

use std::collections::HashMap;

use css_subset::{ComputedStyle, GridTrack};
use page_format::{Attr, Form, ImageRef, Node, NodeKind, Page, NONE};

use crate::tree::FTree;

/// Everything the page needs besides the tree.
pub struct PageInputs {
    /// URL after redirects.
    pub final_url: String,
    /// Viewport-width bucket.
    pub viewport_width: u16,
    /// Sorted breakpoints.
    pub breakpoints: Vec<u16>,
    /// Interned styles (pass 5).
    pub styles: Vec<ComputedStyle>,
    /// Grid track table.
    pub tracks: Vec<GridTrack>,
    /// Image blobs (pass 6).
    pub blobs: Vec<u8>,
}

/// Interns strings into the text blob.
#[derive(Default)]
struct Interner {
    blob: Vec<u8>,
    seen: HashMap<String, (u32, u32)>,
}

impl Interner {
    fn intern(&mut self, s: &str) -> (u32, u32) {
        if s.is_empty() {
            return (0, 0);
        }
        if let Some(&r) = self.seen.get(s) {
            return r;
        }
        let off = self.blob.len() as u32;
        self.blob.extend_from_slice(s.as_bytes());
        let r = (off, s.len() as u32);
        self.seen.insert(s.to_string(), r);
        r
    }
}

/// Builds the page. The tree must have gone through all seven passes.
pub fn to_page(tree: &FTree, inputs: PageInputs) -> Page {
    let order = tree.pre_order();
    // Map arena index -> page index.
    let mut index: Vec<u32> = vec![NONE; tree.nodes.len()];
    for (i, &n) in order.iter().enumerate() {
        index[n] = i as u32;
    }

    let mut text = Interner::default();
    let mut nodes: Vec<Node> = Vec::with_capacity(order.len());
    let mut attrs: Vec<Attr> = Vec::new();
    let mut images: Vec<ImageRef> = Vec::new();
    let mut form_nodes: Vec<(usize, u32)> = Vec::new(); // (form index, page node)
    let mut fields_by_form: Vec<Vec<u32>> = vec![Vec::new(); tree.forms.len()];

    for (i, &n) in order.iter().enumerate() {
        let fnode = &tree.nodes[n];
        let page_idx = i as u32;
        let parent = fnode.parent.map(|p| index[p]).unwrap_or(NONE);
        let first_child = fnode
            .children
            .iter()
            .copied()
            .find(|&c| tree.nodes[c].alive)
            .map(|c| index[c])
            .unwrap_or(NONE);
        let next_sibling = fnode
            .parent
            .and_then(|p| {
                let siblings = &tree.nodes[p].children;
                let pos = siblings.iter().position(|&c| c == n)?;
                siblings[pos + 1..]
                    .iter()
                    .copied()
                    .find(|&c| tree.nodes[c].alive)
            })
            .map(|c| index[c])
            .unwrap_or(NONE);

        let (text_off, text_len) = if fnode.is_textual() {
            text.intern(&fnode.text)
        } else {
            (0, 0)
        };
        let (name_off, name_len) = text.intern(&fnode.name);

        let mut node = Node::new(fnode.kind, fnode.role, fnode.style_id, parent);
        node.flags = fnode.flags.bits();
        node.text_off = text_off;
        node.text_len = text_len;
        node.first_child = first_child;
        node.next_sibling = next_sibling;
        node.name_off = name_off;
        node.name_len = name_len.min(u16::MAX as u32) as u16;
        nodes.push(node);

        for (key, value) in &fnode.kept {
            let (val_off, val_len) = text.intern(value);
            attrs.push(Attr {
                node: page_idx,
                val_off,
                val_len,
                key: *key,
            });
        }
        if let Some(img) = fnode.image.and_then(|i| tree.images.get(i)) {
            let (blob_off, blob_len) = img.blob.unwrap_or((0, 0));
            images.push(ImageRef {
                node: page_idx,
                blob_off,
                blob_len,
                width: img.info.width,
                height: img.info.height,
                format: img.info.format,
            });
        }
        if let Some(f) = fnode.form {
            form_nodes.push((f, page_idx));
        }
        if fnode.flags.contains(css_subset::NodeFlags::IS_FORM_FIELD) {
            // Nearest live <form> ancestor.
            let mut a = fnode.parent;
            while let Some(p) = a {
                if let Some(f) = tree.nodes[p].form {
                    fields_by_form[f].push(page_idx);
                    break;
                }
                a = tree.nodes[p].parent;
            }
        }
    }

    let mut forms = Vec::with_capacity(form_nodes.len());
    let mut fields: Vec<u32> = Vec::new();
    for (f, node) in form_nodes {
        let data = &tree.forms[f];
        let (action_off, action_len) = text.intern(&data.action);
        let first_field = fields.len() as u32;
        fields.extend_from_slice(&fields_by_form[f]);
        forms.push(Form {
            node,
            action_off,
            action_len,
            first_field,
            field_count: fields_by_form[f].len() as u32,
            method: data.method,
        });
    }

    // Top-level blocks: the body's live children (freeze guaranteed they
    // are block-level); without a body, the root's children.
    let container = tree.body().unwrap_or(0);
    let top_level: Vec<u32> = tree.nodes[container]
        .children
        .iter()
        .copied()
        .filter(|&c| tree.nodes[c].alive)
        .map(|c| index[c])
        .collect();

    Page {
        final_url: inputs.final_url,
        title: tree.title.clone(),
        viewport_width: inputs.viewport_width,
        breakpoints: inputs.breakpoints,
        nodes,
        text: text.blob,
        styles: inputs.styles,
        tracks: inputs.tracks,
        attrs,
        images,
        blobs: inputs.blobs,
        forms,
        fields,
        top_level,
    }
}

/// Page node kinds that carry text (for consumers of the page).
pub fn is_text_kind(kind: NodeKind) -> bool {
    matches!(kind, NodeKind::Text | NodeKind::Pseudo | NodeKind::Marker)
}
