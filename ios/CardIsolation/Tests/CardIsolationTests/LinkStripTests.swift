// SPEC-392 A1 to A3: the link strip (L14) renames every `<link` opener, its four letters in any
// ASCII case and wherever the five characters stand, to a `<wbr` opener, and keeps every other
// byte. It is a pure function of the card's text, so it runs on the macOS host with no WebKit.
import XCTest

import CardIsolation

final class LinkStripTests: XCTestCase {
    /// Prints how many goldens were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    /// The goldens the strip got wrong, each with what it returned. Outputs are compared byte for
    /// byte, as UTF-8, so two strings Swift calls equal under canonical equivalence still differ.
    func wrong(_ goldens: [(input: String, wanted: String)]) -> [String] {
        goldens.compactMap { golden -> String? in
            let got = LinkStrip.stripped(golden.input)
            if Array(got.utf8) == Array(golden.wanted.utf8) {
                return nil
            }
            return "\(golden.input.debugDescription) gave \(got.debugDescription), "
                + "wanted \(golden.wanted.debugDescription)"
        }
    }

    /// A1's goldens (SPEC-392 section 3): each input, then its output written out here rather
    /// than derived from the code under test.
    static let renamed: [(input: String, wanted: String)] = [
        (input: "<link rel=\"stylesheet\" href=\"x\">", wanted: "<wbr rel=\"stylesheet\" href=\"x\">"),
        (input: "<LINK REL=preconnect HREF=x>", wanted: "<wbr REL=preconnect HREF=x>"),
        (input: "<LiNk\trel=preload>", wanted: "<wbr\trel=preload>"),
        (input: "<link\nrel=prefetch/>", wanted: "<wbr\nrel=prefetch/>"),
        (input: "<link>", wanted: "<wbr>"),
        (
            input: "<div><template shadowrootmode=\"open\"><link rel=\"preconnect\" href=\"x\"></template></div>",
            wanted: "<div><template shadowrootmode=\"open\"><wbr rel=\"preconnect\" href=\"x\"></template></div>"
        ),
        (input: "<svg><link href=\"x\"/></svg>", wanted: "<svg><wbr href=\"x\"/></svg>"),
        (input: "<linkx>", wanted: "<wbrx>"),
        (
            input: "<link rel=\"preload\" href=\"x\"><p>a</p><link rel=\"prefetch\" href=\"y\">",
            wanted: "<wbr rel=\"preload\" href=\"x\"><p>a</p><wbr rel=\"prefetch\" href=\"y\">"
        ),
        (input: "<link<link>", wanted: "<wbr<wbr>"),
        (
            input: "<iframe srcdoc=\"<link rel=preconnect href=x>\"></iframe>",
            wanted: "<iframe srcdoc=\"<wbr rel=preconnect href=x>\"></iframe>"
        ),
        (input: "<textarea><link></textarea>", wanted: "<textarea><wbr></textarea>"),
        (input: "é<link>ü", wanted: "é<wbr>ü"),
    ]

    /// A2's goldens: an opener that starts the card, one that ends it, and one that is the card.
    static let atTheEnds: [(input: String, wanted: String)] = [
        (input: "<link", wanted: "<wbr"),
        (input: "<p>a</p><link", wanted: "<p>a</p><wbr"),
        (input: "<LINK", wanted: "<wbr"),
    ]

    /// A3's goldens: markup that holds no `<link` opener, each returned unchanged. A Kelvin sign
    /// and a dotted capital I fold to ASCII only under Unicode case folding, which the strip does
    /// not use.
    static let unchanged: [String] = [
        "",
        "<p>a card</p>",
        "<svg><use xlink:href=\"#a\"/></svg>",
        "<p>unlink and blink</p>",
        "&lt;link rel=preconnect&gt;",
        "</link>",
        "< link>",
        "<l ink>",
        "<lin\u{212A}>",
        "<l\u{0130}nk>",
        "<!-- a comment -->",
    ]

    func test_every_link_opener_is_renamed_to_a_wbr_opener_in_any_ascii_case() {
        // The behaviour first: each golden's output is its input with every opener renamed.
        XCTAssertEqual(wrong(Self.renamed), [], "A1's goldens the link strip got wrong")
        _ = examined("goldens", Self.renamed)
    }

    func test_an_opener_at_either_end_of_the_card_is_renamed() {
        // The behaviour first: an opener at the card's start or end is renamed.
        XCTAssertEqual(wrong(Self.atTheEnds), [], "A2's goldens the link strip got wrong")
        _ = examined("goldens", Self.atTheEnds)
    }

    func test_markup_holding_no_link_opener_is_returned_byte_for_byte() {
        // The behaviour first: markup with no opener comes back byte for byte.
        let goldens = Self.unchanged.map { (input: $0, wanted: $0) }
        XCTAssertEqual(wrong(goldens), [], "A3's goldens the link strip changed")
        _ = examined("goldens", goldens)
    }
}
