// The shell model hands the detail pane the review of a chosen deck once its first answer is in
// (SPEC-347 A16). The test opens a fresh collection of its own through the launch argument the app
// reads, so its default deck holds no card and its review ends at once, at the designed end. Its
// review reads no device voice and no VoiceOver state, as the review model's own tests do, and the
// opens must answer within 30 seconds of turns of the main actor.
import Foundation
import XCTest

@testable import DeckStreak

/// Whether the opens have returned, written by the task that runs them.
@MainActor
private final class Opened {
    var done = false
}

final class AppModelTests: XCTestCase {
    @MainActor
    func test_a16_a_chosen_decks_review_shows_once_its_first_answer_is_in() async throws {
        let directory = FileManager.default.temporaryDirectory.appending(
            path: "shell-\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let model = AppModel(review: { deck, session in
            ReviewModel(deck: deck, session: session, voiceOverRunning: { true }, installed: { [] })
        })
        let opened = Opened()
        let opening = Task { @MainActor in
            await model.start(arguments: [
                "DeckStreak", "-DSCollectionDirectory", directory.path(percentEncoded: false),
            ])
            model.chosenDeck = model.decks.first
            await model.openReview()
            opened.done = true
        }
        // Each turn of the main actor while the opens run reads the review the pane would show, for
        // at most 30 seconds: an open that stalls fails here, and its task is left cancelled.
        var handed: [ReviewPhase] = []
        let deadline = Date().addingTimeInterval(30)
        while !opened.done, Date() < deadline {
            if let phase = model.review?.phase {
                handed.append(phase)
            }
            await Task.yield()
        }
        guard opened.done else {
            opening.cancel()
            XCTFail(
                "A16: the collection and the review open within 30 seconds; the decks listed read "
                    + "\(model.decks.count) and the phases handed \(handed)")
            return
        }
        await opening.value
        if let phase = model.review?.phase {
            handed.append(phase)
        }
        _ = try XCTUnwrap(
            model.chosenDeck,
            "a fresh collection lists its default deck; the refusal reads \(model.refusal ?? "none")")
        XCTAssertEqual(
            handed.first, .finished,
            "A16: the pane is handed the review of the empty default deck at its designed end, "
                + "never while it is still loading; the phases handed read \(handed)")
    }
}
