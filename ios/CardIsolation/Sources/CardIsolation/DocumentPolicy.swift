import Foundation

/// Layer L12, the document policy (SPEC-361 R5, ADR-372): a `Content-Security-Policy` the factory
/// hands the view ahead of the card's markup, so it is the first element of the card's `head`. It
/// admits images, media and fonts only as `data:` URLs, and style and script only inline; it admits
/// no plugin, no form target and no base URL, and every other load it leaves to its default, which
/// admits none.
/// It does NOT stop a resource hint's early connection, a navigation, a new window or a peer
/// connection: the rule list (L3), the gate (L5), the window refusal (L6) and the peer-connection
/// removal (L8) hold those.
public enum DocumentPolicy {
    /// The policy, one directive after another.
    public static let policy = "default-src 'none'; img-src data:; media-src data:; font-src data:; "
        + "style-src 'unsafe-inline'; script-src 'unsafe-inline'; object-src 'none'; "
        + "form-action 'none'; base-uri 'none'"

    /// The prefix: the document type, then the policy's own element, which the parser places first
    /// in `head` ahead of anything the card's markup holds.
    public static let prefix = "<!doctype html><meta http-equiv=\"Content-Security-Policy\" content=\"\(policy)\">"

    /// The card's markup with the prefix ahead of it.
    public static func prefixed(_ html: String) -> String {
        prefix + html
    }

    /// Each way `candidate` is weaker than the policy, named: a directive the policy states that
    /// the candidate drops, and a source the candidate admits that the policy's same directive
    /// does not. A directive the policy leaves to its default admits nothing, so any source the
    /// candidate gives it is named.
    public static func weaker(_ candidate: String) -> [String] {
        let reference = directives(of: policy)
        let read = directives(of: candidate)
        var named: [String] = []
        for directive in reference where !read.contains(where: { $0.name == directive.name }) {
            named.append("\(directive.name) dropped")
        }
        for directive in read {
            let admitted = reference.first { $0.name == directive.name }?.sources ?? ["'none'"]
            for source in directive.sources where source != "'none'" && !admitted.contains(source) {
                named.append("\(directive.name) admits \(source)")
            }
        }
        return named
    }

    /// A policy's directives in order, each with its sources, lowercased. The first occurrence of
    /// a directive wins, as a browser reads a policy.
    static func directives(of policy: String) -> [(name: String, sources: [String])] {
        var read: [(name: String, sources: [String])] = []
        for part in policy.lowercased().split(separator: ";") {
            let words = part.split(whereSeparator: \.isWhitespace).map(String.init)
            guard let name = words.first, !read.contains(where: { $0.name == name }) else {
                continue
            }
            read.append((name: name, sources: Array(words.dropFirst())))
        }
        return read
    }
}
