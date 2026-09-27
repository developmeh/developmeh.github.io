//! The "compression is style-preserving" property (plan §6): after the
//! passes ran, re-running the cascade for every surviving node against its
//! parent *in the compressed tree* must reproduce the stored computed
//! style. Runs in debug builds, in tests and with `--check`.

use css_subset::ComputedStyle;

use crate::css::cascade::{Cascade, Inherited};
use crate::tree::{anonymous_style, text_style, FTree};

/// Verifies every live node. Returns a description of the first mismatch.
pub fn check_style_preserving(tree: &FTree, cascade: &mut Cascade<'_, '_>) -> Result<(), String> {
    for idx in tree.pre_order() {
        let n = &tree.nodes[idx];
        let expected = n.style;
        let actual = match (n.dom, n.pseudo, n.anonymous, n.parent) {
            // Root element: computed against the initial context.
            (Some(d), None, false, None) => cascade.compute(d, &Inherited::initial()).0.style,
            (Some(d), None, false, Some(p)) if n.kind.is_element() => {
                let Some(parent) = inherited_for(tree, cascade, p) else {
                    return Err(format!("node {idx}: parent {p} has no cascade context"));
                };
                cascade.compute(d, &parent).0.style
            }
            (Some(d), Some(kind), false, Some(p)) => {
                // Pseudo-elements: recompute the originating element and
                // take its generated box.
                let Some(parent) = inherited_for(tree, cascade, p) else {
                    return Err(format!("pseudo {idx}: parent {p} has no cascade context"));
                };
                // The originating element is the parent itself.
                let ns = cascade.compute(d, &parent).0;
                let g = match kind {
                    crate::css::select::PseudoKind::Before => ns.before,
                    crate::css::select::PseudoKind::After => ns.after,
                };
                match g {
                    Some(g) => g.style,
                    None => return Err(format!("pseudo {idx}: no longer generated")),
                }
            }
            (_, _, true, Some(p)) => anonymous_style(&tree.nodes[p].style),
            (_, _, _, Some(p)) => {
                // Text, markers: inherited style of the parent plus the
                // accumulated decoration.
                text_style(&tree.nodes[p].style, tree.nodes[p].deco)
            }
            _ => return Err(format!("node {idx}: unexpected shape")),
        };
        if actual != expected {
            return Err(format!(
                "node {idx} ({:?} <{}>): style changed by compression\n  expected {:?}\n  actual   {:?}",
                n.kind,
                n.tag,
                summarize(&expected),
                summarize(&actual)
            ));
        }
    }
    Ok(())
}

/// The context a child of arena node `p` inherits, derived from the
/// nearest ancestor that has a DOM element (anonymous wrappers are
/// transparent to inheritance) but using the compressed tree's stored
/// styles.
fn inherited_for<'i>(tree: &FTree, cascade: &Cascade<'_, 'i>, p: usize) -> Option<Inherited<'i>> {
    let mut cur = Some(p);
    while let Some(i) = cur {
        let n = &tree.nodes[i];
        if let (Some(d), None, false) = (n.dom, n.pseudo, n.anonymous) {
            let ctx = cascade.contexts[d].clone()?;
            return Some(Inherited {
                style: n.style,
                ctx,
            });
        }
        cur = n.parent;
    }
    None
}

fn summarize(s: &ComputedStyle) -> String {
    format!(
        "display={:?} font={:?}/{}px/{:?} color={:?} lh={:?} margin={:?} deco={:?}",
        s.display,
        s.font_family,
        s.font_size,
        s.font_weight,
        s.color,
        s.line_height,
        s.margin,
        s.text_decoration
    )
}
