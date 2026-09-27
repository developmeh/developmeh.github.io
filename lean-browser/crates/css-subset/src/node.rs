//! Per-node bit flags shared by the loader and the renderer (plan §4).

bitflags::bitflags! {
    /// `Node.flags`. Stored as a plain `u8` in the page file; unknown bits
    /// are rejected by the validator.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct NodeFlags: u8 {
        /// The node is an `<a href>`; `href` is in the attribute table.
        const LINK = 1 << 0;
        /// The node has an `id` and can be the target of `#fragment`.
        const ANCHOR_TARGET = 1 << 1;
        /// Reachable with Tab.
        const FOCUSABLE = 1 << 2;
        /// Generates a block-level box.
        const BLOCK = 1 << 3;
        /// Inline replaced element (image, form control).
        const INLINE_REPLACED = 1 << 4;
        /// Preformatted text (`white-space: pre*`).
        const PRE = 1 << 5;
        /// The attribute table has entries for this node.
        const HAS_ATTRS = 1 << 6;
        /// A form field contributing to submission.
        const IS_FORM_FIELD = 1 << 7;
    }
}

#[cfg(test)]
mod tests {
    use super::NodeFlags;

    #[test]
    fn all_bits_are_defined() {
        assert_eq!(NodeFlags::all().bits(), 0xFF);
        assert!(NodeFlags::from_bits(0b1000_0001).is_some());
    }
}
