// The review session against the review fixture (SPEC-348 A16, A17). The workflow step names the
// fixture to the test runner as `DS_REVIEW_FIXTURE`; each test copies it into a fresh directory
// beside it and opens that copy through the launch argument the app reads, so no test meets
// another's answers. The fixture's deck `Review` holds four new cards in order: a text card, an
// image card, a sound card and a speech card. Beside it, `parity/` is the parity collection
// (SPEC-393 R15), whose deck `Parity` opens with a card that speaks its front through a TTS tag
// asking for the voices `Absent_Voice` and `Desk_Parity_Voice`; `sound-video/` is the sound
// video collection (SPEC-410 R8), whose deck `Sound video` holds one card whose question's sound
// tags name two videos and a sound; and `health/` is the card health collection (SPEC-507 R17),
// whose deck `Health` holds one review card with 207 review rows, counters apart from its log and a
// stored FSRS memory state.
import Foundation
import XCTest

@testable import DeckStreak

/// The review fixture's text card, which its builder fixes at this id and its deck shows first.
private let textCardID: Int64 = 1_000_007

/// A clock a test moves by hand; the session reads it from its own actor.
final class HandClock: @unchecked Sendable {
    private let lock = NSLock()
    private var instant = Date(timeIntervalSince1970: 1_800_000_000)

    func now() -> Date {
        lock.withLock { instant }
    }

    func advance(_ seconds: TimeInterval) {
        lock.withLock { instant = instant.addingTimeInterval(seconds) }
    }
}

/// A fresh copy of the review fixture: a directory holding `collection.anki2` and its
/// `collection.media` folder. A missing fixture fails the test; it never skips it.
func freshFixture() throws -> URL {
    let named = try XCTUnwrap(
        ProcessInfo.processInfo.environment["DS_REVIEW_FIXTURE"],
        "the step names the review fixture as DS_REVIEW_FIXTURE")
    let fixture = URL(filePath: named, directoryHint: .isDirectory)
    let fresh = fixture.deletingLastPathComponent().appending(
        path: "review-\(UUID().uuidString)", directoryHint: .isDirectory)
    try FileManager.default.copyItem(at: fixture, to: fresh)
    return fresh
}

/// The engine opened on `directory` through the app's launch argument, and its deck `Review`.
func openedFixture(_ directory: URL) async throws -> (EngineSession, Deck) {
    let engine = EngineSession()
    try await engine.open(
        arguments: ["DeckStreak", "-DSCollectionDirectory", directory.path(percentEncoded: false)])
    let decks = try await engine.decks()
    let review = try XCTUnwrap(
        decks.first { $0.name == "Review" },
        "the fixture's deck Review is listed; the decks read \(decks.map(\.name))")
    return (engine, review)
}

final class ReviewSessionTests: XCTestCase {
    func test_a16_a_double_tap_answers_once() async throws {
        let (engine, review) = try await openedFixture(freshFixture())
        let session = ReviewSession(engine: engine)
        try await session.choose(review)
        let first = try await session.next(autoplay: false, installed: [])
        _ = try await session.reveal(autoplay: false, installed: [])

        // Two taps on Good, both in flight at once.
        async let tap: Void = session.answer(.good)
        async let again: Void = session.answer(.good)
        _ = try await (tap, again)

        let sent = await session.sentAnswers
        let second = try await session.next(autoplay: false, installed: [])
        XCTAssertEqual(
            [
                first.face.document.contains("a text card"), sent == 1,
                second.face.document.contains("a grey dot"),
            ],
            [true, true, true],
            "A16: the first card is the text card, one answer was sent (\(sent)), and the next "
                + "card is the image card")
    }

    func test_a17_each_rating_shows_its_interval() async throws {
        let (engine, review) = try await openedFixture(freshFixture())
        let session = ReviewSession(engine: engine)
        try await session.choose(review)
        let step = try await session.next(autoplay: false, installed: [])
        XCTAssertEqual(
            Rating.allCases.map { step.intervals.dropFirst(Int($0.rawValue)).prefix(1).joined() },
            ["<1m", "<10m"],
            "A17: Again and Good carry the intervals A1 pins for a new card, each picked by its "
                + "number as the bar picks it (SPEC-365 R13)")
    }

