import Foundation

/// The card view's content rule list, layer L3 (SPEC-349 R4, ADR-360 D4): one rule that blocks
/// every load of every resource type, with no domain condition and no exception. The card's HTML
/// is handed to the view as a string, so its own document is no load.
/// It does NOT stop script running, a peer connection, or a navigation the app itself starts.
public enum RuleList {
    /// The identifier the list is compiled under.
    public static let identifier = "card-blocks-every-load"

    /// The list's source, as WebKit's content blocker format spells it.
    public static let source = "[]"

    /// Each way `candidate` is weaker than a list that blocks every load, named; empty when it is
    /// exactly as strong.
    public static func weaker(_ candidate: String) -> [String] {
        []
    }
}
