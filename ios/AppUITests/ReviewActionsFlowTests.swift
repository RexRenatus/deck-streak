// The review's bury and flag, and the review across the iPad's sidebar, on an iPhone and an iPad
// simulator (SPEC-358 A11 and A12). Each test copies the review fixture, which the workflow step
// names to the test runner as `DS_REVIEW_FIXTURE`, into a fresh directory beside it, and launches
// the app on that copy with `-DSCollectionDirectory`. The fixture's deck `Review` holds four new
// cards, queued in the order its builder adds them: a text card, an image card, a sound card and a
// speech card, each with the back "the back". A test waits for what it reads with a bounded poll
// and then asserts the value it read, so a red quotes what the app showed when the wait ran out.
import UIKit
import XCTest

/// How long a test waits for the app to show what the engine answered before it reads it.
private let engineWait: TimeInterval = 30

/// What a reading records for an element the app does not show.
private let notShown = "(not shown)"

/// The flag button's two titles: the card carries no flag, or the red one (SPEC-358 R5).
private let unflagged = "Flag"
private let flaggedRed = "Flagged red"

final class ReviewActionsFlowTests: XCTestCase {
    @MainActor
    func test_a11_bury_moves_on_and_flag_toggles_red() throws {
        let app = try launchedOnTheFixture()
        tapWhenShown(app.staticTexts["deck-row-Review"].firstMatch)
        let textCard = app.webViews.staticTexts["a text card"].firstMatch
        let imageCard = app.webViews.images["a grey dot"].firstMatch
        let flag = app.buttons["flag"].firstMatch
        let first = reading(until: ["shown", unflagged]) {
            [shown(textCard) ? "shown" : notShown, label(of: flag)]
        }

        tapWhenShown(flag)
        let flagged = reading(until: flaggedRed) { label(of: flag) }
        tapWhenShown(flag)
        let cleared = reading(until: unflagged) { label(of: flag) }

        // The card after the text card is the image card: the engine queues the fixture's new
        // cards in the order its builder adds them, as `crates/ffi/tests/review_actions.rs` reads.
        tapWhenShown(app.buttons["bury"].firstMatch)
        let buried = reading(until: ["shown", notShown]) {
            [shown(imageCard) ? "shown" : notShown, shown(textCard) ? "shown" : notShown]
        }

        XCTAssertEqual(
            [buried, first, [flagged], [cleared]],
            [["shown", notShown], ["shown", unflagged], [flaggedRed], [unflagged]],
            "A11: Bury shows the image card in the text card's place; the text card's Flag reads "
                + "\"Flagged red\" after one press and \"Flag\" again after the next")
    }

    @MainActor
    func test_a12_the_review_survives_the_sidebar() throws {
        let app = try launchedOnTheFixture()
        let row = app.staticTexts["deck-row-Review"].firstMatch
        tapWhenShown(row)
        tapWhenShown(app.buttons["Show Answer"].firstMatch)
        // The text card's answer side: its question, its back and the Good grade.
        let side = [
            app.webViews.staticTexts["a text card"].firstMatch,
            app.webViews.staticTexts["the back"].firstMatch,
            app.buttons["Good"].firstMatch,
        ]
        let revealed = reading(until: [true, true, true]) { side.map { shown($0) } }

        if UIDevice.current.userInterfaceIdiom == .pad {
            // The iPad's form: the split view's sidebar hides and returns beside the review. Its
            // own button is found by its identifier or by either of its titles.
            let toggle = app.buttons.matching(
                NSPredicate(
                    format: "identifier == %@ OR label ==[c] %@ OR label ==[c] %@",
                    "ToggleSidebar", "Hide Sidebar", "Show Sidebar")
            ).firstMatch
            tapWhenShown(toggle)
            let hidden = reading(until: [false, true, true, true]) {
                [shown(row)] + side.map { shown($0) }
            }
            tapWhenShown(toggle)
            let returned = reading(until: [true, true, true, true]) {
                [shown(row)] + side.map { shown($0) }
            }
            XCTAssertEqual(
                [hidden, returned, revealed],
                [[false, true, true, true], [true, true, true, true], [true, true, true]],
                "A12: on the iPad the text card's answer side stays shown while the sidebar "
                    + "hides, and after it returns")
        } else {
            // The iPhone's form (compact width, no sidebar): the review stands alone, and the
            // card and its revealed side survive a turn to landscape and back to portrait.
            let alone = reading(until: false) { shown(row) }
            XCUIDevice.shared.orientation = .landscapeLeft
            let turned = reading(until: [true, true, true]) { side.map { shown($0) } }
            XCUIDevice.shared.orientation = .portrait
            let back = reading(until: [true, true, true]) { side.map { shown($0) } }
            XCTAssertEqual(
                [turned, back, revealed, [alone]],
                [[true, true, true], [true, true, true], [true, true, true], [false]],
                "A12: on the iPhone the review stands alone, and the text card's answer side "
                    + "stays shown in landscape and back in portrait")
        }
    }

    // MARK: - Steps

    /// The app, launched on a fresh copy of the review fixture, in portrait.
    @MainActor
    private func launchedOnTheFixture() throws -> XCUIApplication {
        let named = try XCTUnwrap(
            ProcessInfo.processInfo.environment["DS_REVIEW_FIXTURE"],
            "the step names the review fixture as DS_REVIEW_FIXTURE")
        let fixture = URL(filePath: named, directoryHint: .isDirectory)
        let fresh = fixture.deletingLastPathComponent().appending(
            path: "review-actions-\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.copyItem(at: fixture, to: fresh)
        XCUIDevice.shared.orientation = .portrait
        let app = XCUIApplication()
        app.launchArguments = ["-DSCollectionDirectory", fresh.path(percentEncoded: false)]
        app.launch()
        return app
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

    /// Whether an element is on screen where a tap would reach it: present in the hierarchy is not
    /// enough, because a collapsed split view keeps the column it hides.
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
