//! The cascade: for every element, gather matching declarations (UA and
//! author origins, `!important`, specificity, order, presentational hints,
//! `style=""`), resolve custom properties and `var()`, then compute a
//! [`ComputedStyle`] against the parent's computed style.
//!
//! [`Cascade::compute`] is a pure function of `(element, parent context)`,
//! which is what the compression style-preservation test relies on: after
//! compression every surviving node is recomputed against its *new* parent
//! and must come out identical (plan §6).

use std::collections::HashMap;
use std::rc::Rc;

use css_subset::{ComputedStyle, Display, Float, Length, Position};
use lightningcss::declaration::DeclarationBlock;
use lightningcss::printer::PrinterOptions;
use lightningcss::properties::custom::{
    CustomPropertyName, Token, TokenList, TokenOrValue, UnparsedProperty,
};
use lightningcss::properties::Property;
use lightningcss::stylesheet::StyleAttribute;
use parcel_selectors::context::{MatchingContext, MatchingMode, QuirksMode};
use parcel_selectors::matching::matches_selector;
use parcel_selectors::NthIndexCache;

use super::media::Viewport;
use super::rules::{parser_options, Origin, Rule, RuleSet};
use super::select::{ElRef, PseudoKind};
use super::values::{self, Apply, LenCtx, TrackTable};
use crate::dom::{Dom, NodeId};

/// Custom properties (`--x`) in scope for an element; inherited.
pub type CustomMap<'i> = HashMap<String, TokenList<'i>>;

/// What a child inherits from its parent besides the computed style.
#[derive(Clone, Debug)]
pub struct ElementCtx<'i> {
    /// Custom properties in scope.
    pub custom: Rc<CustomMap<'i>>,
    /// `line-height: <number>` factor, recomputed against each child's
    /// font size.
    pub line_height_factor: Option<f32>,
}

impl<'i> ElementCtx<'i> {
    /// The root's parent: nothing in scope.
    pub fn initial() -> Self {
        ElementCtx {
            custom: Rc::new(CustomMap::new()),
            line_height_factor: None,
        }
    }
}

/// The parent side of a computation.
#[derive(Clone, Debug)]
pub struct Inherited<'i> {
    /// Parent's computed style.
    pub style: ComputedStyle,
    /// Parent's context.
    pub ctx: ElementCtx<'i>,
}

impl<'i> Inherited<'i> {
    /// For the root element.
    pub fn initial() -> Self {
        Inherited {
            style: ComputedStyle::INITIAL,
            ctx: ElementCtx::initial(),
        }
    }
}

/// A `::before`/`::after` box with string content.
#[derive(Clone, Debug, PartialEq)]
pub struct Generated {
    /// Computed style of the pseudo-element.
    pub style: ComputedStyle,
    /// The `content` string.
    pub content: String,
}

/// Result of computing one element.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeStyle {
    /// The element's computed style.
    pub style: ComputedStyle,
    /// `::before`, if it has string content.
    pub before: Option<Generated>,
    /// `::after`, if it has string content.
    pub after: Option<Generated>,
}

/// Cascade state for one document. `'i` is the lifetime of both the DOM
/// and the stylesheet sources (custom property values borrow from either).
pub struct Cascade<'a, 'i> {
    dom: &'i Dom,
    rules: &'a RuleSet<'a, 'i>,
    vp: Viewport,
    root_font_size: f32,
    nth_cache: NthIndexCache,
    /// Grid track lists for `Page.tracks`.
    pub tracks: TrackTable,
    /// Computed styles by DOM node id (`None` for non-elements and for
    /// elements inside `display: none` subtrees).
    pub styles: Vec<Option<NodeStyle>>,
    /// Contexts by DOM node id.
    pub contexts: Vec<Option<ElementCtx<'i>>>,
}

/// Where a declaration block came from, in cascade precedence order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    UaNormal,
    Hints,
    AuthorNormal,
    StyleAttr,
    AuthorImportant,
    UaImportant,
}

impl<'a, 'i> Cascade<'a, 'i> {
    /// Prepares a cascade for `dom` with the flattened `rules`.
    pub fn new(dom: &'i Dom, rules: &'a RuleSet<'a, 'i>, vp: Viewport) -> Self {
        Cascade {
            dom,
            rules,
            vp,
            root_font_size: 16.0,
            nth_cache: NthIndexCache::default(),
            tracks: TrackTable::default(),
            styles: vec![None; dom.nodes.len()],
            contexts: vec![None; dom.nodes.len()],
        }
    }

