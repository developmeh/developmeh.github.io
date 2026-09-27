//! `parcel_selectors::Element` for arena DOM nodes, so lightningcss
//! selectors match against [`crate::dom::Dom`] directly.
//!
//! Plan §5: `:hover`, `:focus`, `:active`, `:visited`, `:target`, `:has()`
//! and the other dynamic pseudo-classes never match.

use lightningcss::selector::{PseudoClass, PseudoElement};
use parcel_selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use parcel_selectors::context::MatchingContext;
use parcel_selectors::matching::ElementSelectorFlags;
use parcel_selectors::{Element, OpaqueElement};

use crate::dom::{Dom, NodeData, NodeId};

/// lightningcss keeps its `SelectorImpl` type private; this projection
/// names it without spelling it.
pub trait ImplOf {
    /// The `SelectorImpl` parameter.
    type Impl;
}
impl<'i, I: parcel_selectors::SelectorImpl<'i>> ImplOf
    for parcel_selectors::parser::Selector<'i, I>
{
    type Impl = I;
}
/// The `SelectorImpl` lightningcss selectors are parsed with.
pub type SelImpl = <lightningcss::selector::Selector<'static> as ImplOf>::Impl;

/// Which pseudo-element a rule or a match is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoKind {
    /// `::before`
    Before,
    /// `::after`
    After,
}

impl PseudoKind {
    /// Whether a parsed pseudo-element is this kind.
    pub fn accepts(self, pe: &PseudoElement<'_>) -> bool {
        matches!(
            (self, pe),
            (PseudoKind::Before, PseudoElement::Before) | (PseudoKind::After, PseudoElement::After)
        )
    }
}

/// A borrowed element reference used for selector matching.
#[derive(Clone, Copy, Debug)]
pub struct ElRef<'d> {
    /// The document.
    pub dom: &'d Dom,
    /// The element.
    pub id: NodeId,
}

impl<'d> ElRef<'d> {
    /// An element reference for normal matching.
    pub fn new(dom: &'d Dom, id: NodeId) -> Self {
        ElRef { dom, id }
    }

    fn with(&self, id: NodeId) -> Self {
        ElRef { dom: self.dom, id }
    }

    fn tag(&self) -> &'d str {
        self.dom.tag(self.id).unwrap_or("")
    }

    fn ns(&self) -> &'d str {
        match &self.dom.nodes[self.id].data {
            NodeData::Element { name, .. } => &name.ns,
            _ => "",
        }
    }

    fn siblings(&self) -> Option<(&'d [NodeId], usize)> {
        let parent = self.dom.nodes[self.id].parent?;
        let siblings = self.dom.nodes[parent].children.as_slice();
        let pos = siblings.iter().position(|&c| c == self.id)?;
        Some((siblings, pos))
    }
}

