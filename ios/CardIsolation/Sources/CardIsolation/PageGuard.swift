import Foundation

/// Layer L11, the page guard (SPEC-361 R4, ADR-372): the source of the one user script that
/// refuses a card's activation of a node it has detached and every rewrite of its own document.
/// This is the stub its criterion is red against: the source is empty, so nothing is guarded.
public enum PageGuard {
    /// The script's source. Empty here.
    public static let source = ""
}
