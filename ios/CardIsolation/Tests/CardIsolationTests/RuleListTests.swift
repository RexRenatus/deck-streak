// SPEC-349 A2: the rule list blocks every load of every type and has no exception. Its source is
// read as JSON on the macOS host; compiling it is the simulator suite's.
import Foundation
import XCTest

import CardIsolation

final class RuleListTests: XCTestCase {
    /// Prints how many lists were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    func test_the_rule_list_blocks_every_load_of_every_type() throws {
        // The behaviour first: the source is one rule, a `.*` filter with no other condition, that
        // blocks.
        let decoded = try JSONSerialization.jsonObject(with: Data(RuleList.source.utf8))
        let rules = try XCTUnwrap(decoded as? [[String: Any]], "the rule list is not a list of rules")
        XCTAssertEqual(rules.count, 1, "the rule list holds \(rules.count) rules, not one")
        let rule = try XCTUnwrap(rules.first, "the rule list holds no rule")
        XCTAssertEqual(rule.keys.sorted(), ["action", "trigger"])
        let trigger = try XCTUnwrap(rule["trigger"] as? [String: Any], "the rule has no trigger")
        XCTAssertEqual(trigger.keys.sorted(), ["url-filter"])
        XCTAssertEqual(trigger["url-filter"] as? String, ".*")
        let action = try XCTUnwrap(rule["action"] as? [String: Any], "the rule has no action")
        XCTAssertEqual(action.keys.sorted(), ["type"])
        XCTAssertEqual(action["type"] as? String, "block")
        XCTAssertEqual(RuleList.weaker(RuleList.source), [])

        // The controls: each planted weaker list is named by the way it is weaker.
        let block = #""action":{"type":"block"}"#
        let planted: [(name: String, source: String, reason: String)] = [
            ("a resource-type restriction",
             #"[{"trigger":{"url-filter":".*","resource-type":["image"]},"# + block + "}]",
             "resource-type"),
            ("an if-domain",
             #"[{"trigger":{"url-filter":".*","if-domain":["*planted.invalid"]},"# + block + "}]",
             "if-domain"),
            ("an unless-domain",
             #"[{"trigger":{"url-filter":".*","unless-domain":["*planted.invalid"]},"# + block + "}]",
             "unless-domain"),
            ("a url-filter of ^https",
             #"[{"trigger":{"url-filter":"^https"},"# + block + "}]",
             "url-filter ^https"),
            ("an action of css-display-none",
             #"[{"trigger":{"url-filter":".*"},"action":{"type":"css-display-none","selector":"img"}}]"#,
             "action css-display-none"),
            ("a second rule that ignores the first",
             #"[{"trigger":{"url-filter":".*"},"# + block + #"},{"trigger":{"url-filter":".*"},"action":{"type":"ignore-previous-rules"}}]"#,
             "2 rules"),
            ("no rule", "[]", "0 rules"),
            ("not a list", "{}", "not a list of rules"),
        ]
        for plant in examined("planted rule lists", planted) {
            let named = RuleList.weaker(plant.source)
            XCTAssertTrue(named.contains(plant.reason), "\(plant.name): \(named)")
        }
    }
}