    /// SPEC-393 A14 (R8, R9): with no voice kept, the parity card's replay speaks its front in the
    /// installed voice `Parity Voice`, which its tag's second requested voice names.
    func test_the_parity_card_speaks_the_voice_its_template_asks() async throws {
        let directory = try freshFixture().appending(path: "parity", directoryHint: .isDirectory)
        let engine = EngineSession()
        try await engine.open(
            arguments: ["DeckStreak", "-DSCollectionDirectory", directory.path(percentEncoded: false)])
        let decks = try await engine.decks()
        let parity = try XCTUnwrap(
            decks.first { $0.name == "Parity" },
            "the parity collection's deck Parity is listed; the decks read \(decks.map(\.name))")
        let session = ReviewSession(engine: engine)
        try await session.choose(parity)
        let installed = [
            InstalledVoice(
                identifier: "test.voice.parity", name: "Parity Voice", language: "en-US", quality: 0)
        ]
        let first = try await session.next(autoplay: false, installed: installed)
        XCTAssertEqual(
            first.face.replay,
            [.speech(text: "a parity card", language: "en-US", rate: 0.5, voice: "test.voice.parity")],
            "A14: the parity card's replay speaks its front in the voice its tag asks for")
    }

    /// SPEC-410 A6 (R9): the sound video collection's first face shows sign's video, then wave's,
    /// each inline over its own bytes, and replays its one sound.
    func test_a_sound_tags_video_shows_in_the_review_screens_face() async throws {
        let directory = try freshFixture().appending(
            path: "sound-video", directoryHint: .isDirectory)
        let engine = EngineSession()
        try await engine.open(
            arguments: ["DeckStreak", "-DSCollectionDirectory", directory.path(percentEncoded: false)])
        let decks = try await engine.decks()
        let soundVideo = try XCTUnwrap(
            decks.first { $0.name == "Sound video" },
            "the sound video collection's deck Sound video is listed; the decks read "
                + "\(decks.map(\.name))")
        let session = ReviewSession(engine: engine)
        try await session.choose(soundVideo)
        let first = try await session.next(autoplay: false, installed: [])
        let document = first.face.document
        let signVideo: String =
            #"<video playsinline controls style="display:block;max-width:100%;height:auto;"#
            + #"margin:0 auto" src="data:video/mp4;base64,c2lnbi12aWRlbw=="></video>"#
        let waveVideo: String =
            #"<video playsinline controls style="display:block;max-width:100%;height:auto;"#
            + #"margin:0 auto" src="data:video/mp4;base64,d2F2ZS12aWRlbw=="></video>"#
        let sign = try XCTUnwrap(
            document.range(of: signVideo), "A6: the document holds sign's video: \(document)")
        let wave = try XCTUnwrap(
            document.range(of: waveVideo), "A6: the document holds wave's video: \(document)")
        XCTAssertEqual(
            sign.upperBound <= wave.lowerBound, true, "A6: sign's video comes before wave's")
        XCTAssertEqual(
            first.face.replay, [.sound(name: "beep.mp3", bytes: Data("beep-sound".utf8))],
            "A6: the first face replays its one sound")
    }

    /// SPEC-507 B5 (T-SS-17; R13, R14, R17): the card health collection's card, shown by its
    /// deck, reads its record in the panel's sections through the app's session: its counters
    /// beside the log's counts, its capped times, where it came from, and the newest 200 of its
    /// 207 rows newest first, the hour's answer at 60 seconds and the negative time at 0.
    func test_the_card_info_panel_shows_the_health_cards_record() async throws {
        let session = try await shownHealthCard()
        let sections = try await session.health()
        let history = sections.last?.lines ?? []
        XCTAssertEqual(
            sections.map(\.title),
            ["Counts and times", "Recall", "Where it came from", "Newest reviews"],
            "B5: the panel's four sections, in order")
        XCTAssertEqual(
            sections.dropLast().map(\.lines),
            [
                [
                    "Reviews: 8", "Lapses: 1",
                    "Time answering, each answer at most 60 s: 130.0 s",
                    "Mean answer time: 21.7 s", "Median answer time: 16.0 s",
                    "The review log holds 6 answers and 1 lapses.",
                ],
                ["Not available in this build."],
                ["Came through your collection, created 2023-11-14."],
            ],
            "B5: the counts and the capped times, the recall line, and where the card came from")
        XCTAssertEqual(
            Array(history.prefix(8)) + [String(history.count)],
            [
                "The newest 200 of 207 reviews.", "2023-11-14, Good, 30.0 s",
                "2023-11-14, Good, 20.0 s", "2023-11-14, Set by hand, 0.0 s",
                "2023-11-14, Again, 12.0 s", "2023-11-14, Good, 60.0 s",
                "2023-11-14, Again, 0.0 s", "2023-11-14, Good, 8.0 s", "201",
            ],
            "B5: the newest 200 of 207 rows, newest first, each with its capped time")
    }

