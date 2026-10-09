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
    /// The bytes a link start tag opens with, in lower case: the tag's open bracket and its name.
    static let opener = Array("<link".utf8)

    /// What every opener becomes: the opener of a void element that has no attribute of its own.
    static let renamed = Array("<wbr".utf8)

    /// `html` with every link opener renamed to a `wbr` opener, and every other byte kept.
    public static func stripped(_ html: String) -> String {
        let bytes = Array(html.utf8)
        var out: [UInt8] = []
        out.reserveCapacity(bytes.count)
        var index = 0
        while index < bytes.count {
            if opens(bytes, at: index) { out.append(contentsOf: renamed); index += opener.count } else { out.append(bytes[index]); index += 1 }
        }
        return String(decoding: out, as: UTF8.self)
    }

    /// Whether an opener starts at `index`. Only ASCII capitals are lowered, so no Unicode case
    /// folding turns another character into one of its letters.
    static func opens(_ bytes: [UInt8], at index: Int) -> Bool {
        guard index + opener.count <= bytes.count else {
            return false
        }
        for offset in 0..<opener.count where lowered(bytes[index + offset]) != opener[offset] {
            return false
        }
        return true
    }

    /// `byte` with an ASCII capital lowered, and any other byte as it came.
    static func lowered(_ byte: UInt8) -> UInt8 {
        (0x41...0x5A).contains(byte) ? byte | 0x20 : byte
    }
}
