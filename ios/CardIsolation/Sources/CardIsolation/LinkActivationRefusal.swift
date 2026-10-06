import Foundation

/// Layer L10, the link-activation refusal (SPEC-361 R3, ADR-372): the source of the one user
/// script that refuses a link's activation inside a card before the engine follows it.
/// This is the stub its criterion is red against: the source is empty, so no listener is
/// registered.
public enum LinkActivationRefusal {
    /// The script's source. Empty here.
    public static let source = ""
}
