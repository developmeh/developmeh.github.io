//! Arena DOM filled by html5ever (see [`crate::sink`]) and read by the
//! cascade. Nodes are never freed individually; the whole arena is dropped
//! when the loader exits.

use std::cell::{Ref, RefCell};
use std::borrow::Cow;

use html5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::StrTendril;
use html5ever::{Attribute, QualName};

/// Index of a node in [`Dom::nodes`].
pub type NodeId = usize;

/// The kinds of node the arena stores.
#[derive(Clone, Debug)]
pub enum NodeData {
    /// The document node (always index 0).
    Document,
    /// A detached `<template>` contents fragment.
    Fragment,
    /// `<!doctype>`.
    Doctype,
    /// A comment (or processing instruction).
    Comment,
    /// A text run.
    Text(String),
    /// An element.
    Element {
        /// Qualified name (html5ever atoms).
        name: QualName,
        /// Attributes in source order, names lower-cased by the parser.
        attrs: Vec<(String, String)>,
        /// Template contents for `<template>`.
        template_contents: Option<NodeId>,
    },
}

/// One arena node.
#[derive(Clone, Debug)]
pub struct DomNode {
    /// Payload.
    pub data: NodeData,
    /// Parent index, `None` for the document and detached nodes.
    pub parent: Option<NodeId>,
    /// Children in order.
    pub children: Vec<NodeId>,
}

/// The parsed document.
#[derive(Debug, Default)]
pub struct Dom {
    /// All nodes; index 0 is the document.
    pub nodes: Vec<DomNode>,
    /// Whether the parser entered quirks mode.
    pub quirks: bool,
    /// Parse errors (informational).
    pub errors: Vec<String>,
}

impl Dom {
    /// An empty document.
    pub fn new() -> Dom {
        Dom {
            nodes: vec![DomNode {
                data: NodeData::Document,
                parent: None,
                children: Vec::new(),
            }],
            quirks: false,
            errors: Vec::new(),
        }
    }

