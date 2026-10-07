import Observation
import UIKit

/// Where a review stands. A call in flight has its own phase, set before the call is awaited,
/// so a second tap finds the phase moved and does nothing (SPEC-348 R10, R16).
enum ReviewPhase: Equatable, Sendable {
    case loading, question, revealing, answer, answering, finished, refused
}

/// The review screen's state, held in a model and never in a view's state, so a size-class change
/// keeps it (SPEC-348 R16). Every gesture enters through `perform`.
@MainActor
@Observable
final class ReviewModel {
    /// The deck under review.
    let deck: Deck
    /// Where the review stands.
    private(set) var phase = ReviewPhase.loading
    /// The face shown.
    private(set) var face = ReviewFace.empty
    /// The queue's counts at the shown card.
    private(set) var counts = ReviewCounts(new: 0, learning: 0, review: 0)
    /// The four ratings' intervals for the shown card, in the bar's order.
    private(set) var intervals: [String] = []
    /// The clips handed to the player when the shown face appeared.
    private(set) var plan: [ReviewClip] = []

    private let session: ReviewSession
    private let voiceOverRunning: @MainActor () -> Bool
    private let installed: @MainActor () -> [InstalledVoice]

    init(
        deck: Deck, session: ReviewSession,
        voiceOverRunning: @escaping @MainActor () -> Bool = { UIAccessibility.isVoiceOverRunning },
        installed: @escaping @MainActor () -> [InstalledVoice] = InstalledVoices.all
    ) {
        self.deck = deck
        self.session = session
        self.voiceOverRunning = voiceOverRunning
        self.installed = installed
    }

    /// Makes the deck current and shows its first card, or the designed end.
    func start() async {}

    /// The one entry for the review screen's gestures (SPEC-348 R16).
    func perform(_ action: ReviewAction) async {}

    /// Records the voice for a language the face speaks, from the picker (SPEC-348 R14).
    func choose(voice identifier: String?, for language: String) async {}
}
