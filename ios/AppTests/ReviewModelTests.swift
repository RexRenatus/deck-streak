// The review model against the review fixture (SPEC-348 A15, A18), and the player's refusal of a
// sound it cannot play (section 10). Each model opens its own fresh copy of the fixture, whose deck
// `Review` holds a text card, an image card, a sound card and a speech card, in that order. The
// model's VoiceOver reading and installed voices are handed in, so a test decides both.
import Foundation
import XCTest

@testable import DeckStreak

/// A voice the tests install: it is never on a simulator, so its choice is the test's alone.
private let testVoice = InstalledVoice(
    identifier: "invalid.test.voice", name: "Test", language: "en-US", quality: 0)

final class ReviewModelTests: XCTestCase {
    @MainActor
    func test_a15_the_loop_reaches_the_designed_end() async throws {
        let (model, _) = try await started(voiceOver: true, installed: [])
        var phases: [ReviewPhase] = []
        for _ in 0..<4 {
            phases.append(model.phase)
            await model.perform(.showAnswer)
            phases.append(model.phase)
            await model.perform(.rate(.easy))
        }
        phases.append(model.phase)
        XCTAssertEqual(
            phases,
            [
                .question, .answer, .question, .answer, .question, .answer, .question, .answer,
                .finished,
            ],
            "A15: question, answer, rating and next, through four cards to the designed end")
    }

    @MainActor
    func test_a18_the_clip_plan_follows_the_face_and_voiceover() async throws {
        // The sound card, third, with VoiceOver off: its autoplay plan is its clip.
        let (sounding, directory) = try await started(voiceOver: false, installed: [])
        await answeredEasy(sounding, cards: 2)
        let tone = try Data(contentsOf: directory.appending(path: "collection.media/tone.wav"))
        XCTAssertEqual(
            sounding.plan, [.sound(name: "tone.wav", bytes: tone)],
            "A18: the sound card's autoplay plan is its clip")

        // The same card with VoiceOver running: nothing plays by itself.
        let (quiet, _) = try await started(voiceOver: true, installed: [])
        await answeredEasy(quiet, cards: 2)
        XCTAssertEqual(quiet.plan, [], "A18: while VoiceOver runs the autoplay plan is empty")

        // The speech card, fourth, with the chosen voice installed: the plan names it.
        let (chosen, _) = try await started(voiceOver: false, installed: [testVoice])
        await chosen.choose(voice: testVoice.identifier, for: "en-US")
        await answeredEasy(chosen, cards: 3)
        XCTAssertEqual(
            chosen.plan,
            [
                .speech(
                    text: "a spoken card", language: "en-US", rate: 0.5,
                    voice: testVoice.identifier)
            ],
            "A18: the speech card speaks with the chosen voice when it is installed")

        // The same choice with the voice absent: the plan names none, so the language's speaks.
        let (absent, _) = try await started(voiceOver: false, installed: [])
        await absent.choose(voice: testVoice.identifier, for: "en-US")
        await answeredEasy(absent, cards: 3)
        XCTAssertEqual(
            absent.plan,
            [.speech(text: "a spoken card", language: "en-US", rate: 0.5, voice: nil)],
            "A18: the speech card falls back to the language's voice when the chosen one is absent")
    }

    @MainActor
    func test_a_sound_the_player_refuses_is_named_in_one_line() {
        let player = ClipPlayer()
        var lines: [String] = []
        player.refused = { lines.append($0) }
        player.play([.sound(name: "noise.wav", bytes: Data("not a sound".utf8))])
        XCTAssertEqual(
            lines, ["The sound noise.wav could not be played."],
            "a sound the player refuses shows one line naming its file")
    }

    // MARK: - Steps

    /// A model over a fresh copy of the fixture, started on its deck `Review`, and that copy.
    @MainActor
    private func started(
        voiceOver: Bool, installed: [InstalledVoice]
    ) async throws -> (ReviewModel, URL) {
        let directory = try freshFixture()
        let (engine, review) = try await openedFixture(directory)
        let model = ReviewModel(
            deck: review, session: ReviewSession(engine: engine),
            voiceOverRunning: { voiceOver }, installed: { installed })
        await model.start()
        return (model, directory)
    }

    /// Reveals and answers Easy, which graduates a new card out of the queue, `cards` times.
    @MainActor
    private func answeredEasy(_ model: ReviewModel, cards: Int) async {
        for _ in 0..<cards {
            await model.perform(.showAnswer)
            await model.perform(.rate(.easy))
        }
    }
}
