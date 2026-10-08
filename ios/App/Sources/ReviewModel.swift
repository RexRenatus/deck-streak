import Observation
import UIKit

/// The review screen's state, held in a model and never in a view's state, so a size-class change
/// keeps it (SPEC-348 R16). Every gesture enters through `perform`, whose cases pair a gesture
/// with the phase it may start from and write the next phase before the first `await`: a second
/// tap finds the phase moved and matches nothing (R10). The session's cleared head is the second
/// fence, for a caller that bypasses the bar.
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
    /// The engine's four intervals for the shown card, in its ratings' order; the bar shows
    /// Again's and Good's (SPEC-365 R13).
    private(set) var intervals: [String] = []
    /// The clips handed to the player when the shown face appeared.
    private(set) var plan: [ReviewClip] = []
    /// Whether the shown face is the answer, so the bar offers the two grades.
    private(set) var revealed = false
    /// Whether the shown card carries the engine's red flag (SPEC-358 R5).
    private(set) var flagged = false
    /// The answers sent, raised by one with each; the impact haptic's trigger (R12).
    private(set) var answered = 0
    /// One line each: a refusal's sentence, and each sound the player refused on this face.
    private(set) var notes: [String] = []
    /// The languages the shown face speaks, with their voices, for the picker (R14).
    private(set) var voices: [VoiceLanguage] = []
    /// Whether the voice picker is shown.
    var showingVoices = false

    private let session: ReviewSession
    private let voiceOverRunning: @MainActor () -> Bool
    private let installed: @MainActor () -> [InstalledVoice]
    private let player = ClipPlayer()

    init(
        deck: Deck, session: ReviewSession,
        voiceOverRunning: @escaping @MainActor () -> Bool = { UIAccessibility.isVoiceOverRunning },
        installed: @escaping @MainActor () -> [InstalledVoice] = InstalledVoices.all
    ) {
        self.deck = deck
        self.session = session
        self.voiceOverRunning = voiceOverRunning
        self.installed = installed
        player.refused = { [weak self] line in self?.notes.append(line) }
    }

    /// Makes the deck current and shows its first card, or the designed end.
    func start() async {
        await run {
            try await session.choose(deck)
            try await next()
        }
    }

    /// The one entry for the review screen's gestures (SPEC-348 R16). Show Answer starts only
    /// from a question and a rating only from an answer; each writes its phase before it awaits.
    /// Bury and flag start from either side and write `marking`: a bury then shows the next
    /// card, and a flag returns to the side it started from (SPEC-358 R5). Replay and stop go
    /// to the player from any phase.
    func perform(_ action: ReviewAction) async {
        switch (action, phase) {
        case (.showAnswer, .question):
            phase = .revealing
            await run {
                let face = try await session.reveal(
                    autoplay: !voiceOverRunning(), installed: installed())
                show(face)
                revealed = true
                phase = .answer
            }
        case (.rate(let rating), .answer):
            phase = .answering
            await run {
                try await session.answer(rating)
                answered += 1
                try await next()
            }
        case (.bury, .question), (.bury, .answer):
            phase = .marking
            await run {
                try await session.bury()
                try await next()
            }
        case (.flag, .question), (.flag, .answer):
            let side = phase
            phase = .marking
            await run {
                flagged = try await session.flag() == session.red
                phase = side
            }
        default:
            ReviewPlayback.perform(action, face: face, player: player)
        }
    }

    /// Records the voice for a language the face speaks, from the picker (SPEC-348 R14).
    func choose(voice identifier: String?, for language: String) async {
        await run {
            try await session.choose(voice: identifier, language: language)
            await readVoices()
        }
    }

    /// Reads the shown face's languages and their voices for the picker.
    func readVoices() async {
        voices = await session.voices(face.languages, installed: installed())
    }

    /// Shows the next card's question, or the designed end.
    private func next() async throws {
        let step = try await session.next(autoplay: !voiceOverRunning(), installed: installed())
        counts = step.counts
        intervals = step.intervals
        revealed = false
        flagged = step.flag == session.red
        show(step.face)
        phase = step.phase
    }

    /// Shows `face` and plays its autoplay clips; showing a face stops what played (R13).
    private func show(_ face: ReviewFace) {
        self.face = face
        notes = []
        plan = face.autoplay
        player.play(plan)
    }

    /// Runs one step of the review; a refusal shows its sentence and ends the review (R10).
    private func run(_ work: () async throws -> Void) async {
        do {
            try await work()
        } catch {
            player.stop()
            notes = [String(describing: error)]
            phase = .refused
        }
    }
}