    /// Parses a full HTML document (scripting disabled, so `<noscript>`
    /// contents are parsed as markup and rendered, plan §6.1).
    pub fn parse(html: &str) -> Dom {
        use html5ever::tendril::TendrilSink;
        let opts = html5ever::ParseOpts {
            tree_builder: html5ever::tree_builder::TreeBuilderOpts {
                scripting_enabled: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let sink = Sink {
            dom: RefCell::new(Dom::new()),
        };
        html5ever::parse_document(sink, opts).one(html)
    }

    fn push(&mut self, data: NodeData) -> NodeId {
        self.nodes.push(DomNode {
            data,
            parent: None,
            children: Vec::new(),
        });
        self.nodes.len() - 1
    }

    /// Lower-case local name of an element, or `None` for other nodes.
    pub fn tag(&self, id: NodeId) -> Option<&str> {
        match &self.nodes[id].data {
            NodeData::Element { name, .. } => Some(&name.local),
            _ => None,
        }
    }

    /// Whether the node is an element in the HTML namespace.
    pub fn is_html_element(&self, id: NodeId) -> bool {
        match &self.nodes[id].data {
            NodeData::Element { name, .. } => name.ns == html5ever::ns!(html),
            _ => false,
        }
    }

    /// Attributes of an element (empty for other nodes).
    pub fn attrs(&self, id: NodeId) -> &[(String, String)] {
        match &self.nodes[id].data {
            NodeData::Element { attrs, .. } => attrs,
            _ => &[],
        }
    }

    /// Value of an attribute by lower-case name.
    pub fn attr(&self, id: NodeId, key: &str) -> Option<&str> {
        self.attrs(id)
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Text of a text node.
    pub fn text(&self, id: NodeId) -> Option<&str> {
        match &self.nodes[id].data {
            NodeData::Text(t) => Some(t),
            _ => None,
        }
    }

    /// Whether the node is an element.
    pub fn is_element(&self, id: NodeId) -> bool {
        matches!(self.nodes[id].data, NodeData::Element { .. })
    }

    /// Concatenated text of all descendant text nodes.
    pub fn text_content(&self, id: NodeId) -> String {
        let mut out = String::new();
        let mut stack = vec![id];
        // Iterative pre-order; children pushed in reverse to keep order.
        while let Some(n) = stack.pop() {
            match &self.nodes[n].data {
                NodeData::Text(t) => out.push_str(t),
                NodeData::Element { .. } | NodeData::Document | NodeData::Fragment => {
                    for &c in self.nodes[n].children.iter().rev() {
                        stack.push(c);
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Element children only.
    pub fn element_children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes[id]
            .children
            .iter()
            .copied()
            .filter(move |&c| self.is_element(c))
    }

    /// The first `<tag>` element in document order, if any.
    pub fn find_first(&self, tag: &str) -> Option<NodeId> {
        self.descendants(0).find(|&n| self.tag(n) == Some(tag))
    }

    /// Pre-order traversal of the subtree rooted at `id` (excluding
    /// template contents).
    pub fn descendants(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let mut stack = vec![id];
        std::iter::from_fn(move || {
            let n = stack.pop()?;
            for &c in self.nodes[n].children.iter().rev() {
                stack.push(c);
            }
            Some(n)
        })
    }

    /// Index of `id` among its parent's children.
    pub fn sibling_index(&self, id: NodeId) -> Option<usize> {
        let p = self.nodes[id].parent?;
        self.nodes[p].children.iter().position(|&c| c == id)
    }

    /// Serializes the subtree at `id` back to markup (used for inline SVG
    /// blobs). Not a conforming HTML serializer: attributes are quoted and
    /// escaped, text is escaped, void elements are not special-cased.
    pub fn serialize(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.serialize_into(id, &mut out);
        out
    }

    fn serialize_into(&self, id: NodeId, out: &mut String) {
        enum Op {
            Open(NodeId),
            Close(NodeId),
        }
        let mut stack = vec![Op::Open(id)];
        while let Some(op) = stack.pop() {
            match op {
                Op::Open(n) => match &self.nodes[n].data {
                    NodeData::Text(t) => out.push_str(&escape(t, false)),
                    NodeData::Element { name, attrs, .. } => {
                        out.push('<');
                        out.push_str(&name.local);
                        for (k, v) in attrs {
                            out.push(' ');
                            out.push_str(k);
                            out.push_str("=\"");
                            out.push_str(&escape(v, true));
                            out.push('"');
                        }
                        out.push('>');
                        stack.push(Op::Close(n));
                        for &c in self.nodes[n].children.iter().rev() {
                            stack.push(Op::Open(c));
                        }
                    }
                    _ => {}
                },
                Op::Close(n) => {
                    if let NodeData::Element { name, .. } = &self.nodes[n].data {
                        out.push_str("</");
                        out.push_str(&name.local);
                        out.push('>');
                    }
                }
            }
        }
    }

    fn detach(&mut self, id: NodeId) {
        if let Some(p) = self.nodes[id].parent.take() {
            self.nodes[p].children.retain(|&c| c != id);
        }
    }

    fn append_text_or_node(&mut self, parent: NodeId, at: Option<usize>, child: NodeOrText<NodeId>) {
        match child {
            NodeOrText::AppendNode(n) => {
                self.detach(n);
                self.nodes[n].parent = Some(parent);
                match at {
                    Some(i) => self.nodes[parent].children.insert(i, n),
                    None => self.nodes[parent].children.push(n),
                }
            }
            NodeOrText::AppendText(t) => {
                // Merge with the previous sibling text node, if any.
                let prev = match at {
                    Some(0) => None,
                    Some(i) => self.nodes[parent].children.get(i - 1).copied(),
                    None => self.nodes[parent].children.last().copied(),
                };
                if let Some(prev) = prev {
                    if let NodeData::Text(existing) = &mut self.nodes[prev].data {
                        existing.push_str(&t);
                        return;
                    }
                }
                let n = self.push(NodeData::Text(t.to_string()));
                self.nodes[n].parent = Some(parent);
                match at {
                    Some(i) => self.nodes[parent].children.insert(i, n),
                    None => self.nodes[parent].children.push(n),
                }
            }
        }
    }
}

fn escape(s: &str, attr: bool) -> Cow<'_, str> {
    if !s.contains(['&', '<', '>', '"']) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// html5ever tree sink writing into a [`Dom`].
struct Sink {
    dom: RefCell<Dom>,
}

impl TreeSink for Sink {
    type Handle = NodeId;
    type Output = Dom;
    type ElemName<'a>
        = Ref<'a, QualName>
    where
        Self: 'a;

    fn finish(self) -> Dom {
        self.dom.into_inner()
    }

    fn parse_error(&self, msg: Cow<'static, str>) {
        let mut dom = self.dom.borrow_mut();
        if dom.errors.len() < 64 {
            dom.errors.push(msg.into_owned());
        }
    }

    fn get_document(&self) -> NodeId {
        0
    }

    fn elem_name<'a>(&'a self, target: &'a NodeId) -> Ref<'a, QualName> {
        Ref::map(self.dom.borrow(), |dom| match &dom.nodes[*target].data {
            NodeData::Element { name, .. } => name,
            _ => panic!("elem_name on a non-element"),
        })
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> NodeId {
        let mut dom = self.dom.borrow_mut();
        let template_contents = flags.template.then(|| dom.push(NodeData::Fragment));
        let attrs = attrs
            .into_iter()
            .map(|a| (a.name.local.to_string(), a.value.to_string()))
            .collect();
        dom.push(NodeData::Element {
            name,
            attrs,
            template_contents,
        })
    }

    fn create_comment(&self, _text: StrTendril) -> NodeId {
        self.dom.borrow_mut().push(NodeData::Comment)
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> NodeId {
        self.dom.borrow_mut().push(NodeData::Comment)
    }

    fn append(&self, parent: &NodeId, child: NodeOrText<NodeId>) {
        self.dom.borrow_mut().append_text_or_node(*parent, None, child);
    }

    fn append_based_on_parent_node(
        &self,
        element: &NodeId,
        prev_element: &NodeId,
        child: NodeOrText<NodeId>,
    ) {
        let has_parent = self.dom.borrow().nodes[*element].parent.is_some();
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(&self, _n: StrTendril, _p: StrTendril, _s: StrTendril) {
        let mut dom = self.dom.borrow_mut();
        let id = dom.push(NodeData::Doctype);
        dom.nodes[id].parent = Some(0);
        dom.nodes[0].children.push(id);
    }

    fn get_template_contents(&self, target: &NodeId) -> NodeId {
        match &self.dom.borrow().nodes[*target].data {
            NodeData::Element {
                template_contents: Some(c),
                ..
            } => *c,
            _ => panic!("get_template_contents on a non-template"),
        }
    }

    fn same_node(&self, x: &NodeId, y: &NodeId) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, mode: QuirksMode) {
        self.dom.borrow_mut().quirks = matches!(mode, QuirksMode::Quirks);
    }

    fn append_before_sibling(&self, sibling: &NodeId, new_node: NodeOrText<NodeId>) {
        let mut dom = self.dom.borrow_mut();
        let Some(parent) = dom.nodes[*sibling].parent else {
            return;
        };
        let at = dom.nodes[parent]
            .children
            .iter()
            .position(|&c| c == *sibling)
            .unwrap_or(dom.nodes[parent].children.len());
        dom.append_text_or_node(parent, Some(at), new_node);
    }

    fn add_attrs_if_missing(&self, target: &NodeId, new_attrs: Vec<Attribute>) {
        let mut dom = self.dom.borrow_mut();
        if let NodeData::Element { attrs, .. } = &mut dom.nodes[*target].data {
            for a in new_attrs {
                let k = a.name.local.to_string();
                if !attrs.iter().any(|(existing, _)| *existing == k) {
                    attrs.push((k, a.value.to_string()));
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &NodeId) {
        self.dom.borrow_mut().detach(*target);
    }

    fn reparent_children(&self, node: &NodeId, new_parent: &NodeId) {
        let mut dom = self.dom.borrow_mut();
        let children = std::mem::take(&mut dom.nodes[*node].children);
        for &c in &children {
            dom.nodes[c].parent = Some(*new_parent);
        }
        dom.nodes[*new_parent].children.extend(children);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_document() {
        let dom = Dom::parse("<!doctype html><title>T</title><p class=a>Hi <b>there</b></p>");
        let html = dom.find_first("html").unwrap();
        assert_eq!(dom.nodes[html].parent, Some(0));
        let p = dom.find_first("p").unwrap();
        assert_eq!(dom.attr(p, "class"), Some("a"));
        assert_eq!(dom.text_content(p), "Hi there");
        let title = dom.find_first("title").unwrap();
        assert_eq!(dom.text_content(title), "T");
        assert!(!dom.quirks);
        assert!(dom.is_html_element(p));
    }

    #[test]
    fn quirks_and_noscript() {
        let dom = Dom::parse("<p>x<noscript><b>y</b></noscript>");
        assert!(dom.quirks);
        let b = dom.find_first("b").unwrap();
        assert_eq!(dom.tag(dom.nodes[b].parent.unwrap()), Some("noscript"));
    }

    #[test]
    fn adjacent_text_is_merged_and_serialized() {
        let dom = Dom::parse("<svg viewBox=\"0 0 1 1\"><rect/></svg>a&amp;b");
        let svg = dom.find_first("svg").unwrap();
        assert_eq!(
            dom.serialize(svg),
            "<svg viewBox=\"0 0 1 1\"><rect></rect></svg>"
        );
        let body = dom.find_first("body").unwrap();
        let texts: Vec<_> = dom.nodes[body]
            .children
            .iter()
            .filter_map(|&c| dom.text(c))
            .collect();
        assert_eq!(texts, vec!["a&b"]);
    }

    #[test]
    fn template_contents_are_detached() {
        let dom = Dom::parse("<template><p>t</p></template><p>v</p>");
        let ps: Vec<_> = dom
            .descendants(0)
            .filter(|&n| dom.tag(n) == Some("p"))
            .collect();
        assert_eq!(ps.len(), 1);
        assert_eq!(dom.text_content(ps[0]), "v");
    }
}
