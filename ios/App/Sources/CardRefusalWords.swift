import CardIsolation
import UIKit

/// Why a card face shows no card, in DeckStreak's words (SPEC-505 R7, R8; ADR-523 D5): each refusal
/// the card view's factory can give reads a sentence of the app's own, never the error's case name,
/// and the card face announces the words it shows to VoiceOver once.
enum CardRefusalWords {
    /// Every refusal the card view's factory can give.
    static let refusals = CardViewRefusal.allCases

    /// The words for an error that is not the factory's refusal.
    static let general = "This card cannot be shown here. You can still answer, bury or flag it."

    /// The words for `error`: its refusal's sentence, or the general one. The switch names every
    /// case, so a new refusal does not build without its words.
    static func text(of error: Error) -> String {
        guard let refusal = error as? CardViewRefusal else { return general }
        switch refusal {
        case .ruleListDidNotCompile:
            return "This card cannot be shown safely here: the card view's content blocker did not "
                + "start, so no card view was built. Restart DeckStreak. You can still answer, bury or "
                + "flag it."
        }
    }

    /// Announces `text` to VoiceOver once through `post`, the system's announcement by default.
    @MainActor
    static func announce(
        _ text: String,
        post: @MainActor (UIAccessibility.Notification, Any?) -> Void = { notification, argument in
            UIAccessibility.post(notification: notification, argument: argument)
        }
    ) {
        post(.announcement, text)
    }
}
