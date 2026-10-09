import Foundation
import HarnessWire

/// The queue's three counts, which the review screen shows above the card (SPEC-348 R9).
struct ReviewCounts: Equatable, Sendable {
    var new: UInt32
    var learning: UInt32
    var review: UInt32
}

/// What the session shows next: a card's question with the queue's counts and the four
/// intervals, or the designed end (SPEC-348 R10, R11); and the card's flag, 0 at the end
/// (SPEC-358 R5).
struct ReviewStep: Equatable, Sendable {
    var phase: ReviewPhase
    var face: ReviewFace
    var counts: ReviewCounts
    var intervals: [String]
    var flag: UInt32
}

/// One review of one deck, over the engine's session: the queue's head card, the time it was
/// shown, and the answers sent (SPEC-348 R10). The head is cleared before an answer is sent, with
/// no suspension point between the read and the write, so a second rating for one shown card
/// finds no head and sends nothing.
actor ReviewSession {
    private let engine: EngineSession
    /// The engine's red flag, which the model compares a card's flag with (SPEC-358 R5).
    nonisolated let red: UInt32
    /// The answers this session has sent; a double tap must raise it once (A16).
    private(set) var sentAnswers = 0
    /// The card shown, until its answer is sent.
    private var head: QueuedCard?
    /// When the head card's question was shown, for the answer's time taken.
    private var shownAt = Date()

    init(engine: EngineSession) {
        self.engine = engine
        red = engine.red
    }

    /// Makes `deck` the current deck (7,22), so the queue is its own.
    func choose(_ deck: Deck) async throws {
        try await engine.setCurrentDeck(deck)
    }

    /// The queue's head card's question, its counts and its four intervals, or the designed end.
    func next(autoplay: Bool, installed: [InstalledVoice]) async throws -> ReviewStep {
        let queue = try await engine.queue()
        let counts = ReviewCounts(
            new: queue.newCount, learning: queue.learningCount, review: queue.reviewCount)
        guard let card = queue.cards.first else {
            head = nil
            return ReviewStep(
                phase: .finished, face: .empty, counts: counts, intervals: [], flag: 0)
        }
        let intervals = try await engine.intervals(card)
        let face = try await engine.face(
            card.cardID, answer: false, autoplay: autoplay, installed: installed)
        head = card
        shownAt = Date()
        return ReviewStep(
            phase: .question, face: face, counts: counts, intervals: intervals, flag: card.flag)
    }

    /// The shown card's answer face, or the empty face when no card is shown.
    func reveal(autoplay: Bool, installed: [InstalledVoice]) async throws -> ReviewFace {
        try await shown(ReviewFace.empty) { card in
            try await engine.face(
                card.cardID, answer: true, autoplay: autoplay, installed: installed)
        }
    }

    /// Buries the shown card as the user's bury; nothing when no card is shown (SPEC-358 R5).
    func bury() async throws {
        try await shown(()) { card in
            try await engine.bury(card)
        }
    }

    /// Toggles red on the shown card and keeps the engine's answer on the head card, so the
    /// next press toggles from it; the new flag, or nil when no card is shown (SPEC-358 R5).
    func flag() async throws -> UInt32? {
        try await shown(nil) { card in
            let flag = try await engine.flag(card)
            head?.flag = flag
            return flag
        }
    }

    /// Sends the press of `rating` on the shown card, with the states the card was shown with, at
    /// most once per shown card: the head is cleared before the call is awaited. The adapter picks
    /// the rating's state, so the session builds no next state of its own (SPEC-365 R12).
    func answer(_ rating: Rating) async throws {
        guard let card = head else { return }
        head = nil
        sentAnswers += 1
        try await engine.answer(
            card, rating: rating,
            millisecondsTaken: UInt32(clamping: Int64(Date().timeIntervalSince(shownAt) * 1000)))
    }

    /// The languages' installed voices and the choice for each, for the voice picker.
    func voices(_ languages: [String], installed: [InstalledVoice]) async -> [VoiceLanguage] {
        await engine.voices(languages, installed: installed)
    }

    /// Records `identifier` as the voice for `language`, or clears the choice given nil.
    func choose(voice identifier: String?, language: String) async throws {
        try await engine.choose(voice: identifier, language: language)
    }

    /// The one read of the shown card that reveal, bury and flag share: `work` over the head
    /// card, or `none` when no card is shown.
    private func shown<Value>(
        _ none: Value, _ work: (QueuedCard) async throws -> Value
    ) async throws -> Value {
        guard let card = head else { return none }
        return try await work(card)
    }
}
