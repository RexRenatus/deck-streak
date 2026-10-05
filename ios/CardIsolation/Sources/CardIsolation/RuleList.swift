import Foundation
import WebKit

/// The card view's content rule list, layer L3 (SPEC-349 R4, ADR-360 D4): one rule that blocks
/// every load of every resource type, with no domain condition and no exception. The card's HTML
/// is handed to the view as a string, so its own document is no load.
/// It does NOT stop script running, a peer connection, or a navigation the app itself starts.
public enum RuleList {
    /// The identifier the list is compiled under.
    public static let identifier = "card-blocks-every-load"

    /// The list's source, as WebKit's content blocker format spells it.
    public static let source = #"[{"trigger":{"url-filter":".*"},"action":{"type":"block"}}]"#

    /// Each way `candidate` is weaker than a list that blocks every load, named; empty when it is
    /// exactly as strong.
    public static func weaker(_ candidate: String) -> [String] {
        guard let decoded = try? JSONSerialization.jsonObject(with: Data(candidate.utf8)),
              let rules = decoded as? [[String: Any]]
        else {
            return ["not a list of rules"]
        }
        var named: [String] = []
        if rules.count != 1 {
            named.append("\(rules.count) rules")
        }
        for rule in rules {
            let trigger = rule["trigger"] as? [String: Any] ?? [:]
            for key in trigger.keys.sorted() where key != "url-filter" {
                named.append(key)
            }
            let filter = trigger["url-filter"] as? String ?? "none"
            if filter != ".*" {
                named.append("url-filter \(filter)")
            }
            let action = rule["action"] as? [String: Any] ?? [:]
            let type = action["type"] as? String ?? "none"
            if type != "block" {
                named.append("action \(type)")
            }
        }
        return named
    }

    /// The list, compiled; nil when WebKit refuses it, and then the factory builds no view.
    @MainActor
    public static func compile() async -> WKContentRuleList? {
        try? await WKContentRuleListStore.default().compileContentRuleList(
            forIdentifier: identifier, encodedContentRuleList: source)
    }
}
