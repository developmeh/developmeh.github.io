//! Stylesheet collection (`<style>`, `<link rel=stylesheet>`, `@import`)
//! and flattening of parsed sheets into one ordered rule list with the
//! at-rules of plan §5 resolved: `@media` evaluated for the viewport
//! bucket, `@layer` flattened in declaration order, `@supports` evaluated
//! against our property list, `@font-face`/`@keyframes` ignored.

use std::collections::HashMap;

use lightningcss::declaration::DeclarationBlock;
use lightningcss::properties::Property;
use lightningcss::rules::supports::SupportsCondition;
use lightningcss::rules::{CssRule, CssRuleList};
use lightningcss::selector::{PseudoElement, Selector};
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::traits::ToCss;
use parcel_selectors::parser::Component;
use url::Url;

use super::media::{evaluate, parse_media_list, Breakpoints, Viewport};
use super::select::{selector_supported, PseudoKind};
use crate::dom::Dom;
use crate::fetch::Fetcher;

/// Where a stylesheet came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// The bundled UA stylesheet.
    Ua,
    /// The page's own styles.
    Author,
}

/// One stylesheet's text plus what it needs to be interpreted.
#[derive(Clone, Debug)]
pub struct SheetSource {
    /// CSS text.
    pub css: String,
    /// URL for resolving `url()` and `@import`.
    pub base: Url,
    /// Origin.
    pub origin: Origin,
    /// Media lists that must all match for the sheet to apply (from
    /// `media=""` attributes and `@import` media, outermost first).
    pub media: Vec<String>,
}

/// Maximum `@import` nesting (plan §5).
const MAX_IMPORT_DEPTH: usize = 3;

/// Parser options used everywhere: recover from errors, skip bad rules.
pub fn parser_options<'i>() -> ParserOptions<'i> {
    ParserOptions {
        error_recovery: true,
        ..ParserOptions::default()
    }
}

/// Gathers the UA sheet and every author sheet in document order,
/// expanding `@import`s (depth <= 3).
pub fn collect_sheets(dom: &Dom, base: &Url, fetcher: &mut Fetcher, ua: &str) -> Vec<SheetSource> {
    let mut out = vec![SheetSource {
        css: ua.to_string(),
        base: base.clone(),
        origin: Origin::Ua,
        media: Vec::new(),
    }];
    let mut authored = Vec::new();
    for id in dom.descendants(0) {
        match dom.tag(id) {
            Some("style") => {
                if let Some(css) = collect_style_element(dom, id, base) {
                    authored.push(css);
                }
            }
            Some("link") => {
                let is_stylesheet = dom.attr(id, "rel").is_some_and(|rel| {
                    rel.split_ascii_whitespace()
                        .any(|t| t.eq_ignore_ascii_case("stylesheet"))
                });
                let type_ok = dom
                    .attr(id, "type")
                    .map(|t| t.trim().is_empty() || t.trim().eq_ignore_ascii_case("text/css"))
                    .unwrap_or(true);
                if !is_stylesheet || !type_ok {
                    continue;
                }
                let Some(href) = dom.attr(id, "href") else {
                    continue;
                };
                let Ok(url) = base.join(href.trim()) else {
                    continue;
                };
                let Ok(res) = fetcher.fetch_subresource(&url) else {
                    continue;
                };
                let media = dom
                    .attr(id, "media")
                    .map(|m| m.trim())
                    .filter(|m| !m.is_empty())
                    .map(|m| vec![m.to_string()])
                    .unwrap_or_default();
                authored.push(SheetSource {
                    css: String::from_utf8_lossy(&res.bytes).into_owned(),
                    base: res.url,
                    origin: Origin::Author,
                    media,
                });
            }
            _ => {}
        }
    }
    for sheet in authored {
        expand_imports(sheet, 0, fetcher, &mut out);
    }
    out
}

fn collect_style_element(dom: &Dom, id: usize, base: &Url) -> Option<SheetSource> {
    let css = dom.text_content(id);
    if css.trim().is_empty() {
        return None;
    }
    let media = dom
        .attr(id, "media")
        .map(|m| m.trim())
        .filter(|m| !m.is_empty())
        .map(|m| vec![m.to_string()])
        .unwrap_or_default();
    Some(SheetSource {
        css,
        base: base.clone(),
        origin: Origin::Author,
        media,
    })
}

