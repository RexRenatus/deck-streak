// The review model against the review fixture (SPEC-348 A15, A18), the player's refusal of a
// sound it cannot play (section 10), and the end of a sound the player already replaced, which
// leaves the new list where it is. Each model opens its own fresh copy of the fixture, whose deck
// `Review` holds a text card, an image card, a sound card and a speech card, in that order. The
// model's VoiceOver reading and installed voices are handed in, so a test decides both. The
// fixture's second collection holds one image occlusion card, which the model withholds: no
// reveal and no rating, only a flag and a bury (SPEC-380 A14).
import AVFoundation
import Foundation
import UIKit
import XCTest

@testable import DeckStreak

/// A poster that records each announcement the review makes, in order, in place of the system's.
@MainActor
private final class RecordingPoster {
    private(set) var notifications: [UIAccessibility.Notification] = []
    private(set) var said: [String] = []

    func post(_ notification: UIAccessibility.Notification, _ argument: Any?) {
        notifications.append(notification)
        said.append((argument as? String) ?? "")
    }
}

/// A voice the tests install: it is never on a simulator, so its choice is the test's alone.
private let testVoice = InstalledVoice(
    identifier: "invalid.test.voice", name: "Test", language: "en-US", quality: 0)

final class ReviewModelTests: XCTestCase {
    @MainActor
    func test_a15_the_loop_reaches_the_designed_end() async throws {
        let (model, _) = try await started(voiceOver: true, installed: [])
        var phases: [ReviewPhase] = []
        // Good sends each new card ten minutes into learning, so the four new cards show first
        // and then each again, within the learn-ahead limit, until a second Good graduates it.
        for _ in 0..<8 {
            phases.append(model.phase)
            await model.perform(.showAnswer)
            phases.append(model.phase)
            await model.perform(.rate(.good))
        }
        phases.append(model.phase)
        XCTAssertEqual(
            phases,
            Array(repeating: [ReviewPhase.question, .answer], count: 8).flatMap { $0 }
                + [.finished],
            "A15: question, answer, rating and next, through four cards each pressed Good twice to "
                + "the designed end")
    }