    /// Computes every rendered element in document order. Subtrees of
    /// `display: none` elements are skipped (they are never rendered in
    /// document mode, plan §6.1).
    pub fn run(&mut self) {
        let Some(root) = self.dom.nodes[0]
            .children
            .iter()
            .copied()
            .find(|&c| self.dom.is_element(c))
        else {
            return;
        };
        let mut stack: Vec<(NodeId, Inherited<'i>)> = vec![(root, Inherited::initial())];
        while let Some((id, parent)) = stack.pop() {
            let (ns, ctx) = self.compute(id, &parent);
            if id == root {
                self.root_font_size = ns.style.font_size;
            }
            let hidden = ns.style.display == Display::None;
            let inherited = Inherited {
                style: ns.style,
                ctx: ctx.clone(),
            };
            self.styles[id] = Some(ns);
            self.contexts[id] = Some(ctx);
            if hidden {
                continue;
            }
            for &c in self.dom.nodes[id].children.iter().rev() {
                if self.dom.is_element(c) {
                    stack.push((c, inherited.clone()));
                }
            }
        }
    }

    /// The inherited context a child of `id` sees (after [`Self::run`]).
    pub fn inherited_of(&self, id: NodeId) -> Option<Inherited<'i>> {
        Some(Inherited {
            style: self.styles[id].as_ref()?.style,
            ctx: self.contexts[id].clone()?,
        })
    }

