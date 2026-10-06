import Foundation

/// Layer L12, the document policy (SPEC-361 R5, ADR-372): the prefix the factory hands the view
/// ahead of the card's markup, and the reader that names how a candidate policy is weaker.
/// This is the stub its criterion is red against: the prefix is empty and nothing is named.
public enum DocumentPolicy {
    /// The prefix. Empty here.
    public static let prefix = ""

    /// The card's markup with the prefix ahead of it.
    public static func prefixed(_ html: String) -> String {
        prefix + html
    }

    /// Each way `candidate` is weaker than the policy, named. Names nothing here.
    public static func weaker(_ candidate: String) -> [String] {
        []
    }
}