/// Pushes `sheet`'s imports (recursively, in order) and then the sheet
/// itself, so imported rules precede the importing sheet's rules.
fn expand_imports(
    sheet: SheetSource,
    depth: usize,
    fetcher: &mut Fetcher,
    out: &mut Vec<SheetSource>,
) {
    if depth < MAX_IMPORT_DEPTH {
        let imports: Vec<(String, Option<String>)> =
            match StyleSheet::parse(&sheet.css, parser_options()) {
                Ok(parsed) => parsed
                    .rules
                    .0
                    .iter()
                    .filter_map(|r| match r {
                        CssRule::Import(i) => {
                            let media = if i.media.media_queries.is_empty() {
                                None
                            } else {
                                i.media.to_css_string(Default::default()).ok()
                            };
                            Some((i.url.to_string(), media))
                        }
                        _ => None,
                    })
                    .collect(),
                Err(_) => Vec::new(),
            };
        for (href, media) in imports {
            let Ok(url) = sheet.base.join(&href) else {
                continue;
            };
            let Ok(res) = fetcher.fetch_subresource(&url) else {
                continue;
            };
            let mut media_stack = sheet.media.clone();
            media_stack.extend(media);
            let imported = SheetSource {
                css: String::from_utf8_lossy(&res.bytes).into_owned(),
                base: res.url,
                origin: sheet.origin,
                media: media_stack,
            };
            expand_imports(imported, depth + 1, fetcher, out);
        }
    }
    out.push(sheet);
}

/// A parsed sheet borrowing its [`SheetSource`].
pub struct ParsedSheet<'i> {
    /// Origin.
    pub origin: Origin,
    /// Media lists that must all match.
    pub media: &'i [String],
    /// The parsed rules.
    pub sheet: StyleSheet<'i>,
}

/// Parses every source; sheets that fail to parse are skipped.
pub fn parse_sheets(sources: &[SheetSource]) -> Vec<ParsedSheet<'_>> {
    sources
        .iter()
        .filter_map(|src| {
            let sheet = StyleSheet::parse(&src.css, parser_options()).ok()?;
            Some(ParsedSheet {
                origin: src.origin,
                media: &src.media,
                sheet,
            })
        })
        .collect()
}

/// One flattened style rule: a single selector with its declarations.
pub struct Rule<'a, 'i> {
    /// The selector (already checked with [`selector_supported`]).
    pub selector: &'a Selector<'i>,
    /// Declarations (normal and `!important`).
    pub decls: &'a DeclarationBlock<'i>,
    /// Origin.
    pub origin: Origin,
    /// Position in the flattened order (later wins on ties).
    pub order: u32,
    /// Selector specificity.
    pub specificity: u32,
    /// `::before`/`::after` rules apply to generated content only.
    pub pseudo: Option<PseudoKind>,
}

/// All rules, bucketed by the rightmost compound selector's key so
/// matching only visits candidates.
#[derive(Default)]
pub struct RuleSet<'a, 'i> {
    /// Every rule, in order.
    pub rules: Vec<Rule<'a, 'i>>,
    by_id: HashMap<String, Vec<u32>>,
    by_class: HashMap<String, Vec<u32>>,
    by_tag: HashMap<String, Vec<u32>>,
    universal: Vec<u32>,
    /// Whether any `::before`/`::after` rule exists.
    pub has_pseudo_rules: bool,
}