    /// Computes the style of element `id` against `parent`. Pure: calling
    /// it again with the same inputs yields the same output.
    pub fn compute(&mut self, id: NodeId, parent: &Inherited<'i>) -> (NodeStyle, ElementCtx<'i>) {
        let dom = self.dom;
        let tag = dom.tag(id).unwrap_or("");
        let candidates = self
            .rules
            .candidates(tag, dom.attr(id, "id"), dom.attr(id, "class").unwrap_or(""));

        // Selector matching.
        let mut matched: Vec<&Rule<'a, 'i>> = Vec::new();
        let mut matched_before: Vec<&Rule<'a, 'i>> = Vec::new();
        let mut matched_after: Vec<&Rule<'a, 'i>> = Vec::new();
        let quirks = if dom.quirks {
            QuirksMode::Quirks
        } else {
            QuirksMode::NoQuirks
        };
        let el = ElRef::new(dom, id);
        {
            let mut ctx =
                MatchingContext::new(MatchingMode::Normal, None, Some(&mut self.nth_cache), quirks);
            for rule in candidates.iter().filter(|r| r.pseudo.is_none()) {
                if matches_selector(rule.selector, 0, None, &el, &mut ctx, &mut |_, _| {}) {
                    matched.push(rule);
                }
            }
        }
        if self.rules.has_pseudo_rules {
            for (kind, out) in [
                (PseudoKind::Before, &mut matched_before),
                (PseudoKind::After, &mut matched_after),
            ] {
                let hook = move |pe: &lightningcss::selector::PseudoElement<'_>| kind.accepts(pe);
                let mut ctx = MatchingContext::new(
                    MatchingMode::ForStatelessPseudoElement,
                    None,
                    Some(&mut self.nth_cache),
                    quirks,
                );
                ctx.pseudo_element_matching_fn = Some(&hook);
                for rule in candidates.iter().filter(|r| r.pseudo == Some(kind)) {
                    if matches_selector(rule.selector, 0, None, &el, &mut ctx, &mut |_, _| {}) {
                        out.push(rule);
                    }
                }
            }
        }

        // Presentational hints and the style attribute.
        let hints_css = presentational_hints(dom, id);
        let hints = hints_css
            .as_deref()
            .and_then(|css| DeclarationBlock::parse_string(css, parser_options()).ok());
        let style_attr = dom
            .attr(id, "style")
            .and_then(|s| StyleAttribute::parse(s, parser_options()).ok());

        // Ordered declaration list, lowest precedence first. Hints are
        // generated locally and kept apart (they never define custom
        // properties, so they need not share the sheet lifetime).
        let mut decls: Vec<(Level, &Property<'i>)> = Vec::new();
        for rule in &matched {
            let (normal, important) = match rule.origin {
                Origin::Ua => (Level::UaNormal, Level::UaImportant),
                Origin::Author => (Level::AuthorNormal, Level::AuthorImportant),
            };
            decls.extend(rule.decls.declarations.iter().map(|p| (normal, p)));
            decls.extend(rule.decls.important_declarations.iter().map(|p| (important, p)));
        }
        if let Some(s) = &style_attr {
            decls.extend(s.declarations.declarations.iter().map(|p| (Level::StyleAttr, p)));
            decls.extend(
                s.declarations
                    .important_declarations
                    .iter()
                    .map(|p| (Level::AuthorImportant, p)),
            );
        }
        // Stable sort keeps (specificity, order) within a level.
        decls.sort_by_key(|(level, _)| *level);

        let (style, ctx) = self.compute_from_decls(&decls, hints.as_ref(), parent, id, None);

        // Generated content.
        let own = Inherited {
            style,
            ctx: ctx.clone(),
        };
        let mut generated = |rules: &[&Rule<'a, 'i>]| -> Option<Generated> {
            if rules.is_empty() {
                return None;
            }
            let mut decls: Vec<(Level, &Property<'i>)> = Vec::new();
            for rule in rules {
                let (normal, important) = match rule.origin {
                    Origin::Ua => (Level::UaNormal, Level::UaImportant),
                    Origin::Author => (Level::AuthorNormal, Level::AuthorImportant),
                };
                decls.extend(rule.decls.declarations.iter().map(|p| (normal, p)));
                decls.extend(rule.decls.important_declarations.iter().map(|p| (important, p)));
            }
            decls.sort_by_key(|(level, _)| *level);
            let content = generated_content(&decls, dom, id, &own.ctx)?;
            let (style, _) = self.compute_from_decls(&decls, None, &own, id, Some(&own.style));
            if style.display == Display::None {
                return None;
            }
            Some(Generated { style, content })
        };
        let before = generated(&matched_before);
        let after = generated(&matched_after);

        (
            NodeStyle {
                style,
                before,
                after,
            },
            ctx,
        )
    }

    /// Applies an ordered declaration list. `pseudo_parent` is the
    /// originating element's style when computing generated content.
    fn compute_from_decls(
        &mut self,
        decls: &[(Level, &Property<'i>)],
        hints: Option<&DeclarationBlock<'_>>,
        parent: &Inherited<'i>,
        id: NodeId,
        pseudo_parent: Option<&ComputedStyle>,
    ) -> (ComputedStyle, ElementCtx<'i>) {
        let parent_style = &parent.style;
        let rem = self.root_font_size;
        // Visits every declaration in precedence order (hints sit between
        // UA and author normal declarations).
        let each = |f: &mut dyn FnMut(&Property<'_>)| {
            for (_, p) in decls.iter().filter(|(l, _)| *l < Level::Hints) {
                f(p);
            }
            if let Some(h) = hints {
                for p in &h.declarations {
                    f(p);
                }
            }
            for (_, p) in decls.iter().filter(|(l, _)| *l >= Level::Hints) {
                f(p);
            }
        };

        // Custom properties: declared values are stored verbatim and
        // substituted on use.
        let mut custom = parent.ctx.custom.clone();
        for (_, p) in decls {
            if let Property::Custom(c) = p {
                if let CustomPropertyName::Custom(name) = &c.name {
                    Rc::make_mut(&mut custom).insert(name.0.to_string(), owned_tokens(&c.value));
                }
            }
        }

        // Stage A: font-size, which every other em-relative value needs.
        let mut font_size = parent_style.font_size;
        let fs_ctx = LenCtx {
            em: parent_style.font_size,
            rem,
            vw: self.vp.width,
            vh: self.vp.height,
        };
        each(&mut |p| match p {
            Property::FontSize(fs) => font_size = values::font_size(fs, parent_style.font_size, &fs_ctx),
            Property::Font(f) => font_size = values::font_size(&f.size, parent_style.font_size, &fs_ctx),
            Property::Unparsed(u) => {
                let name = u.property_id.name();
                if name != "font-size" && name != "font" {
                    return;
                }
                match wide_keyword(&u.value) {
                    Some("inherit") | Some("unset") | Some("revert") | Some("revert-layer") => {
                        font_size = parent_style.font_size
                    }
                    Some(_) => font_size = 16.0,
                    None => {
                        with_reparsed(u, &custom, |block| match block.declarations.first() {
                            Some(Property::FontSize(fs)) => {
                                font_size = values::font_size(fs, parent_style.font_size, &fs_ctx)
                            }
                            Some(Property::Font(f)) => {
                                font_size = values::font_size(&f.size, parent_style.font_size, &fs_ctx)
                            }
                            _ => {}
                        });
                    }
                }
            }
            _ => {}
        });

        // Stage B: everything else.
        let mut st = ComputedStyle::inherit_from(parent_style);
        st.font_size = font_size;
        let mut a = Apply {
            st: &mut st,
            parent: parent_style,
            len: LenCtx {
                em: font_size,
                rem,
                vw: self.vp.width,
                vh: self.vp.height,
            },
            line_height_factor: parent.ctx.line_height_factor,
            border_current: [true; 4],
            tracks: &mut self.tracks,
        };
        if let Some(f) = parent.ctx.line_height_factor {
            a.st.line_height = Length::px(f * font_size);
        }
        each(&mut |p| apply_decl(p, &mut a, parent_style, &custom, 0));
        a.finish();
        let line_height_factor = a.line_height_factor;

        // Blockification (CSS Display §2.7): out-of-flow boxes and flex/grid
        // items are block-level.
        let container = pseudo_parent.unwrap_or(parent_style);
        let blockify = matches!(st.position, Position::Absolute | Position::Fixed)
            || st.float != Float::None
            || container.display.is_flex_or_grid_container();
        if blockify {
            st.display = match st.display {
                Display::Inline | Display::InlineBlock => Display::Block,
                Display::InlineFlex => Display::Flex,
                Display::InlineGrid => Display::Grid,
                d => d,
            };
        }
        // The root element is always a block container.
        if pseudo_parent.is_none() && self.dom.nodes[id].parent == Some(0) && st.display != Display::None {
            st.display = Display::Block;
        }

        (
            st,
            ElementCtx {
                custom,
                line_height_factor,
            },
        )
    }
}

/// Applies one declaration, resolving `var()` and CSS-wide keywords.
fn apply_decl(
    p: &Property<'_>,
    a: &mut Apply<'_>,
    parent: &ComputedStyle,
    custom: &CustomMap<'_>,
    depth: u8,
) {
    match p {
        Property::FontSize(_) => {}
        Property::Custom(c) => {
            if let CustomPropertyName::Unknown(_) = c.name {
                values::apply(p, a);
            }
        }
        Property::Unparsed(u) => {
            let name = u.property_id.name();
            if name == "font-size" {
                return;
            }
            if let Some(kw) = wide_keyword(&u.value) {
                apply_wide_keyword(name, kw, a, parent);
                return;
            }
            if depth > 0 {
                return; // substitution produced another unparsable value
            }
            with_reparsed(u, custom, |block| {
                for prop in &block.declarations {
                    apply_decl(prop, a, parent, custom, depth + 1);
                }
            });
        }
        Property::All(_) => {}
        other => values::apply(other, a),
    }
}

fn apply_wide_keyword(name: &str, kw: &str, a: &mut Apply<'_>, parent: &ComputedStyle) {
    let inherited = values::is_inherited(name);
    match kw {
        "inherit" => values::copy_property(name, parent, a),
        "initial" => values::copy_property(name, &ComputedStyle::INITIAL, a),
        _ => {
            // unset, revert, revert-layer
            if inherited {
                values::copy_property(name, parent, a);
            } else {
                values::copy_property(name, &ComputedStyle::INITIAL, a);
            }
        }
    }
    if name == "line-height" || name == "font" {
        a.line_height_factor = None;
    }
}

/// If the token list is exactly one CSS-wide keyword, returns it.
fn wide_keyword<'t>(list: &'t TokenList<'_>) -> Option<&'t str> {
    let tokens: Vec<&TokenOrValue<'_>> = list
        .0
        .iter()
        .filter(|t| !matches!(t, TokenOrValue::Token(Token::WhiteSpace(_))))
        .collect();
    match tokens.as_slice() {
        [TokenOrValue::Token(Token::Ident(id))] => {
            let s: &str = id;
            if matches!(
                s.to_ascii_lowercase().as_str(),
                "inherit" | "initial" | "unset" | "revert" | "revert-layer"
            ) {
                Some(s)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Clones a token list with an unbounded lifetime is not possible without
/// `into_owned`; the values only need to live as long as the sheet text,
/// which they borrow, so a plain clone suffices.
fn owned_tokens<'i>(list: &TokenList<'i>) -> TokenList<'i> {
    list.clone()
}

/// Maximum `var()` nesting before a value is treated as invalid.
const MAX_VAR_DEPTH: u8 = 16;

/// Substitutes `var()` references in `list`. `None` if a reference has no
/// value and no fallback (the declaration is invalid at computed-value time).
fn substitute<'i>(list: &TokenList<'i>, custom: &CustomMap<'i>, depth: u8) -> Option<TokenList<'i>> {
    if depth > MAX_VAR_DEPTH {
        return None;
    }
    let mut out = Vec::with_capacity(list.0.len());
    for t in &list.0 {
        match t {
            TokenOrValue::Var(v) => {
                let name: &str = &v.name.ident.0;
                match custom.get(name) {
                    Some(value) => {
                        let sub = substitute(value, custom, depth + 1)?;
                        out.extend(sub.0);
                    }
                    None => {
                        let fb = v.fallback.as_ref()?;
                        let sub = substitute(fb, custom, depth + 1)?;
                        out.extend(sub.0);
                    }
                }
            }
            TokenOrValue::Function(f) => {
                let args = substitute(&f.arguments, custom, depth + 1)?;
                out.push(TokenOrValue::Function(
                    lightningcss::properties::custom::Function {
                        name: f.name.clone(),
                        arguments: args,
                    },
                ));
            }
            other => out.push(other.clone()),
        }
    }
    Some(TokenList(out))
}

/// Substitutes `var()` in an unparsed declaration, serializes the result,
/// parses it again and hands the (short-lived) block to `f`.
fn with_reparsed<R>(
    u: &UnparsedProperty<'_>,
    custom: &CustomMap<'_>,
    f: impl FnOnce(&DeclarationBlock<'_>) -> R,
) -> Option<R> {
    let value = substitute_to_string(u, custom)?;
    let css = format!("{}:{}", u.property_id.name(), value);
    let block = DeclarationBlock::parse_string(&css, parser_options()).ok()?;
    Some(f(&block))
}

/// Substitutes `var()` in an unparsed declaration and serializes the
/// result for re-parsing.
fn substitute_to_string(u: &UnparsedProperty<'_>, custom: &CustomMap<'_>) -> Option<String> {
    let has_var = u.value.0.iter().any(|t| {
        matches!(t, TokenOrValue::Var(_))
            || matches!(t, TokenOrValue::Function(f) if f.arguments.0.iter().any(|t| matches!(t, TokenOrValue::Var(_))))
    });
    if !has_var {
        return None;
    }
    let value = substitute(&u.value, custom, 0)?;
    let prop = Property::Unparsed(UnparsedProperty {
        property_id: u.property_id.clone(),
        value,
    });
    prop.value_to_css_string(PrinterOptions::default()).ok()
}

/// Resolves the `content` of a pseudo-element: strings and `attr()`;
/// `none`/`normal`/anything else yields no box (plan §5).
fn generated_content<'i>(
    decls: &[(Level, &Property<'i>)],
    dom: &Dom,
    id: NodeId,
    ctx: &ElementCtx<'_>,
) -> Option<String> {
    let mut content: Option<String> = None;
    for (_, p) in decls {
        let tokens = match p {
            Property::Custom(c) if c.name.as_ref().eq_ignore_ascii_case("content") => substitute(&c.value, &ctx.custom, 0),
            Property::Unparsed(u) if u.property_id.name() == "content" => substitute(&u.value, &ctx.custom, 0),
            _ => continue,
        };
        let Some(tokens) = tokens else {
            content = None;
            continue;
        };
        let mut s = String::new();
        let mut ok = true;
        for t in &tokens.0 {
            match t {
                TokenOrValue::Token(Token::String(q)) => s.push_str(q),
                TokenOrValue::Token(Token::WhiteSpace(_)) => {}
                TokenOrValue::Token(Token::Ident(i)) => {
                    // none | normal | open-quote …: no string content
                    let _ = i;
                    ok = false;
                    break;
                }
                TokenOrValue::Function(f) if f.name.0.eq_ignore_ascii_case("attr") => {
                    let name = f.arguments.0.iter().find_map(|t| match t {
                        TokenOrValue::Token(Token::Ident(i)) => Some(i.to_string()),
                        _ => None,
                    });
                    if let Some(v) = name.and_then(|n| dom.attr(id, &n.to_ascii_lowercase())) {
                        s.push_str(v);
                    }
                }
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        content = if ok { Some(s) } else { None };
    }
    content.filter(|s| !s.is_empty())
}

/// Legacy attributes that map to CSS (HTML "rendering" section), emitted
/// as a declaration string applied below author rules.
fn presentational_hints(dom: &Dom, id: NodeId) -> Option<String> {
    let tag = dom.tag(id)?;
    let mut css = String::new();
    let dim = |v: &str| -> Option<String> {
        let v = v.trim();
        if let Some(p) = v.strip_suffix('%') {
            p.trim().parse::<f32>().ok().map(|n| format!("{n}%"))
        } else {
            let digits: String = v.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            digits.parse::<f32>().ok().map(|n| format!("{n}px"))
        }
    };
    match tag {
        "img" | "iframe" | "video" | "canvas" | "object" | "embed" | "table" | "td" | "th" | "hr"
        | "col" => {
            if let Some(w) = dom.attr(id, "width").and_then(|v| dim(v)) {
                css.push_str(&format!("width:{w};"));
            }
            if let Some(h) = dom.attr(id, "height").and_then(|v| dim(v)) {
                css.push_str(&format!("height:{h};"));
            }
        }
        _ => {}
    }
    if matches!(
        tag,
        "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "td" | "th" | "tr" | "caption"
            | "thead" | "tbody" | "tfoot"
    ) {
        if let Some(align) = dom.attr(id, "align") {
            match align.trim().to_ascii_lowercase().as_str() {
                "left" => css.push_str("text-align:left;"),
                "right" => css.push_str("text-align:right;"),
                "center" | "middle" => css.push_str("text-align:center;"),
                "justify" => css.push_str("text-align:justify;"),
                _ => {}
            }
        }
    }
    if matches!(tag, "table" | "td" | "th" | "tr" | "body" | "div") {
        if let Some(c) = dom.attr(id, "bgcolor") {
            css.push_str(&format!("background-color:{};", c.trim()));
        }
    }
    if tag == "body" {
        if let Some(c) = dom.attr(id, "text") {
            css.push_str(&format!("color:{};", c.trim()));
        }
    }
    if tag == "font" {
        if let Some(c) = dom.attr(id, "color") {
            css.push_str(&format!("color:{};", c.trim()));
        }
    }
    if matches!(tag, "td" | "th") && dom.attr(id, "nowrap").is_some() {
        css.push_str("white-space:nowrap;");
    }
    if tag == "table" {
        if let Some(b) = dom.attr(id, "border") {
            if b.trim() != "0" {
                css.push_str("border:1px solid gray;");
            }
        }
    }
    if css.is_empty() {
        None
    } else {
        Some(css)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::media::Breakpoints;
    use crate::css::rules::{parse_sheets, SheetSource};
    use crate::css::ua::UA_STYLESHEET;
    use css_subset::{FontFamily, FontWeight, Rgba, TextAlign, WhiteSpace};
    use url::Url;

    fn sources(author: &str) -> Vec<SheetSource> {
        let base = Url::parse("https://example.test/").unwrap();
        vec![
            SheetSource {
                css: UA_STYLESHEET.to_string(),
                base: base.clone(),
                origin: Origin::Ua,
                media: vec![],
            },
            SheetSource {
                css: author.to_string(),
                base,
                origin: Origin::Author,
                media: vec![],
            },
        ]
    }

    /// Runs the cascade and returns `f(dom, cascade)`.
    fn with_cascade<T>(html: &str, css: &str, f: impl FnOnce(&Dom, &Cascade<'_, '_>) -> T) -> T {
        let dom = Dom::parse(html);
        let srcs = sources(css);
        let parsed = parse_sheets(&srcs);
        let mut bps = Breakpoints::default();
        let vp = Viewport {
            width: 1280.0,
            height: 800.0,
        };
        let rules = RuleSet::build(&parsed, &vp, &mut bps);
        let mut cascade = Cascade::new(&dom, &rules, vp);
        cascade.run();
        f(&dom, &cascade)
    }

    fn style_of(dom: &Dom, c: &Cascade<'_, '_>, tag: &str) -> ComputedStyle {
        let id = dom.find_first(tag).unwrap();
        c.styles[id].as_ref().unwrap().style
    }

    #[test]
    fn ua_and_inheritance() {
        with_cascade("<body><h1>T</h1><p>x<b>y</b></p><pre>z</pre><a href=#>l</a>", "", |dom, c| {
            let body = style_of(dom, c, "body");
            assert_eq!(body.display, Display::Block);
            assert_eq!(body.margin[0], Length::px(8.0));
            assert_eq!(body.font_family, FontFamily::Serif);
            let h1 = style_of(dom, c, "h1");
            assert_eq!(h1.font_size, 32.0);
            assert_eq!(h1.font_weight, FontWeight::Bold);
            assert_eq!(h1.margin[0], Length::px(0.67 * 32.0));
            let b = style_of(dom, c, "b");
            assert_eq!(b.font_weight, FontWeight::Bold);
            assert_eq!(b.display, Display::Inline);
            let pre = style_of(dom, c, "pre");
            assert_eq!(pre.white_space, WhiteSpace::Pre);
            assert_eq!(pre.font_family, FontFamily::Mono);
            let a = style_of(dom, c, "a");
            assert_eq!(a.color, Rgba::rgb(0, 0, 0xee));
            assert!(a.text_decoration.contains(css_subset::TextDecoration::UNDERLINE));
            let html = style_of(dom, c, "html");
            assert_eq!(html.display, Display::Block);
        });
    }

    #[test]
    fn specificity_importance_and_style_attr() {
        let css = "p { color: red } .c { color: blue } #i { color: green } p { color: black !important }
                   div { text-align: center } span { text-align: inherit } .u { text-align: unset }";
        with_cascade(
            "<div><p id=i class=c style='color: pink'>x</p><span>y</span><em class=u>z</em></div>",
            css,
            |dom, c| {
                // !important beats the style attribute and the id.
                assert_eq!(style_of(dom, c, "p").color, Rgba::BLACK);
                assert_eq!(style_of(dom, c, "span").text_align, TextAlign::Center);
                // unset on an inherited property inherits.
                assert_eq!(style_of(dom, c, "em").text_align, TextAlign::Center);
            },
        );
        with_cascade("<p id=i class=c style='color: pink'>x</p>", "p{color:red} .c{color:blue} #i{color:green}", |dom, c| {
            assert_eq!(style_of(dom, c, "p").color, Rgba::rgb(255, 192, 203));
        });
        with_cascade("<p id=i class=c>x</p>", "p{color:red} .c{color:blue} #i{color:green}", |dom, c| {
            assert_eq!(style_of(dom, c, "p").color, Rgba::rgb(0, 128, 0));
        });
    }

    #[test]
    fn em_rem_and_line_height() {
        let css = "html { font-size: 20px } body { line-height: 1.5; font-size: 10px } p { font-size: 2em; margin: 1em 1rem; padding: 50% } h1 { line-height: 30px } h2 { line-height: 150% }";
        with_cascade("<body><p>x</p><h1>y</h1><h2>z</h2>", css, |dom, c| {
            let p = style_of(dom, c, "p");
            assert_eq!(p.font_size, 20.0);
            assert_eq!(p.margin[0], Length::px(20.0));
            assert_eq!(p.margin[1], Length::px(20.0));
            assert_eq!(p.padding[0], Length::percent(50.0));
            // Number line-height recomputes against the child's font size.
            assert_eq!(p.line_height, Length::px(30.0));
            assert_eq!(style_of(dom, c, "body").line_height, Length::px(15.0));
            assert_eq!(style_of(dom, c, "h1").line_height, Length::px(30.0));
            let h2 = style_of(dom, c, "h2");
            assert_eq!(h2.line_height, Length::px(h2.font_size * 1.5));
        });
    }

    #[test]
    fn custom_properties_and_var() {
        let css = ":root { --main: #ff0000; --pad: 4px } div { --main: #00ff00; --gap: calc(var(--pad) * 2) }
                   p { color: var(--main); padding: var(--pad) var(--missing, 3px); margin: var(--gap); width: var(--nope) }
                   span { color: var(--main) }";
        with_cascade("<div><p>x</p></div><span>y</span>", css, |dom, c| {
            let p = style_of(dom, c, "p");
            assert_eq!(p.color, Rgba::rgb(0, 255, 0));
            assert_eq!(p.padding[0], Length::px(4.0));
            assert_eq!(p.padding[1], Length::px(3.0));
            assert_eq!(p.margin[0], Length::px(8.0));
            assert!(p.width.is_auto());
            assert_eq!(style_of(dom, c, "span").color, Rgba::rgb(255, 0, 0));
        });
    }

    #[test]
    fn media_queries_pick_the_bucket() {
        let css = "p { color: red } @media (max-width: 600px) { p { color: blue } } @media (min-width: 1000px) { p { color: green } }";
        with_cascade("<p>x</p>", css, |dom, c| {
            assert_eq!(style_of(dom, c, "p").color, Rgba::rgb(0, 128, 0));
        });
    }

    #[test]
    fn display_none_subtrees_are_skipped_and_blockified() {
        with_cascade(
            "<div style='display:none'><p>x</p></div><section style='display:flex'><span>i</span></section><em style='float:left'>f</em>",
            "",
            |dom, c| {
                assert_eq!(style_of(dom, c, "div").display, Display::None);
                let p = dom.find_first("p").unwrap();
                assert!(c.styles[p].is_none());
                assert_eq!(style_of(dom, c, "span").display, Display::Block);
                assert_eq!(style_of(dom, c, "em").display, Display::Block);
                assert!(c.styles[dom.find_first("title").unwrap_or(0)].is_none());
            },
        );
    }

    #[test]
    fn pseudo_elements_with_string_content() {
        let css = "p::before { content: 'A' attr(data-x) ; color: red } p::after { content: none } q::after { content: url(x.png) } li::before { content: '' }";
        with_cascade("<p data-x=B>x</p><q>y</q><li>z</li>", css, |dom, c| {
            let p = dom.find_first("p").unwrap();
            let ns = c.styles[p].as_ref().unwrap();
            let before = ns.before.as_ref().expect("::before generated");
            assert_eq!(before.content, "AB");
            assert_eq!(before.style.color, Rgba::rgb(255, 0, 0));
            assert_eq!(before.style.font_size, 16.0);
            assert!(ns.after.is_none());
            let q = dom.find_first("q").unwrap();
            assert!(c.styles[q].as_ref().unwrap().after.is_none());
            let li = dom.find_first("li").unwrap();
            assert!(c.styles[li].as_ref().unwrap().before.is_none());
        });
    }

    #[test]
    fn presentational_hints_and_attribute_selectors() {
        with_cascade(
            "<img src=x width=100 height='50%'><p align=center>c</p><td nowrap>n</td><table border=1></table><div hidden>h</div><input type=HIDDEN>",
            "",
            |dom, c| {
                let img = style_of(dom, c, "img");
                assert_eq!(img.width, Length::px(100.0));
                assert_eq!(img.height, Length::percent(50.0));
                assert_eq!(style_of(dom, c, "p").text_align, TextAlign::Center);
                assert_eq!(style_of(dom, c, "div").display, Display::None);
                assert_eq!(style_of(dom, c, "input").display, Display::None);
            },
        );
    }

    #[test]
    fn compute_is_pure() {
        with_cascade("<div><p>x</p></div>", "div { font-size: 20px } p { margin: 1em }", |dom, c| {
            let p = dom.find_first("p").unwrap();
            let div = dom.find_first("div").unwrap();
            let expected = c.styles[p].clone().unwrap();
            // Recompute against the parent's stored context.
            let parent = c.inherited_of(div).unwrap();
            let mut again = Cascade::new(c.dom, c.rules, c.vp);
            again.root_font_size = c.root_font_size;
            let (ns, _) = again.compute(p, &parent);
            assert_eq!(ns, expected);
            assert_eq!(ns.style.margin[0], Length::px(20.0));
        });
    }
}
