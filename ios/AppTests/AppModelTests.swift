// The shell model hands the detail pane the review of a chosen deck once its first answer is in
// (SPEC-347 A16). The test opens a fresh collection of its own through the launch argument the app
// reads, so its default deck holds no card and its review ends at once, at the designed end.
import Foundation
import XCTest

@testable import DeckStreak

/// Whether the open has returned, written by the task that runs it.
@MainActor
private final class Opened {
    var done = false
}

final class AppModelTests: XCTestCase {
    @MainActor
    func test_a16_a_chosen_decks_review_shows_once_its_first_answer_is_in() async throws {
        let directory = FileManager.default.temporaryDirectory.appending(
            path: "shell-\(UUID().uuidString)", directoryHint: .isDirectory)
        let model = AppModel()
        await model.start(
            arguments: ["DeckStreak", "-DSCollectionDirectory", directory.path(percentEncoded: false)])
        model.chosenDeck = try XCTUnwrap(
            model.decks.first,
            "a fresh collection lists its default deck; the refusal reads \(model.refusal ?? "none")")
        let opened = Opened()
        let opening = Task { @MainActor in
            await model.openReview()
            opened.done = true
        }
        // Each turn of the main actor while the open runs reads the review the pane would show.
        var handed: [ReviewPhase] = []
        let deadline = Date().addingTimeInterval(30)
        while !opened.done, Date() < deadline {
            if let phase = model.review?.phase {
                handed.append(phase)
            }
            await Task.yield()
        }
        await opening.value
        if let phase = model.review?.phase {
            handed.append(phase)
        }
        XCTAssertEqual(
            handed.first, .finished,
            "A16: the pane is handed the review of the empty default deck at its designed end, "
                + "never while it is still loading; the phases handed read \(handed)")
    }
}
