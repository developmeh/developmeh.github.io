//! Accessible role and name computation (plan §12), from tag, `role=`,
//! `aria-label`, `aria-labelledby`, `alt`, `<label for>`, heading level
//! and landmark tags. Runs on the DOM before compression so the a11y
//! structure is known when pass 3 decides what it may collapse.

use std::collections::HashMap;

use page_format::{NodeKind, Role};

use crate::dom::{Dom, NodeId};

/// Maps an HTML tag to the page-file node kind.
pub fn kind_for_tag(tag: &str) -> NodeKind {
    use NodeKind as K;
    match tag {
        "html" => K::Html,
        "body" => K::Body,
        "div" => K::Div,
        "p" => K::P,
        "span" => K::Span,
        "a" => K::A,
        "h1" => K::H1,
        "h2" => K::H2,
        "h3" => K::H3,
        "h4" => K::H4,
        "h5" => K::H5,
        "h6" => K::H6,
        "ul" | "menu" | "dir" => K::Ul,
        "ol" => K::Ol,
        "li" => K::Li,
        "dl" => K::Dl,
        "dt" => K::Dt,
        "dd" => K::Dd,
        "pre" | "listing" | "xmp" | "plaintext" => K::Pre,
        "code" | "tt" => K::Code,
        "blockquote" => K::Blockquote,
        "em" => K::Em,
        "strong" => K::Strong,
        "b" => K::B,
        "i" => K::I,
        "u" | "ins" => K::U,
        "s" | "strike" | "del" => K::S,
        "small" => K::Small,
        "sub" => K::Sub,
        "sup" => K::Sup,
        "hr" => K::Hr,
        "table" => K::Table,
        "thead" => K::Thead,
        "tbody" => K::Tbody,
        "tfoot" => K::Tfoot,
        "tr" => K::Tr,
        "td" => K::Td,
        "th" => K::Th,
        "caption" => K::Caption,
        "nav" => K::Nav,
        "main" => K::Main,
        "header" => K::Header,
        "footer" => K::Footer,
        "aside" => K::Aside,
        "article" => K::Article,
        "section" => K::Section,
        "figure" => K::Figure,
        "figcaption" => K::Figcaption,
        "form" => K::Form,
        "label" => K::Label,
        "fieldset" => K::Fieldset,
        "legend" => K::Legend,
        "details" => K::Details,
        "summary" => K::Summary,
        "iframe" => K::Iframe,
        "video" => K::Video,
        "audio" => K::Audio,
        "canvas" => K::Canvas,
        "object" | "embed" => K::Object,
        "noscript" => K::Noscript,
        "kbd" => K::Kbd,
        "samp" => K::Samp,
        "var" => K::Var,
        "q" => K::Q,
        "cite" => K::Cite,
        "abbr" | "acronym" | "dfn" => K::Abbr,
        "time" => K::Time,
        "mark" => K::Mark,
        "wbr" => K::Wbr,
        "br" => K::LineBreak,
        "img" => K::Image,
        "svg" => K::Svg,
        "input" | "textarea" | "select" | "button" => K::FormControl,
        _ => K::Element,
    }
}

/// Document-wide lookup tables for name computation.
pub struct A11y {
    /// `<label for="id">` text by id.
    labels_for: HashMap<String, String>,
    /// Element text by `id` (for `aria-labelledby`).
    text_by_id: HashMap<String, String>,
}

impl A11y {
    /// Scans the document once.
    pub fn new(dom: &Dom) -> A11y {
        let mut labels_for = HashMap::new();
        let mut text_by_id = HashMap::new();
        for id in dom.descendants(0) {
            if !dom.is_element(id) {
                continue;
            }
            if dom.tag(id) == Some("label") {
                if let Some(target) = dom.attr(id, "for") {
                    labels_for
                        .entry(target.to_string())
                        .or_insert_with(|| normalize(&dom.text_content(id)));
                }
            }
            if let Some(el_id) = dom.attr(id, "id") {
                text_by_id
                    .entry(el_id.to_string())
                    .or_insert_with(|| normalize(&dom.text_content(id)));
            }
        }
        A11y {
            labels_for,
            text_by_id,
        }
    }

