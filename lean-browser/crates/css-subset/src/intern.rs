//! Style interning table (compression pass 5, plan §6).

use std::collections::HashMap;
use std::fmt;

use crate::style::ComputedStyle;

/// Index of a style in the page's `styles` table.
pub type StyleId = u16;

/// "Not rendered" sentinel; never present in frozen pages.
pub const STYLE_NONE: StyleId = 0xFFFF;

/// Maximum number of distinct styles a page may carry. Leaves headroom below
/// [`STYLE_NONE`] so the loader can detect overflow and quantize (plan §6.5).
pub const MAX_STYLES: usize = 65_000;

/// Returned by [`StyleTable::intern`] when the table is full.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleTableFull;

impl fmt::Display for StyleTableFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "style table holds the maximum of {MAX_STYLES} styles")
    }
}

impl std::error::Error for StyleTableFull {}

/// Assigns `u16` ids to distinct computed styles in first-use order.
#[derive(Clone, Debug, Default)]
pub struct StyleTable {
    styles: Vec<ComputedStyle>,
    index: HashMap<ComputedStyle, StyleId>,
}

impl StyleTable {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the id of `style`, adding it if it is new.
    pub fn intern(&mut self, style: &ComputedStyle) -> Result<StyleId, StyleTableFull> {
        if let Some(&id) = self.index.get(style) {
            return Ok(id);
        }
        if self.styles.len() >= MAX_STYLES {
            return Err(StyleTableFull);
        }
        let id = self.styles.len() as StyleId;
        self.styles.push(*style);
        self.index.insert(*style, id);
        Ok(id)
    }

    /// Looks up an already-interned style without inserting.
    pub fn id_of(&self, style: &ComputedStyle) -> Option<StyleId> {
        self.index.get(style).copied()
    }

    /// The style with the given id.
    pub fn get(&self, id: StyleId) -> Option<&ComputedStyle> {
        self.styles.get(id as usize)
    }

    /// Number of distinct styles.
    pub fn len(&self) -> usize {
        self.styles.len()
    }

    /// Whether no style has been interned.
    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    /// Styles in id order.
    pub fn styles(&self) -> &[ComputedStyle] {
        &self.styles
    }

    /// Consumes the table, yielding the styles in id order (for `Page.styles`).
    pub fn into_vec(self) -> Vec<ComputedStyle> {
        self.styles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Display, Length};

    #[test]
    fn interns_in_first_use_order() {
        let mut t = StyleTable::new();
        let a = ComputedStyle::INITIAL;
        let mut b = ComputedStyle::INITIAL;
        b.display = Display::Block;
        assert_eq!(t.intern(&a), Ok(0));
        assert_eq!(t.intern(&b), Ok(1));
        assert_eq!(t.intern(&a), Ok(0));
        assert_eq!(t.len(), 2);
        assert_eq!(t.get(1), Some(&b));
        assert_eq!(t.id_of(&b), Some(1));
        assert_eq!(t.into_vec(), vec![a, b]);
    }

    #[test]
    fn reports_overflow() {
        let mut t = StyleTable::new();
        for i in 0..MAX_STYLES {
            let mut s = ComputedStyle::INITIAL;
            s.width = Length::px(i as f32);
            assert!(t.intern(&s).is_ok());
        }
        let mut s = ComputedStyle::INITIAL;
        s.width = Length::px(-1.0);
        assert_eq!(t.intern(&s), Err(StyleTableFull));
        // Existing styles still resolve.
        let mut first = ComputedStyle::INITIAL;
        first.width = Length::px(0.0);
        assert_eq!(t.intern(&first), Ok(0));
    }
}
