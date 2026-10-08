// The review screen on an iPhone and an iPad simulator (SPEC-348 A19 to A22). Each test copies the
// review fixture, which the workflow step names to the test runner as `DS_REVIEW_FIXTURE`, into a
// fresh directory beside it, and launches the app on that copy with `-DSCollectionDirectory`. The
// fixture's deck `Review` holds four new cards: a text card, an image card, a sound card and a
// speech card. A test waits for what it reads with a bounded poll and then asserts the value it
// read, so a red quotes what the app showed when the wait ran out.
import AVFAudio
import UIKit
import XCTest

/// How long a test waits for the app to show what the engine answered before it reads it.
private let engineWait: TimeInterval = 30

/// The ratings' titles, in the bar's order, and the intervals A1 pins for a new card.
private let titles = ["Again", "Hard", "Good", "Easy"]
private let intervals = ["<1m", "<6m", "<10m", "4d"]

/// What A19 reads of one rating button.
private struct RatingReading: Equatable {
    var name: String
    var value: String
    var inTheBottomFifth: Bool
    var atLeast44By44: Bool
}

final class ReviewFlowTests: XCTestCase {
    @MainActor
    func test_a19_the_ratings_sit_at_the_bottom_named_by_their_titles() throws {
        let app = try launchedOnTheFixture()
        tapWhenShown(app.staticTexts["deck-row-Review"].firstMatch)
        tapWhenShown(app.buttons["Show Answer"].firstMatch)
        let buttons = titles.map { app.buttons[$0].firstMatch }
        _ = reading(until: Array(repeating: true, count: 4)) { buttons.map { shown($0) } }
        let window = app.windows.firstMatch.frame
        let read = buttons.map { button in
            RatingReading(
                name: button.label, value: (button.value as? String) ?? "",
                inTheBottomFifth: button.frame.minY >= window.maxY - window.height / 5,
                atLeast44By44: button.frame.width >= 44 && button.frame.height >= 44)
        }
        XCTAssertEqual(
            read,
            zip(titles, intervals).map {
                RatingReading(name: $0, value: $1, inTheBottomFifth: true, atLeast44By44: true)
            },
            "A19: the four ratings sit in the bottom fifth, at least 44 by 44 pt, named by their "
                + "titles, their values the intervals")
    }

    @MainActor
    func test_a20_show_reveal_rate_next() throws {
        let app = try launchedOnTheFixture()
        tapWhenShown(app.staticTexts["deck-row-Review"].firstMatch)
        let counts = ["count-new", "count-learning", "count-review"].map {
            app.staticTexts[$0].firstMatch
        }
        let textCard = app.webViews.staticTexts["a text card"].firstMatch
        let first = reading(until: ["4 new", "0 learning", "0 to review", "shown"]) {
            counts.map { label(of: $0) } + [shown(textCard) ? "shown" : notShown]
        }

        tapWhenShown(app.buttons["Show Answer"].firstMatch)
        let ratings = titles.map { app.buttons[$0].firstMatch }
        let revealed = reading(until: Array(repeating: true, count: 4)) {
            ratings.map { shown($0) }
        }

        tapWhenShown(app.buttons["Good"].firstMatch)
        let imageCard = app.webViews.images["a grey dot"].firstMatch
        let next = reading(until: true) { shown(imageCard) }

        XCTAssertEqual(
            [first, revealed.map { $0 ? "shown" : notShown }, [next ? "shown" : notShown]],
            [
                ["4 new", "0 learning", "0 to review", "shown"],
                Array(repeating: "shown", count: 4),
                ["shown"],
            ],
            "A20: the deck shows its counts and the text card; Show Answer reveals the four "
                + "ratings; Good shows the image card")
    }

    @MainActor
    func test_a21_the_image_card_renders_its_image() throws {
        let app = try launchedOnTheFixture()
        tapWhenShown(app.staticTexts["deck-row-Review"].firstMatch)
        answerEasy(app, cards: 1)
        let image = app.webViews.images["a grey dot"].firstMatch
        let size = reading(until: [64, 64]) {
            image.exists ? [image.frame.width.rounded(), image.frame.height.rounded()] : [0, 0]
        }
        XCTAssertEqual(
            size, [64, 64],
            "A21: the image named by its alt text is the fixture PNG's 64 by 64, not a broken "
                + "image's size")
    }

    @MainActor
    func test_a22_the_voice_picker_names_the_default() throws {
        let app = try launchedOnTheFixture()
        tapWhenShown(app.staticTexts["deck-row-Review"].firstMatch)
        answerEasy(app, cards: 3)
        tapWhenShown(app.buttons["Voices"].firstMatch)

        // The speech card speaks en-US; the sentence shows when no English voice is installed.
        let installed = AVSpeechSynthesisVoice.speechVoices().contains {
            $0.language.split(separator: "-").first == "en"
        }
        let systemDefault = app.buttons["System default"].firstMatch
        let sentence = app.staticTexts["no-voice-en-US"].firstMatch
        let read = reading(until: [true, !installed]) { [shown(systemDefault), shown(sentence)] }
        XCTAssertEqual(
            read, [true, !installed],
            "A22: the picker lists System default, and says so when no voice is installed "
                + "(installed: \(installed))")
    }

    // MARK: - Steps

    /// The app, launched on a fresh copy of the review fixture.
    @MainActor
    private func launchedOnTheFixture() throws -> XCUIApplication {
        let named = try XCTUnwrap(
            ProcessInfo.processInfo.environment["DS_REVIEW_FIXTURE"],
            "the step names the review fixture as DS_REVIEW_FIXTURE")
        let fixture = URL(filePath: named, directoryHint: .isDirectory)
        let fresh = fixture.deletingLastPathComponent().appending(
            path: "review-\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.copyItem(at: fixture, to: fresh)
        let app = XCUIApplication()
        app.launchArguments = ["-DSCollectionDirectory", fresh.path(percentEncoded: false)]
        app.launch()
        return app
    }

    /// Reveals and answers Easy, which graduates a new card out of the queue, `cards` times.
    @MainActor
    private func answerEasy(_ app: XCUIApplication, cards: Int) {
        for _ in 0..<cards {
            tapWhenShown(app.buttons["Show Answer"].firstMatch)
            tapWhenShown(app.buttons["Easy"].firstMatch)
        }
    }

    /// Taps an element once it is shown and enabled. One the app never shows is left for the
    /// test's assertion to report through the value it reads.
    @MainActor
    private func tapWhenShown(_ element: XCUIElement) {
        _ = reading(until: true) { shown(element) && element.isEnabled }
        if shown(element) {
            element.tap()
        }
    }

    /// An element's label, or `notShown` when the app does not show it.
    @MainActor
    private func label(of element: XCUIElement) -> String {
        element.exists ? element.label : notShown
    }

    /// Whether an element is on screen where a tap would reach it.
    @MainActor
    private func shown(_ element: XCUIElement) -> Bool {
        element.exists && element.isHittable
    }

    /// Reads until the reading is `expected` or `engineWait` runs out, and returns the last
    /// reading, so the assertion quotes what was shown when the wait ended.
    @MainActor
    private func reading<Value: Equatable>(until expected: Value, _ read: () -> Value) -> Value {
        let deadline = Date().addingTimeInterval(engineWait)
        var last = read()
        while last != expected, Date() < deadline {
            Thread.sleep(forTimeInterval: 0.5)
            last = read()
        }
        return last
    }
}

/// What a reading records for an element the app does not show.
private let notShown = "(not shown)"