    /// The role of element `id`.
    pub fn role(&self, dom: &Dom, id: NodeId) -> Role {
        if let Some(explicit) = dom.attr(id, "role").and_then(|r| aria_role(dom, id, r)) {
            return explicit;
        }
        let tag = dom.tag(id).unwrap_or("");
        match tag {
            "html" => Role::Document,
            "h1" => Role::Heading1,
            "h2" => Role::Heading2,
            "h3" => Role::Heading3,
            "h4" => Role::Heading4,
            "h5" => Role::Heading5,
            "h6" => Role::Heading6,
            "a" | "area" => {
                if dom.attr(id, "href").is_some() {
                    Role::Link
                } else {
                    Role::Generic
                }
            }
            "button" => Role::Button,
            "input" => match dom
                .attr(id, "type")
                .map(|t| t.trim().to_ascii_lowercase())
                .as_deref()
            {
                Some("submit") | Some("button") | Some("reset") | Some("image") => Role::Button,
                Some("checkbox") => Role::Checkbox,
                Some("radio") => Role::Radio,
                Some("hidden") => Role::Generic,
                Some("range") | Some("color") | Some("file") => Role::Generic,
                _ => Role::TextField,
            },
            "textarea" => Role::TextField,
            "select" => Role::ComboBox,
            "img" => {
                if dom.attr(id, "alt").is_some_and(|a| a.trim().is_empty()) {
                    Role::Presentation
                } else {
                    Role::Img
                }
            }
            "svg" => Role::Img,
            "ul" | "ol" | "menu" | "dir" => Role::List,
            "li" => Role::ListItem,
            "nav" => Role::Navigation,
            "main" => Role::Main,
            "header" => {
                if inside_sectioning(dom, id) {
                    Role::Generic
                } else {
                    Role::Banner
                }
            }
            "footer" => {
                if inside_sectioning(dom, id) {
                    Role::Generic
                } else {
                    Role::ContentInfo
                }
            }
            "aside" => Role::Complementary,
            "article" => Role::Article,
            "section" => {
                if has_aria_name(dom, id) {
                    Role::Region
                } else {
                    Role::Generic
                }
            }
            "table" => Role::Table,
            "tr" => Role::Row,
            "td" => Role::Cell,
            "th" => {
                if dom
                    .attr(id, "scope")
                    .is_some_and(|s| s.eq_ignore_ascii_case("row"))
                {
                    Role::RowHeader
                } else {
                    Role::ColumnHeader
                }
            }
            "code" | "pre" | "kbd" | "samp" | "tt" => Role::Code,
            "blockquote" => Role::Blockquote,
            "em" => Role::Emphasis,
            "strong" => Role::Strong,
            "hr" => Role::Separator,
            "figure" => Role::Figure,
            "figcaption" | "caption" => Role::Caption,
            "form" => Role::Form,
            "label" => Role::Label,
            "br" => Role::LineBreak,
            "p" => Role::Paragraph,
            "fieldset" | "details" | "optgroup" => Role::Group,
            _ => Role::Generic,
        }
    }

    /// The accessible name of element `id` (empty if none). Whitespace is
    /// normalized; the result is capped at 512 bytes.
    pub fn name(&self, dom: &Dom, id: NodeId, role: Role) -> String {
        let name = self.name_uncapped(dom, id, role);
        cap(name)
    }

    fn name_uncapped(&self, dom: &Dom, id: NodeId, role: Role) -> String {
        if let Some(l) = dom.attr(id, "aria-label") {
            let l = normalize(l);
            if !l.is_empty() {
                return l;
            }
        }
        if let Some(ids) = dom.attr(id, "aria-labelledby") {
            let joined: Vec<&str> = ids
                .split_ascii_whitespace()
                .filter_map(|i| self.text_by_id.get(i).map(|s| s.as_str()))
                .filter(|s| !s.is_empty())
                .collect();
            if !joined.is_empty() {
                return joined.join(" ");
            }
        }
        let tag = dom.tag(id).unwrap_or("");
        match tag {
            "img" | "area" => {
                if let Some(alt) = dom.attr(id, "alt") {
                    return normalize(alt);
                }
            }
            "input" | "textarea" | "select" | "button" => {
                if let Some(l) = dom.attr(id, "id").and_then(|i| self.labels_for.get(i)) {
                    if !l.is_empty() {
                        return l.clone();
                    }
                }
                if let Some(label) = ancestor(dom, id, "label") {
                    let l = normalize(&dom.text_content(label));
                    if !l.is_empty() {
                        return l;
                    }
                }
                if tag == "button" {
                    let l = normalize(&dom.text_content(id));
                    if !l.is_empty() {
                        return l;
                    }
                }
                if tag == "input" {
                    if let Some(v) = dom.attr(id, "value").filter(|_| role == Role::Button) {
                        return normalize(v);
                    }
                }
                if let Some(p) = dom.attr(id, "placeholder") {
                    return normalize(p);
                }
            }
            "svg" => {
                if let Some(t) = dom
                    .element_children(id)
                    .find(|&c| dom.tag(c) == Some("title"))
                {
                    return normalize(&dom.text_content(t));
                }
            }
            _ => {}
        }
        if matches!(
            role,
            Role::Link
                | Role::Button
                | Role::Heading1
                | Role::Heading2
                | Role::Heading3
                | Role::Heading4
                | Role::Heading5
                | Role::Heading6
                | Role::Label
                | Role::Caption
                | Role::ColumnHeader
                | Role::RowHeader
        ) {
            let l = normalize(&dom.text_content(id));
            if !l.is_empty() {
                return l;
            }
        }
        dom.attr(id, "title").map(normalize).unwrap_or_default()
    }
}

