// The review session against the review fixture (SPEC-348 A16, A17). The workflow step names the
// fixture to the test runner as `DS_REVIEW_FIXTURE`; each test copies it into a fresh directory
// beside it and opens that copy through the launch argument the app reads, so no test meets
// another's answers. The fixture's deck `Review` holds four new cards in order: a text card, an
// image card, a sound card and a speech card. Its second collection, in `occlusion/`, holds the
// deck `Occlusion` with one image occlusion card, which the session withholds (SPEC-380 A13).
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

/// The engine opened on `directory` through the app's launch argument, and its deck named `name`:
/// `Review` unless a test names another.
func openedFixture(
    _ directory: URL, deck name: String = "Review"
) async throws -> (EngineSession, Deck) {
    let engine = EngineSession()
    try await engine.open(
        arguments: ["DeckStreak", "-DSCollectionDirectory", directory.path(percentEncoded: false)])
    let decks = try await engine.decks()
    let review = try XCTUnwrap(
        decks.first { $0.name == name },
        "the fixture's deck \(name) is listed; the decks read \(decks.map(\.name))")
    return (engine, review)
}

/// The fixture's second collection (SPEC-380 R10), in its directory `occlusion` beside the first,
/// and its deck `Occlusion`, which holds one image occlusion card the engine built.
func openedOcclusion() async throws -> (EngineSession, Deck) {
    try await openedFixture(
        freshFixture().appending(path: "occlusion", directoryHint: .isDirectory),
        deck: "Occlusion")
}

/// The line a withheld card's document holds in the card's place (SPEC-380 R6, R7), as the test
/// spells it: never read from the ffi's constant.
let withheldLine =
    "This image occlusion card cannot be shown here, because this app does not draw its masks. "
    + "You can still bury or flag it."

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

    func test_an_occlusion_card_is_withheld() async throws {
        let (engine, occlusion) = try await openedOcclusion()
        let session = ReviewSession(engine: engine)
        try await session.choose(occlusion)
        // Autoplay is wished, so a shown question would hand over its header's speech.
        let step = try await session.next(autoplay: true, installed: [])
        XCTAssertEqual(
            step.phase, .withheld,
            "A13: the session answers the occlusion card in the withheld phase")
        XCTAssertEqual(
            [
                "withheld \(step.face.withheld)",
                "line \(step.face.document.contains(withheldLine))",
                "mask layer \(step.face.document.contains("image-occlusion"))",
                "image \(step.face.document.contains("<img"))",
                "clips \(step.face.autoplay.count + step.face.replay.count)",
            ],
            ["withheld true", "line true", "mask layer false", "image false", "clips 0"],
            "A13: the occlusion card's face is the withheld face: the line, no image and no clip")
    }
}
