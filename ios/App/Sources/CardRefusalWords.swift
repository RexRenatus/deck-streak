import CardIsolation
import UIKit

/// Why a card face shows no card, in DeckStreak's words (SPEC-505 R7, R8; ADR-523 D5): each refusal
/// the card view's factory can give reads a sentence of the app's own, and the card face announces
/// the words it shows to VoiceOver once.
enum CardRefusalWords {
    /// Every refusal the card view's factory can give.
    static let refusals = CardViewRefusal.allCases

    /// The words for `error`.
    static func text(of error: Error) -> String {
        String(describing: error)
    }

    /// Announces `text` to VoiceOver through `post`, the system's announcement by default.
    @MainActor
    static func announce(
        _ text: String,
        post: @MainActor (UIAccessibility.Notification, Any?) -> Void = { notification, argument in
            UIAccessibility.post(notification: notification, argument: argument)
        }
    ) {}
}