/// Maps an explicit `role=""` token to our enum (first recognised token).
fn aria_role(dom: &Dom, id: NodeId, value: &str) -> Option<Role> {
    for token in value.split_ascii_whitespace() {
        let r = match token.to_ascii_lowercase().as_str() {
            "heading" => match dom
                .attr(id, "aria-level")
                .and_then(|l| l.trim().parse::<u8>().ok())
            {
                Some(1) => Role::Heading1,
                Some(3) => Role::Heading3,
                Some(4) => Role::Heading4,
                Some(5) => Role::Heading5,
                Some(6) => Role::Heading6,
                _ => Role::Heading2,
            },
            "link" => Role::Link,
            "button" => Role::Button,
            "textbox" | "searchbox" => Role::TextField,
            "checkbox" | "switch" => Role::Checkbox,
            "radio" => Role::Radio,
            "combobox" | "listbox" => Role::ComboBox,
            "img" | "image" => Role::Img,
            "list" => Role::List,
            "listitem" => Role::ListItem,
            "navigation" => Role::Navigation,
            "main" => Role::Main,
            "banner" => Role::Banner,
            "contentinfo" => Role::ContentInfo,
            "complementary" => Role::Complementary,
            "article" => Role::Article,
            "region" => Role::Region,
            "table" | "grid" => Role::Table,
            "row" => Role::Row,
            "cell" | "gridcell" => Role::Cell,
            "columnheader" => Role::ColumnHeader,
            "rowheader" => Role::RowHeader,
            "code" => Role::Code,
            "blockquote" => Role::Blockquote,
            "emphasis" => Role::Emphasis,
            "strong" => Role::Strong,
            "separator" => Role::Separator,
            "figure" => Role::Figure,
            "caption" => Role::Caption,
            "form" | "search" => Role::Form,
            "presentation" | "none" => Role::Presentation,
            "group" | "radiogroup" => Role::Group,
            "paragraph" => Role::Paragraph,
            "document" => Role::Document,
            "generic" => Role::Generic,
            _ => continue,
        };
        return Some(r);
    }
    None
}

fn has_aria_name(dom: &Dom, id: NodeId) -> bool {
    dom.attr(id, "aria-label")
        .is_some_and(|l| !l.trim().is_empty())
        || dom
            .attr(id, "aria-labelledby")
            .is_some_and(|l| !l.trim().is_empty())
}

fn inside_sectioning(dom: &Dom, id: NodeId) -> bool {
    let mut n = dom.nodes[id].parent;
    while let Some(p) = n {
        if matches!(
            dom.tag(p),
            Some(
                "article"
                    | "aside"
                    | "main"
                    | "nav"
                    | "section"
                    | "blockquote"
                    | "details"
                    | "fieldset"
                    | "figure"
                    | "td"
            )
        ) {
            return true;
        }
        n = dom.nodes[p].parent;
    }
    false
}

fn ancestor(dom: &Dom, id: NodeId, tag: &str) -> Option<NodeId> {
    let mut n = dom.nodes[id].parent;
    while let Some(p) = n {
        if dom.tag(p) == Some(tag) {
            return Some(p);
        }
        n = dom.nodes[p].parent;
    }
    None
}