impl<'i, 'd> Element<'i> for ElRef<'d> {
    type Impl = SelImpl;

    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(&self.dom.nodes[self.id])
    }

    fn parent_element(&self) -> Option<Self> {
        let p = self.dom.nodes[self.id].parent?;
        self.dom.is_element(p).then(|| self.with(p))
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        let (siblings, pos) = self.siblings()?;
        siblings[..pos]
            .iter()
            .rev()
            .copied()
            .find(|&c| self.dom.is_element(c))
            .map(|c| self.with(c))
    }

    fn next_sibling_element(&self) -> Option<Self> {
        let (siblings, pos) = self.siblings()?;
        siblings[pos + 1..]
            .iter()
            .copied()
            .find(|&c| self.dom.is_element(c))
            .map(|c| self.with(c))
    }

    fn is_html_element_in_html_document(&self) -> bool {
        self.dom.is_html_element(self.id)
    }

    fn has_local_name(&self, local_name: &lightningcss::values::ident::Ident<'i>) -> bool {
        self.tag().eq_ignore_ascii_case(&local_name.0)
    }

    fn has_namespace(&self, ns: &lightningcss::values::string::CowArcStr<'i>) -> bool {
        self.ns() == &**ns
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.tag() == other.tag() && self.ns() == other.ns()
    }

    fn attr_matches(
        &self,
        _ns: &NamespaceConstraint<&lightningcss::values::string::CowArcStr<'i>>,
        local_name: &lightningcss::values::ident::Ident<'i>,
        operation: &AttrSelectorOperation<&lightningcss::values::string::CSSString<'i>>,
    ) -> bool {
        let Some(value) = self
            .dom
            .attrs(self.id)
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&local_name.0))
            .map(|(_, v)| v.as_str())
        else {
            return false;
        };
        match operation {
            AttrSelectorOperation::Exists => true,
            AttrSelectorOperation::WithValue {
                operator,
                case_sensitivity,
                expected_value,
            } => operator.eval_str(value, &expected_value.0, *case_sensitivity),
        }
    }

    fn match_non_ts_pseudo_class<F>(
        &self,
        pc: &PseudoClass<'i>,
        _context: &mut MatchingContext<'_, 'i, Self::Impl>,
        _flags_setter: &mut F,
    ) -> bool
    where
        F: FnMut(&Self, ElementSelectorFlags),
    {
        match pc {
            PseudoClass::Link | PseudoClass::AnyLink(_) => self.is_link(),
            PseudoClass::Enabled => {
                is_form_control(self.tag()) && self.dom.attr(self.id, "disabled").is_none()
            }
            PseudoClass::Disabled => {
                is_form_control(self.tag()) && self.dom.attr(self.id, "disabled").is_some()
            }
            PseudoClass::Checked => {
                self.dom.attr(self.id, "checked").is_some()
                    || (self.tag() == "option" && self.dom.attr(self.id, "selected").is_some())
            }
            PseudoClass::Required => self.dom.attr(self.id, "required").is_some(),
            PseudoClass::Optional => {
                is_form_control(self.tag()) && self.dom.attr(self.id, "required").is_none()
            }
            PseudoClass::Lang { languages, .. } => {
                let mut n = Some(self.id);
                while let Some(id) = n {
                    if let Some(lang) = self.dom.attr(id, "lang") {
                        return languages.iter().any(|l| {
                            let l: &str = l;
                            lang.eq_ignore_ascii_case(l)
                                || (lang.len() > l.len()
                                    && lang[..l.len()].eq_ignore_ascii_case(l)
                                    && lang.as_bytes()[l.len()] == b'-')
                        });
                    }
                    n = self.dom.nodes[id].parent;
                }
                false
            }
            // Dynamic/user-state pseudo-classes never match (plan §5).
            _ => false,
        }
    }

    fn match_pseudo_element(
        &self,
        _pe: &PseudoElement<'i>,
        _context: &mut MatchingContext<'_, 'i, Self::Impl>,
    ) -> bool {
        // Stateless pseudo-element matching goes through
        // `MatchingContext::pseudo_element_matching_fn` (see the cascade).
        false
    }

    fn is_link(&self) -> bool {
        matches!(self.tag(), "a" | "area") && self.dom.attr(self.id, "href").is_some()
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(
        &self,
        id: &lightningcss::values::ident::Ident<'i>,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        self.dom
            .attr(self.id, "id")
            .is_some_and(|v| case_sensitivity.eq(v.as_bytes(), id.0.as_bytes()))
    }

    fn has_class(
        &self,
        name: &lightningcss::values::ident::Ident<'i>,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        self.dom.attr(self.id, "class").is_some_and(|v| {
            v.split_ascii_whitespace()
                .any(|c| case_sensitivity.eq(c.as_bytes(), name.0.as_bytes()))
        })
    }

    fn imported_part(
        &self,
        _name: &lightningcss::values::ident::Ident<'i>,
    ) -> Option<lightningcss::values::ident::Ident<'i>> {
        None
    }

    fn is_part(&self, _name: &lightningcss::values::ident::Ident<'i>) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        self.dom.nodes[self.id]
            .children
            .iter()
            .all(|&c| match &self.dom.nodes[c].data {
                NodeData::Element { .. } => false,
                NodeData::Text(t) => t.is_empty(),
                _ => true,
            })
    }

    fn is_root(&self) -> bool {
        self.dom.nodes[self.id]
            .parent
            .is_some_and(|p| matches!(self.dom.nodes[p].data, NodeData::Document))
    }
}

/// Whether the matcher can evaluate this selector. `:has()`, `&` nesting,
/// `::slotted()`, `::part()` and `:host` are unsupported (plan §5) and
/// would panic inside parcel_selectors, so their rules are dropped.
pub fn selector_supported(sel: &lightningcss::selector::Selector<'_>) -> bool {
    use parcel_selectors::parser::Component;
    sel.iter_raw_match_order().all(|c| match c {
        Component::Has(_)
        | Component::Nesting
        | Component::Slotted(_)
        | Component::Part(_)
        | Component::Host(_) => false,
        Component::Negation(list)
        | Component::Is(list)
        | Component::Where(list)
        | Component::Any(_, list) => list.iter().all(selector_supported),
        Component::NthOf(data) => data.selectors().iter().all(selector_supported),
        _ => true,
    })
}

