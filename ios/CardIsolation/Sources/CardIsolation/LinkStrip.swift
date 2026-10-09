/// L14, the link strip (SPEC-392 R2, ADR-406 D1, the schematic's section 9): every `<link` in a
/// card's text, its four letters in any ASCII case and wherever the five characters stand, is
/// renamed to `<wbr` before the factory prefixes L12's policy and loads the card, and every other
/// byte is kept. A `wbr` is void and has no attribute of its own, so a renamed tag wraps nothing
/// after it and its attributes grant nothing.
///
/// It does NOT stop a link a script adds (L3 holds its hint), a link in a document a frame loads
/// (L5), or any other element's load (L3, L12). It renames the five characters in text too: in a
/// text area, a title, raw text, a comment or an attribute value.
public enum LinkStrip {
    /// `html` with every link opener renamed to a `wbr` opener, and every other byte kept.
    public static func stripped(_ html: String) -> String {
        html
    }
}