impl<'a, 'i> RuleSet<'a, 'i> {
    /// Flattens parsed sheets, evaluating media queries for `vp` and
    /// recording width breakpoints in `bps`.
    pub fn build(sheets: &'a [ParsedSheet<'i>], vp: &Viewport, bps: &mut Breakpoints) -> Self {
        let mut set = RuleSet::default();
        for sheet in sheets {
            let media_ok = sheet.media.iter().all(|m| {
                parse_media_list(m)
                    .map(|list| evaluate(&list, vp, bps))
                    .unwrap_or(false)
            });
            if !media_ok {
                continue;
            }
            set.add_rules(&sheet.sheet.rules, sheet.origin, vp, bps);
        }
        set
    }

    fn add_rules(
        &mut self,
        list: &'a CssRuleList<'i>,
        origin: Origin,
        vp: &Viewport,
        bps: &mut Breakpoints,
    ) {
        for rule in &list.0 {
            match rule {
                CssRule::Style(style) => {
                    self.add_style_rule(&style.selectors.0, &style.declarations, origin)
                }
                CssRule::Media(m) => {
                    if evaluate(&m.query, vp, bps) {
                        self.add_rules(&m.rules, origin, vp, bps);
                    }
                }
                CssRule::LayerBlock(l) => self.add_rules(&l.rules, origin, vp, bps),
                CssRule::Supports(s) => {
                    if supports(&s.condition) {
                        self.add_rules(&s.rules, origin, vp, bps);
                    }
                }
                CssRule::Nesting(n) => {
                    self.add_style_rule(&n.style.selectors.0, &n.style.declarations, origin)
                }
                // @import handled by collect_sheets; @font-face, @keyframes,
                // @page, @container, @scope, unknown: ignored (plan §5).
                _ => {}
            }
        }
    }

    fn add_style_rule(
        &mut self,
        selectors: &'a [Selector<'i>],
        decls: &'a DeclarationBlock<'i>,
        origin: Origin,
    ) {
        if decls.declarations.is_empty() && decls.important_declarations.is_empty() {
            return;
        }
        for selector in selectors {
            if !selector_supported(selector) {
                continue;
            }
            let pseudo = match selector.pseudo_element() {
                None => None,
                Some(PseudoElement::Before) => Some(PseudoKind::Before),
                Some(PseudoElement::After) => Some(PseudoKind::After),
                // ::marker, ::selection, ::first-line, …: never matched.
                Some(_) => continue,
            };
            let idx = self.rules.len() as u32;
            self.rules.push(Rule {
                selector,
                decls,
                origin,
                order: idx,
                specificity: selector.specificity(),
                pseudo,
            });
            self.has_pseudo_rules |= pseudo.is_some();
            match rightmost_key(selector) {
                Key::Id(id) => self.by_id.entry(id).or_default().push(idx),
                Key::Class(c) => self.by_class.entry(c).or_default().push(idx),
                Key::Tag(t) => self.by_tag.entry(t).or_default().push(idx),
                Key::Universal => self.universal.push(idx),
            }
        }
    }

    /// Candidate rules for an element with the given tag, id and classes,
    /// sorted by `(specificity, order)` ascending.
    pub fn candidates(&self, tag: &str, id: Option<&str>, classes: &str) -> Vec<&Rule<'a, 'i>> {
        let mut idx: Vec<u32> = self.universal.clone();
        if let Some(v) = self.by_tag.get(tag) {
            idx.extend_from_slice(v);
        }
        if let Some(v) = id.and_then(|id| self.by_id.get(id)) {
            idx.extend_from_slice(v);
        }
        for c in classes.split_ascii_whitespace() {
            if let Some(v) = self.by_class.get(c) {
                idx.extend_from_slice(v);
            }
        }
        let mut out: Vec<&Rule<'a, 'i>> =
            idx.into_iter().map(|i| &self.rules[i as usize]).collect();
        out.sort_by_key(|r| (r.specificity, r.order));
        out
    }
}

enum Key {
    Id(String),
    Class(String),
    Tag(String),
    Universal,
}

/// The most selective simple selector of the rightmost compound.
fn rightmost_key(selector: &Selector<'_>) -> Key {
    let mut class = None;
    let mut tag = None;
    for c in selector.iter() {
        match c {
            Component::ID(id) => return Key::Id(id.0.to_string()),
            Component::Class(c) if class.is_none() => class = Some(c.0.to_string()),
            Component::LocalName(ln) if tag.is_none() => tag = Some(ln.lower_name.0.to_string()),
            _ => {}
        }
    }
    if let Some(c) = class {
        Key::Class(c)
    } else if let Some(t) = tag {
        Key::Tag(t)
    } else {
        Key::Universal
    }
}