    /// SPEC-507 B5 (T-SS-18; R6, R13): the card stores an FSRS memory state, stability 12.34 days
    /// and difficulty 5.6. This build's scheduler gives no readout, so the recall section reads the
    /// absent state's line alone, and no line of the panel carries the stored state's figures.
    func test_no_figure_of_the_stored_state_reaches_the_recall_lines() async throws {
        let session = try await shownHealthCard()
        let sections = try await session.health()
        let recall = sections.first { $0.title == "Recall" }?.lines ?? []
        let stored = sections.flatMap(\.lines).filter { $0.contains("12.3") || $0.contains("5.6") }
        XCTAssertEqual(
            [recall, stored, [String(sections.flatMap(\.lines).count)]],
            [["Not available in this build."], [], ["209"]],
            "B5: the recall section reads the absent state's line alone, no line carries the "
                + "stored stability or difficulty, and the panel's 209 lines were read")
    }

    /// SPEC-507 B6 (R15): the review fixture's text card shown, the card info panel held open 30
    /// of the 40 seconds before Good, the engine records 10 seconds, which the card's panel then
    /// shows in its newest row; a clock that ran through the panel would send 40.
    @MainActor
    func test_the_card_info_panel_pauses_the_answer_clock() async throws {
        let (engine, review) = try await openedFixture(freshFixture())
        let clock = HandClock()
        let session = ReviewSession(engine: engine, now: { clock.now() })
        let panel = CardHealthModel(session: session)
        try await session.choose(review)
        _ = try await session.next(autoplay: false, installed: [])
        clock.advance(5)
        await panel.open()
        let opened = [String(panel.showing), panel.sections.first?.lines.first ?? "(no line)"]
        clock.advance(30)
        await panel.close()
        clock.advance(5)
        _ = try await session.reveal(autoplay: false, installed: [])
        try await session.answer(.good)
        let after = try await engine.cardHealth(textCardID)
        let newest = after.last?.lines.last ?? "(no row)"
        XCTAssertEqual(
            opened + [
                String(panel.showing), after.first?.lines.first ?? "(no line)",
                String(newest.hasSuffix(", Good, 10.0 s")),
            ],
            ["true", "Reviews: 0", "false", "Reviews: 1", "true"],
            "B6: the panel opened on the text card and closed, and Good after 40 seconds, 30 of "
                + "them in the panel, records 10 seconds: the newest row reads \(newest)")
    }

    // MARK: - Steps

    /// A session over a fresh copy of the card health collection, `health/` beside the review
    /// fixture, with its deck `Health` chosen and its one card shown.
    private func shownHealthCard() async throws -> ReviewSession {
        let directory = try freshFixture().appending(path: "health", directoryHint: .isDirectory)
        let engine = EngineSession()
        try await engine.open(
            arguments: ["DeckStreak", "-DSCollectionDirectory", directory.path(percentEncoded: false)])
        let decks = try await engine.decks()
        let health = try XCTUnwrap(
            decks.first { $0.name == "Health" },
            "the card health collection's deck Health is listed; the decks read "
                + "\(decks.map(\.name))")
        let session = ReviewSession(engine: engine)
        try await session.choose(health)
        let step = try await session.next(autoplay: false, installed: [])
        XCTAssertEqual(step.phase, .question, "the deck's queue shows its one card")
        return session
    }
}