    @MainActor
    func test_a18_the_clip_plan_follows_the_face_and_voiceover() async throws {
        // The sound card, third, with VoiceOver off: its autoplay plan is its clip.
        let (sounding, directory) = try await started(voiceOver: false, installed: [])
        await answeredGood(sounding, cards: 2)
        let tone = try Data(contentsOf: directory.appending(path: "collection.media/tone.wav"))
        XCTAssertEqual(
            sounding.plan, [.sound(name: "tone.wav", bytes: tone)],
            "A18: the sound card's autoplay plan is its clip")

        // The same card with VoiceOver running: nothing plays by itself.
        let (quiet, _) = try await started(voiceOver: true, installed: [])
        await answeredGood(quiet, cards: 2)
        XCTAssertEqual(quiet.plan, [], "A18: while VoiceOver runs the autoplay plan is empty")

        // The speech card, fourth, with the chosen voice installed: the plan names it.
        let (chosen, _) = try await started(voiceOver: false, installed: [testVoice])
        await chosen.choose(voice: testVoice.identifier, for: "en-US")
        await answeredGood(chosen, cards: 3)
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
        await answeredGood(absent, cards: 3)
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

    @MainActor
    func test_a_replaced_sounds_end_leaves_the_new_list_where_it_is() async throws {
        let player = ClipPlayer()
        let quiet = silence(seconds: 4)
        player.play([.sound(name: "a.wav", bytes: quiet), .sound(name: "b.wav", bytes: quiet)])
        let replaced = try XCTUnwrap(player.sound, "the first list's first sound plays")
        player.play([.sound(name: "c.wav", bytes: quiet), .sound(name: "d.wav", bytes: quiet)])
        let current = try XCTUnwrap(player.sound, "the second list's first sound plays")

        // The replaced sound's end arrives after the second list began, as a late callback does.
        player.audioPlayerDidFinishPlaying(replaced, successfully: true)
        try await Task.sleep(for: .milliseconds(500))

        XCTAssertEqual(
            player.queue, [.sound(name: "d.wav", bytes: quiet)],
            "the replaced sound's end leaves the second list's queue where it was")
        XCTAssertTrue(player.sound === current, "the second list's first sound still plays")
    }

    @MainActor
    func test_an_undo_asks_first_keep_writes_nothing_and_the_confirmation_returns_the_card()
        async throws
    {
        // SPEC-515 A5: the review's undo of its own last answer, from the model's one entry.
        let (model, _) = try await started(voiceOver: true, installed: [])
        let opened = model.undo.label
        await answeredGood(model, cards: 1)
        let answered = model.undo.label
        let next = model.face.document
        await model.perform(.undo)
        let offered = model.undo.offer
        await model.perform(.keep)
        let kept = [model.undo.offer == nil, model.face.document == next]
        await model.perform(.undo)
        await model.perform(.confirmUndo)

        XCTAssertEqual(opened, "", "when the review opens, it has nothing of its own to undo")
        XCTAssertEqual(answered, "Undo answer", "after the answer, the control names its undo")
        XCTAssertEqual(offered?.title, "Undo this answer?", "the press asks first")
        XCTAssertEqual(
            offered?.lines,
            [
                "Card: a text card", "Your answer: Good", "Goes back to: New, in today's queue",
                "This answer has not synced yet. Undoing it removes it from your review history; "
                    + "nothing else changes.",
            ],
            "the dialog names the card as text, the answer and the state it goes back to")
        XCTAssertEqual(kept, [true, true], "Keep drops the offer and writes nothing")
        XCTAssertEqual(model.phase, .question, "the confirmation shows the next queued card")
        XCTAssertTrue(
            model.face.document.contains("a text card"), "the undone card is queued first again")
        XCTAssertEqual(model.undo.label, "", "the undo emptied the review's slot")
    }

    @MainActor
    func test_a_refused_undo_says_why() async throws {
        // SPEC-515 A5: a confirmation after a later change is refused, and the review says why.
        let (model, _) = try await started(voiceOver: true, installed: [])
        await answeredGood(model, cards: 1)
        await model.perform(.undo)
        // A later change before the confirmation: the image card, shown now, is flagged red.
        await model.perform(.flag)
        await model.perform(.confirmUndo)

        XCTAssertEqual(
            model.undo.said,
            ["This answer can no longer be undone: something changed after it."],
            "the refused confirmation says why")
        XCTAssertEqual(model.undo.label, "Undo flag", "the flag is what the review offers now")
        XCTAssertTrue(model.flagged, "the later flag stands")
    }

    @MainActor
    func test_a_withheld_card_reveals_nothing_and_takes_no_rating() async throws {
        let (engine, occlusion) = try await openedOcclusion()
        let session = ReviewSession(engine: engine)
        let model = ReviewModel(
            deck: occlusion, session: session, voiceOverRunning: { true }, installed: { [] })
        await model.start()
        let shown = model.face

        // Show Answer on the withheld card: no answer face, and no reveal.
        await model.perform(.showAnswer)
        XCTAssertEqual(
            "\(model.phase) revealed \(model.revealed) face kept \(model.face == shown)",
            "withheld revealed false face kept true",
            "A14: Show Answer on a withheld card reveals nothing")

        // Every rating: nothing sent, and the engine still holds the card new.
        for rating in Rating.allCases {
            await model.perform(.rate(rating))
        }
        let sent = await session.sentAnswers
        let after = try await ReviewSession(engine: engine).next(autoplay: false, installed: [])
        XCTAssertEqual(
            "\(model.phase) answered \(model.answered) sent \(sent) "
                + "new \(after.counts.new) learning \(after.counts.learning)",
            "withheld answered 0 sent 0 new 1 learning 0",
            "A14: no rating is taken on a withheld card, and it stays due")

        // A flag marks the card and returns to it; a bury moves on, here to the designed end.
        await model.perform(.flag)
        let flag = "\(model.phase) \(model.flagged)"
        await model.perform(.bury)
        XCTAssertEqual(
            "flag \(flag) bury \(model.phase)", "flag withheld true bury finished",
            "A14: a flag keeps the withheld card, and a bury moves on to the designed end")
    }

    /// SPEC-680 R4: a grade pressed as its answer shows, inside the settle interval, is not sent,
    /// and the answer stays shown; the control, with no interval, sends the same press.
    @MainActor
    func test_a_rating_pressed_as_the_answer_shows_is_not_sent() async throws {
        let (engine, review) = try await openedFixture(freshFixture())
        let model = ReviewModel(
            deck: review, session: ReviewSession(engine: engine), settle: .seconds(3600),
            voiceOverRunning: { true }, installed: { [] })
        await model.start()
        await model.perform(.showAnswer)
        await model.perform(.rate(.good))
        XCTAssertEqual(model.answered, 0, "a grade pressed as the answer shows is not sent")
        XCTAssertEqual(model.phase, .answer, "the answer stays shown")

        // the control: with no settle interval the same press is sent
        let (open, _) = try await started(voiceOver: true, installed: [])
        await open.perform(.showAnswer)
        await open.perform(.rate(.good))
        XCTAssertEqual(open.answered, 1, "with no settle interval the grade is sent")
    }

    /// SPEC-680 R6: the review announces the card shown, the answer when it shows, the grade sent
    /// and the next card after it, through to the finish in its words, each once and each as an
    /// announcement, through the poster a test hands in.
    @MainActor
    func test_the_review_announces_each_answer_grade_card_and_finish() async throws {
        let (engine, review) = try await openedFixture(freshFixture())
        let poster = RecordingPoster()
        let model = ReviewModel(
            deck: review, session: ReviewSession(engine: engine),
            post: { notification, argument in poster.post(notification, argument) },
            voiceOverRunning: { true }, installed: { [] })
        await model.start()
        XCTAssertEqual(poster.said.count, 1, "the first card is announced: \(poster.said)")
        await model.perform(.showAnswer)
        XCTAssertEqual(poster.said.count, 2, "the answer is announced: \(poster.said)")
        await model.perform(.rate(.good))
        XCTAssertEqual(poster.said.count, 4, "the grade and the next card: \(poster.said)")
        await answeredGood(model, cards: 7)
        XCTAssertEqual(model.phase, .finished, "eight grades finish the deck")
        XCTAssertEqual(poster.said.count, 25, "each answer, grade and card or finish once")
        XCTAssertEqual(poster.said.last, FinishWords.text(answered: 8), "the finish's words")
        XCTAssertEqual(
            poster.notifications,
            Array(repeating: UIAccessibility.Notification.announcement, count: 25))
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

    /// Reveals and answers Good, `cards` times: each press sends a new card ten minutes into
    /// learning, so the next new card shows next (SPEC-365 R13).
    @MainActor
    private func answeredGood(_ model: ReviewModel, cards: Int) async {
        for _ in 0..<cards {
            await model.perform(.showAnswer)
            await model.perform(.rate(.good))
        }
    }
}

/// A WAVE file of `seconds` of silence: 8000 samples a second, 16 bits, one channel.
private func silence(seconds: Int) -> Data {
    func littleEndian(_ value: Int, _ count: Int) -> [UInt8] {
        (0..<count).map { UInt8(truncatingIfNeeded: value >> (8 * $0)) }
    }
    let size = 2 * 8000 * seconds
    let pieces: [[UInt8]] = [
        Array("RIFF".utf8), littleEndian(36 + size, 4), Array("WAVE".utf8),
        Array("fmt ".utf8), littleEndian(16, 4), littleEndian(1, 2), littleEndian(1, 2),
        littleEndian(8000, 4), littleEndian(16000, 4), littleEndian(2, 2), littleEndian(16, 2),
        Array("data".utf8), littleEndian(size, 4), [UInt8](repeating: 0, count: size),
    ]
    var bytes = Data()
    for piece in pieces { bytes.append(contentsOf: piece) }
    return bytes
}
