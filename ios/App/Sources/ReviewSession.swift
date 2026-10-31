import Foundation
import HarnessWire

/// The queue's three counts, which the review screen shows above the card (SPEC-348 R9).
struct ReviewCounts: Equatable, Sendable {
    var new: UInt32
    var learning: UInt32
    var review: UInt32
}

/// What the session shows next: a card's question with the queue's counts and the four
/// intervals, or the designed end (SPEC-348 R10, R11); the card's flag, 0 at the end
/// (SPEC-358 R5); and what the review's undo offers now, nothing at the end (SPEC-515 R6).
struct ReviewStep: Equatable, Sendable {
    var phase: ReviewPhase
    var face: ReviewFace
    var counts: ReviewCounts
    var intervals: [String]
    var flag: UInt32
    var undo: UndoDialog?
}

/// One review of one deck, over the engine's session: the queue's head card, its answer clock, and
/// the answers sent (SPEC-348 R10). The head is cleared before an answer is sent, with no
/// suspension point between the read and the write, so a second rating for one shown card finds no
/// head and sends nothing. The clock reads the session's own `now`, which a test hands in, and an
/// overlay over the review stops it while it is open (SPEC-507 R15).
actor ReviewSession {
    private let engine: EngineSession
    /// The engine's red flag, which the model compares a card's flag with (SPEC-358 R5).
    nonisolated let red: UInt32
    /// The answers this session has sent; a double tap must raise it once (A16).
    private(set) var sentAnswers = 0
    /// The card shown, until its answer is sent.
    private var head: QueuedCard?
    /// The instant source the answer clock reads.
    private let now: @Sendable () -> Date
    /// The head card's answer clock: its show, and the time overlays held it since.
    private var clock: AnswerClock

    init(engine: EngineSession, now: @escaping @Sendable () -> Date = { Date() }) {
        self.engine = engine
        red = engine.red
        self.now = now
        clock = AnswerClock(shownAt: now())
    }

    /// Makes `deck` the current deck (7,22), so the queue is its own.
    func choose(_ deck: Deck) async throws {
        try await engine.setCurrentDeck(deck)
    }

    /// The queue's head card's question, its counts, its four intervals and what the review's undo
    /// offers now, or the designed end, which offers none.
    func next(autoplay: Bool, installed: [InstalledVoice]) async throws -> ReviewStep {
        let queue = try await engine.queue()
        let counts = ReviewCounts(
            new: queue.newCount, learning: queue.learningCount, review: queue.reviewCount)
        guard let card = queue.cards.first else {
            head = nil
            return ReviewStep(
                phase: .finished, face: .empty, counts: counts, intervals: [], flag: 0, undo: nil)
        }
        let intervals = try await engine.intervals(card)
        let face = try await engine.face(
            card.cardID, answer: false, autoplay: autoplay, installed: installed)
        let undo = try await undoOffer().dialog
        head = card
        clock.shown(at: now())
        return ReviewStep(
            phase: .question, face: face, counts: counts, intervals: intervals, flag: card.flag,
            undo: undo)
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
    /// the rating's state, so the session builds no next state of its own (SPEC-365 R12). The time
    /// it sends is the answer clock's: since the card was shown, less every moment an overlay held
    /// the clock (SPEC-507 R15).
    func answer(_ rating: Rating) async throws {
        guard let card = head else { return }
        head = nil
        sentAnswers += 1
        try await engine.answer(
            card, rating: rating, millisecondsTaken: clock.taken(at: now()))
    }

    /// The card info panel's sections for the shown card, as the adapter words them, or none when
    /// no card is shown (SPEC-507 R14).
    func health() async throws -> [CardInfoSection] {
        try await shown([]) { card in
            try await engine.cardHealth(card.cardID)
        }
    }

    /// An overlay opens over the shown card: the answer clock stops (SPEC-507 R15).
    func pause() {
        clock.pause(at: now())
    }

    /// The overlay closes: the answer clock runs again.
    func resume() {
        clock.resume(at: now())
    }

    /// The languages' installed voices and the choice for each, for the voice picker.
    func voices(_ languages: [String], installed: [InstalledVoice]) async -> [VoiceLanguage] {
        await engine.voices(languages, installed: installed)
    }

    /// What the adapter would undo now, with its dialog's words, or none and the sentence that
    /// says why (SPEC-515 R3, R5).
    func undoOffer() async throws -> (dialog: UndoDialog?, said: String) {
        try await engine.undoOffer()
    }

    /// The owner's confirmation of `dialog`, through the session; the sentence that says why it
    /// was not undone, or empty (SPEC-515 R4, R5).
    func undo(_ dialog: UndoDialog) async throws -> String {
        try await engine.undo(dialog)
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