fn is_form_control(tag: &str) -> bool {
    matches!(
        tag,
        "input" | "button" | "select" | "textarea" | "option" | "optgroup" | "fieldset"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightningcss::stylesheet::{ParserOptions, StyleSheet};
    use parcel_selectors::context::{MatchingMode, QuirksMode};
    use parcel_selectors::matching::matches_selector;
    use parcel_selectors::NthIndexCache;

    fn matches(dom: &Dom, id: NodeId, selector: &str, pseudo: Option<PseudoKind>) -> bool {
        let css = format!("{selector} {{ color: red }}");
        let sheet = StyleSheet::parse(&css, ParserOptions::default()).expect("selector parses");
        let lightningcss::rules::CssRule::Style(rule) = &sheet.rules.0[0] else {
            panic!("not a style rule");
        };
        let sel = &rule.selectors.0[0];
        if !selector_supported(sel) {
            return false;
        }
        let mut cache = NthIndexCache::default();
        let mode = if sel.pseudo_element().is_some() {
            MatchingMode::ForStatelessPseudoElement
        } else {
            MatchingMode::Normal
        };
        let mut ctx = MatchingContext::new(mode, None, Some(&mut cache), QuirksMode::NoQuirks);
        let hook = move |pe: &PseudoElement<'_>| pseudo.is_some_and(|p| p.accepts(pe));
        ctx.pseudo_element_matching_fn = Some(&hook);
        let el = ElRef::new(dom, id);
        matches_selector(sel, 0, None, &el, &mut ctx, &mut |_, _| {})
    }

    #[test]
    fn selector_subset() {
        let dom = Dom::parse(
            r##"<html lang="en"><body>
            <div id="main" class="Wrap  big" data-x="Foo bar">
              <p>one</p><p class="mid">two</p><p><a href="#">three</a></p><p></p>
              <ul><li>a</li><li>b</li><li>c</li></ul>
              <input type="text" disabled>
            </div></body></html>"##,
        );
        let div = dom.find_first("div").unwrap();
        let ps: Vec<_> = dom
            .descendants(0)
            .filter(|&n| dom.tag(n) == Some("p"))
            .collect();
        let a = dom.find_first("a").unwrap();
        let lis: Vec<_> = dom
            .descendants(0)
            .filter(|&n| dom.tag(n) == Some("li"))
            .collect();
        let input = dom.find_first("input").unwrap();
        let html = dom.find_first("html").unwrap();

        assert!(matches(&dom, div, "div", None));
        assert!(matches(&dom, div, "DIV", None));
        assert!(matches(&dom, div, "*", None));
        assert!(matches(&dom, div, "#main", None));
        assert!(matches(&dom, div, ".big", None));
        assert!(matches(&dom, div, ".Wrap.big", None));
        assert!(!matches(&dom, div, ".wrap", None));
        assert!(matches(&dom, div, "[data-x]", None));
        assert!(matches(&dom, div, "[data-x=\"Foo bar\"]", None));
        assert!(matches(&dom, div, "[data-x=\"foo bar\" i]", None));
        assert!(!matches(&dom, div, "[data-x=\"foo bar\"]", None));
        assert!(matches(&dom, div, "[data-x~=bar]", None));
        assert!(matches(&dom, div, "[data-x^=Foo]", None));
        assert!(matches(&dom, div, "[data-x$=bar]", None));
        assert!(matches(&dom, div, "[data-x*=\"o b\"]", None));
        assert!(matches(&dom, html, "[lang|=en]", None));
        assert!(matches(&dom, ps[0], "div p", None));
        assert!(matches(&dom, ps[0], "div > p", None));
        assert!(!matches(&dom, ps[0], "body > p", None));
        assert!(matches(&dom, ps[1], "p + p", None));
        assert!(!matches(&dom, ps[0], "p + p", None));
        assert!(matches(&dom, ps[2], ".mid ~ p", None));
        assert!(matches(&dom, html, ":root", None));
        assert!(!matches(&dom, div, ":root", None));
        assert!(matches(&dom, ps[0], "p:first-child", None));
        assert!(matches(&dom, input, "input:last-child", None));
        assert!(!matches(&dom, ps[3], "p:last-child", None));
        assert!(!matches(&dom, ps[3], "p:only-child", None));
        assert!(matches(&dom, a, "a:only-child", None));
        assert!(matches(&dom, lis[1], "li:nth-child(2)", None));
        assert!(matches(&dom, lis[0], "li:nth-child(odd)", None));
        assert!(matches(&dom, lis[2], "li:nth-child(2n+1)", None));
        assert!(matches(&dom, ps[1], "p:nth-of-type(2)", None));
        assert!(matches(&dom, ps[0], "p:not(.mid)", None));
        assert!(!matches(&dom, ps[1], "p:not(.mid)", None));
        assert!(matches(&dom, ps[1], "p:is(.mid, .other)", None));
        assert!(matches(&dom, ps[1], ":where(.mid)", None));
        assert!(matches(&dom, a, "a:link", None));
        assert!(matches(&dom, a, ":any-link", None));
        assert!(!matches(&dom, a, "a:visited", None));
        assert!(!matches(&dom, a, "a:hover", None));
        assert!(!matches(&dom, div, "div:has(p)", None));
        assert!(matches(&dom, ps[3], "p:empty", None));
        assert!(!matches(&dom, ps[0], "p:empty", None));
        assert!(matches(&dom, input, "input:disabled", None));
        assert!(!matches(&dom, input, "input:enabled", None));
        assert!(matches(&dom, ps[0], "p:lang(en)", None));
        assert!(!matches(&dom, ps[0], "p:lang(fr)", None));
        // Pseudo-elements only match when asked for.
        assert!(matches(&dom, ps[0], "p::before", Some(PseudoKind::Before)));
        assert!(!matches(&dom, ps[0], "p::before", Some(PseudoKind::After)));
        assert!(!matches(&dom, ps[0], "p::before", None));
    }
}
