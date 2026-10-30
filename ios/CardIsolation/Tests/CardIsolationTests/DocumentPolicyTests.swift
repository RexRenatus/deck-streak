// SPEC-361 A4: the document policy (L12) is exactly R5's, the prefix puts it first in the card's
// `head`, and `weaker` names each way a candidate policy is weaker than it. The policy is read from
// the prefix as text on the macOS host; whether the engine enforces it is the simulator suite's.
import XCTest

import CardIsolation

final class DocumentPolicyTests: XCTestCase {
    /// Prints how many directives or candidates were judged, and refuses zero (the tdd pack's
    /// contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    /// R5's policy, written here rather than read from the code under test.
    static let policy = "default-src 'none'; img-src data:; media-src data:; font-src data:; "
        + "style-src 'unsafe-inline'; script-src 'unsafe-inline'; object-src 'none'; "
        + "form-action 'none'; base-uri 'none'"

    /// R5's policy as directives, each with its sources.
    static let directives: [String: [String]] = [
        "default-src": ["'none'"],
        "img-src": ["data:"],
        "media-src": ["data:"],
        "font-src": ["data:"],
        "style-src": ["'unsafe-inline'"],
        "script-src": ["'unsafe-inline'"],
        "object-src": ["'none'"],
        "form-action": ["'none'"],
        "base-uri": ["'none'"],
    ]

    /// How the prefix opens: the document type, then the policy's own element, first in `head`.
    static let opening = #"<!doctype html><meta http-equiv="Content-Security-Policy" content=""#

    /// A policy's directives, each with its sources, as the test reads them.
    func directives(of policy: String) -> [String: [String]] {
        var read: [String: [String]] = [:]
        for part in policy.split(separator: ";") {
            let words = part.split(whereSeparator: \.isWhitespace).map(String.init)
            if let name = words.first, read[name] == nil {
                read[name] = Array(words.dropFirst())
            }
        }
        return read
    }

    func test_the_policy_admits_only_data_media_and_inline_style_and_script() {
        // The behaviour first: the policy the prefix carries is exactly R5's, and it opens the
        // document, so it is the first thing in `head`.
        let prefix = DocumentPolicy.prefix
        var carried = ""
        if prefix.hasPrefix(Self.opening) && prefix.hasSuffix(#"">"#) {
            carried = String(prefix.dropFirst(Self.opening.count).dropLast(2))
        }
        let read = directives(of: carried)
        for name in examined("directives", Self.directives.keys.sorted()) {
            XCTAssertEqual(read[name], Self.directives[name], "the policy's \(name) is not R5's")
        }
        XCTAssertEqual(read, Self.directives, "the policy the prefix carries is not R5's")
        XCTAssertEqual(carried, Self.policy, "the policy the prefix carries is not R5's text")
        XCTAssertTrue(prefix.hasPrefix(Self.opening), "the prefix does not open with the policy: \(prefix)")
        XCTAssertEqual(
            DocumentPolicy.prefixed("<p>a card</p>"), prefix + "<p>a card</p>",
            "the card's markup does not follow the prefix")

        // The controls: R5's own policy, and one that only narrows it, are weaker in no way; each
        // planted weaker policy is named by the way it is weaker, and by nothing else.
        let policy = Self.policy
        let planted: [(name: String, candidate: String, named: [String])] = [
            ("R5's policy", policy, []),
            ("a narrower img-src", policy.replacingOccurrences(of: "img-src data:", with: "img-src 'none'"), []),
            ("a source added to default-src",
             policy.replacingOccurrences(of: "default-src 'none'", with: "default-src 'none' https:"),
             ["default-src admits https:"]),
            ("a scheme added to img-src",
             policy.replacingOccurrences(of: "img-src data:", with: "img-src data: https:"),
             ["img-src admits https:"]),
            ("a scheme added to media-src",
             policy.replacingOccurrences(of: "media-src data:", with: "media-src data: blob:"),
             ["media-src admits blob:"]),
            ("a scheme added to font-src",
             policy.replacingOccurrences(of: "font-src data:", with: "font-src data: http:"),
             ["font-src admits http:"]),
            ("'unsafe-eval' added to script-src",
             policy.replacingOccurrences(
                of: "script-src 'unsafe-inline'", with: "script-src 'unsafe-inline' 'unsafe-eval'"),
             ["script-src admits 'unsafe-eval'"]),
            ("a directive R5 leaves to default-src, opened",
             policy + "; connect-src https:", ["connect-src admits https:"]),
            ("object-src dropped",
             policy.replacingOccurrences(of: "object-src 'none'; ", with: ""), ["object-src dropped"]),
            ("form-action dropped",
             policy.replacingOccurrences(of: "form-action 'none'; ", with: ""), ["form-action dropped"]),
            ("base-uri dropped",
             policy.replacingOccurrences(of: "; base-uri 'none'", with: ""), ["base-uri dropped"]),
        ]
        for plant in examined("planted candidate policies", planted) {
            if plant.name != "R5's policy" {
                XCTAssertNotEqual(plant.candidate, policy, "\(plant.name): the planted candidate did not change")
            }
            XCTAssertEqual(DocumentPolicy.weaker(plant.candidate), plant.named, plant.name)
        }
    }
}
