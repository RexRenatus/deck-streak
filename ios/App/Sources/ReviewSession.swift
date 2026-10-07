import Foundation

/// A rating as the answer bar names it, in the bar's order (SPEC-348 R10). The codec's own
/// `Rating` numbers the engine's; this one is the app's, so a view or the model can name a
/// rating without importing the codec.
enum Rating: Int32, CaseIterable, Sendable { case again, hard, good, easy }

/// The queue's three counts, which the review screen shows above the card (SPEC-348 R9).
struct ReviewCounts: Equatable, Sendable {
    var new: UInt32
    var learning: UInt32
    var review: UInt32
}

/// What the session shows next: a card's question with the queue's counts and the four
/// intervals, or the designed end (SPEC-348 R10, R11).
struct ReviewStep: Equatable, Sendable {
    var phase: ReviewPhase
    var face: ReviewFace
    var counts: ReviewCounts
    var intervals: [String]
}

/// One review of one deck, over the engine's session: the queue's head card, the time it was
/// shown, and the answers sent (SPEC-348 R10).
actor ReviewSession {
    private let engine: EngineSession
    /// The answers this session has sent; a double tap must raise it once (A16).
    private(set) var sentAnswers = 0

    init(engine: EngineSession) {
        self.engine = engine
    }

    /// Makes `deck` the current deck (7,22), so the queue is its own.
    func choose(_ deck: Deck) async throws {}

    /// The queue's head card's question, its counts and its four intervals, or the designed end.
    func next(autoplay: Bool, installed: [InstalledVoice]) async throws -> ReviewStep {
        ReviewStep(
            phase: .finished, face: .empty, counts: ReviewCounts(new: 0, learning: 0, review: 0),
            intervals: [])
    }

    /// The shown card's answer face.
    func reveal(autoplay: Bool, installed: [InstalledVoice]) async throws -> ReviewFace {
        .empty
    }

    /// Sends the shown card's answer with the rating's own state, at most once per shown card.
    func answer(_ rating: Rating) async throws {}

    /// The languages' installed voices and the choice for each, for the voice picker.
    func voices(for languages: [String], installed: [InstalledVoice]) async -> [VoiceLanguage] {
        []
    }

    /// Records `identifier` as the voice for `language`, or clears the choice given nil.
    func choose(voice identifier: String?, for language: String) async throws {}
}
