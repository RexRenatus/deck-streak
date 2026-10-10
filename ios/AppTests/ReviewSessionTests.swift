// The review session against the review fixture (SPEC-348 A16, A17). The workflow step names the
// fixture to the test runner as `DS_REVIEW_FIXTURE`; each test copies it into a fresh directory
// beside it and opens that copy through the launch argument the app reads, so no test meets
// another's answers. The fixture's deck `Review` holds four new cards in order: a text card, an
// image card, a sound card and a speech card. Beside it, `parity/` is the parity collection
// (SPEC-393 R15), whose deck `Parity` opens with a card that speaks its front through a TTS tag
// asking for the voices `Absent_Voice` and `Desk_Parity_Voice`.
import Foundation
import XCTest

@testable import DeckStreak

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
}