/// Collapses whitespace runs to single spaces and trims.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = true;
    for c in s.chars() {
        if c.is_whitespace() {
            if !space {
                out.push(' ');
                space = true;
            }
        } else {
            out.push(c);
            space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

fn cap(mut s: String) -> String {
    const MAX: usize = 512;
    if s.len() > MAX {
        let mut end = MAX;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_and_names() {
        let dom = Dom::parse(
            r##"<body>
            <header><nav aria-label="Main"><a href="/">Home  page</a><a>plain</a></nav></header>
            <main><article><header>inner</header>
              <h2 id="t">Title <em>x</em></h2>
              <section aria-labelledby="t">s</section><section>anon</section>
              <img src=a alt=""><img src=b alt="A cat"><img src=c title="tip">
              <form><label for="q">Search</label><input id="q" type="text">
                <label>Wrapped <input type="checkbox"></label>
                <input type="submit" value="Go"><input type="text" placeholder="ph">
                <button>  Press </button><select></select></form>
              <div role="button" aria-label="Menu">m</div>
              <div role="heading" aria-level="3">h</div>
              <table><tr><th scope=row>r</th><td>c</td></tr></table>
              <ul><li>i</li></ul><footer>f</footer>
            </article></main><footer>site</footer></body>"##,
        );
        let a = A11y::new(&dom);
        let find = |tag: &str, n: usize| {
            dom.descendants(0)
                .filter(|&i| dom.tag(i) == Some(tag))
                .nth(n)
                .unwrap()
        };
        let role = |id| a.role(&dom, id);
        let name = |id| a.name(&dom, id, a.role(&dom, id));

        assert_eq!(role(find("header", 0)), Role::Banner);
        assert_eq!(role(find("header", 1)), Role::Generic);
        assert_eq!(role(find("footer", 0)), Role::Generic);
        assert_eq!(role(find("footer", 1)), Role::ContentInfo);
        assert_eq!(role(find("nav", 0)), Role::Navigation);
        assert_eq!(name(find("nav", 0)), "Main");
        assert_eq!(role(find("a", 0)), Role::Link);
        assert_eq!(name(find("a", 0)), "Home page");
        assert_eq!(role(find("a", 1)), Role::Generic);
        assert_eq!(role(find("h2", 0)), Role::Heading2);
        assert_eq!(name(find("h2", 0)), "Title x");
        assert_eq!(role(find("section", 0)), Role::Region);
        assert_eq!(name(find("section", 0)), "Title x");
        assert_eq!(role(find("section", 1)), Role::Generic);
        assert_eq!(role(find("img", 0)), Role::Presentation);
        assert_eq!(role(find("img", 1)), Role::Img);
        assert_eq!(name(find("img", 1)), "A cat");
        assert_eq!(name(find("img", 2)), "tip");
        assert_eq!(role(find("input", 0)), Role::TextField);
        assert_eq!(name(find("input", 0)), "Search");
        assert_eq!(role(find("input", 1)), Role::Checkbox);
        assert_eq!(name(find("input", 1)), "Wrapped");
        assert_eq!(role(find("input", 2)), Role::Button);
        assert_eq!(name(find("input", 2)), "Go");
        assert_eq!(name(find("input", 3)), "ph");
        assert_eq!(role(find("button", 0)), Role::Button);
        assert_eq!(name(find("button", 0)), "Press");
        assert_eq!(role(find("select", 0)), Role::ComboBox);
        assert_eq!(role(find("div", 0)), Role::Button);
        assert_eq!(name(find("div", 0)), "Menu");
        assert_eq!(role(find("div", 1)), Role::Heading3);
        assert_eq!(name(find("div", 1)), "h");
        assert_eq!(role(find("th", 0)), Role::RowHeader);
        assert_eq!(role(find("td", 0)), Role::Cell);
        assert_eq!(role(find("ul", 0)), Role::List);
        assert_eq!(role(find("li", 0)), Role::ListItem);
        assert_eq!(name(find("li", 0)), "");
        assert_eq!(role(find("main", 0)), Role::Main);
        assert_eq!(role(find("html", 0)), Role::Document);
        assert_eq!(kind_for_tag("h3"), NodeKind::H3);
        assert_eq!(kind_for_tag("custom-el"), NodeKind::Element);
    }

    #[test]
    fn normalize_and_cap() {
        assert_eq!(normalize("  a \n\t b  "), "a b");
        let long = "x".repeat(600);
        assert_eq!(cap(long).len(), 512);
    }
}