/// Evaluates an `@supports` condition against what the loader parses.
pub fn supports(cond: &SupportsCondition<'_>) -> bool {
    match cond {
        SupportsCondition::Not(inner) => !supports(inner),
        SupportsCondition::And(list) => list.iter().all(supports),
        SupportsCondition::Or(list) => list.iter().any(supports),
        SupportsCondition::Declaration { property_id, value } => {
            match Property::parse_string(property_id.clone(), value, parser_options()) {
                Ok(Property::Unparsed(_)) | Ok(Property::Custom(_)) | Err(_) => false,
                Ok(_) => true,
            }
        }
        SupportsCondition::Selector(sel) => {
            let css = format!("{sel}{{}}");
            let parsed = StyleSheet::parse(&css, ParserOptions::default());
            let ok = match &parsed {
                Ok(sheet) => sheet
                    .rules
                    .0
                    .iter()
                    .any(|r| matches!(r, CssRule::Style(s) if s.selectors.0.iter().all(selector_supported))),
                Err(_) => false,
            };
            drop(parsed);
            ok
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::Limits;

    type RuleSummary = (String, u32, Option<PseudoKind>);

    fn build(css: &str) -> (Vec<u16>, Vec<RuleSummary>) {
        let sources = vec![SheetSource {
            css: css.to_string(),
            base: Url::parse("https://example.test/").unwrap(),
            origin: Origin::Author,
            media: vec![],
        }];
        let parsed = parse_sheets(&sources);
        let mut bps = Breakpoints::default();
        let set = RuleSet::build(
            &parsed,
            &Viewport {
                width: 1280.0,
                height: 800.0,
            },
            &mut bps,
        );
        let rules = set
            .rules
            .iter()
            .map(|r| {
                (
                    r.selector.to_css_string(Default::default()).unwrap(),
                    r.specificity,
                    r.pseudo,
                )
            })
            .collect();
        (bps.into_vec(), rules)
    }

    #[test]
    fn flattens_at_rules() {
        let css = r#"
            p { color: red }
            @media (max-width: 600px) { .narrow { color: blue } }
            @media screen and (min-width: 1000px) { .wide, #w { color: blue } }
            @layer base { h1 { margin: 0 } }
            @supports (display: grid) { .g { display: grid } }
            @supports (display: pancake) { .no { display: block } }
            @supports not (float: left) { .nofloat { color: red } }
            @supports selector(:has(a)) { .has { color: red } }
            @font-face { font-family: x; src: url(x.woff) }
            @keyframes k { from { opacity: 0 } }
            li::before { content: "x" }
            li::marker { color: red }
            div:has(p) { color: red }
            a:hover { color: red }
            .empty { }
        "#;
        let (bps, rules) = build(css);
        assert_eq!(bps, vec![600, 1000]);
        let names: Vec<&str> = rules.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "p",
                ".wide",
                "#w",
                "h1",
                ".g",
                ".nofloat",
                "li:before",
                "a:hover"
            ]
        );
        assert_eq!(rules[0].1, 1);
        assert_eq!(rules[2].1, 1 << 20);
        assert_eq!(rules[6].2, Some(PseudoKind::Before));
    }

    #[test]
    fn candidates_are_bucketed_and_sorted() {
        let sources = vec![SheetSource {
            css: "* { color: red } p { color: red } .c { color: red } #i { color: red } div p.c { color: red } span { color: red } p:first-child { color: red }".to_string(),
            base: Url::parse("https://example.test/").unwrap(),
            origin: Origin::Author,
            media: vec![],
        }];
        let parsed = parse_sheets(&sources);
        let mut bps = Breakpoints::default();
        let set = RuleSet::build(
            &parsed,
            &Viewport {
                width: 1280.0,
                height: 800.0,
            },
            &mut bps,
        );
        let c = set.candidates("p", Some("i"), "c other");
        let sels: Vec<String> = c
            .iter()
            .map(|r| r.selector.to_css_string(Default::default()).unwrap())
            .collect();
        assert_eq!(sels, vec!["*", "p", ".c", "p:first-child", "div p.c", "#i"]);
        assert_eq!(set.candidates("span", None, "").len(), 2);
    }

    #[test]
    fn collects_and_imports() {
        let dir = std::env::temp_dir().join(format!("lean-loader-rules-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.css"),
            "@import url(b.css) print; @import 'c.css'; .a{color:red}",
        )
        .unwrap();
        std::fs::write(dir.join("b.css"), ".b{color:red}").unwrap();
        std::fs::write(dir.join("c.css"), "@import url(a.css); .c{color:red}").unwrap();
        let base = Url::from_directory_path(&dir)
            .unwrap()
            .join("index.html")
            .unwrap();
        let dom = Dom::parse(
            "<link rel=stylesheet href=a.css><style media=\"(min-width: 100px)\">.s{color:red}</style><link rel=icon href=x.css><body>",
        );
        let mut fetcher = Fetcher::new(Limits::default());
        let sheets = collect_sheets(&dom, &base, &mut fetcher, "html{}");
        let summary: Vec<(Origin, &str, Vec<String>)> = sheets
            .iter()
            .map(|s| (s.origin, s.css.trim(), s.media.clone()))
            .collect();
        // Imports precede the importing sheet; the depth limit stops the
        // a -> c -> a cycle.
        assert_eq!(summary[0], (Origin::Ua, "html{}", vec![]));
        assert_eq!(
            summary[1],
            (Origin::Author, ".b{color:red}", vec!["print".to_string()])
        );
        let pos = |prefix: &str| {
            summary
                .iter()
                .position(|s| s.1.starts_with(prefix))
                .unwrap()
        };
        assert!(pos("@import url(a.css)") < pos("@import url(b.css) print;"));
        assert!(summary.len() < 12);
        let last = summary.last().unwrap();
        assert_eq!(last.1, ".s{color:red}");
        assert_eq!(last.2, vec!["(min-width: 100px)".to_string()]);
        // Print-only import contributes no rules; the media'd style does.
        let parsed = parse_sheets(&sheets);
        let mut bps = Breakpoints::default();
        let set = RuleSet::build(
            &parsed,
            &Viewport {
                width: 1280.0,
                height: 800.0,
            },
            &mut bps,
        );
        let sels: Vec<String> = set
            .rules
            .iter()
            .map(|r| r.selector.to_css_string(Default::default()).unwrap())
            .collect();
        assert!(!sels.contains(&".b".to_string()));
        assert!(sels.contains(&".s".to_string()));
        assert!(sels.contains(&".a".to_string()));
        assert_eq!(bps.into_vec(), vec![100]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
