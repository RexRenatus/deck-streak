// The harness's flow on an iPhone and an iPad simulator (SPEC-339 A4 to A7). Each test launches
// the app fresh, so each starts from the bundled synthetic collection's one new card. A test waits
// for what it reads with a bounded poll and then asserts the value it read, so a red quotes what
// the app showed when the wait ran out, never only that an element was missing.
import XCTest

/// How long a test waits for the app to show what the engine answered before it reads it.
private let engineWait: TimeInterval = 30

/// What a reading records for an element the app does not show.
private let notShown = "(not shown)"

final class HarnessFlowTests: XCTestCase {
    @MainActor
    func test_a4_opens_the_synthetic_collection() {
        let app = launched()
        let status = app.staticTexts["open-status"].firstMatch
        XCTAssertEqual(
            reading(until: "opened") { label(of: status) }, "opened",
            "A4: the harness opens its bundled synthetic collection at launch, and shows that it did")
    }

    @MainActor
    func test_a5_lists_the_collections_decks() {
        let app = launched()
        let rows = app.staticTexts.matching(NSPredicate(format: "identifier BEGINSWITH %@", "deck-row-"))
        XCTAssertEqual(
            reading(until: ["Default", "Synthetic"]) { rows.allElementsBoundByIndex.map(\.label) },
            ["Default", "Synthetic"],
            "A5: the deck list names Default then Synthetic, and nothing else")
    }

    @MainActor
    func test_a6_renders_the_queued_cards_question() {
        let app = launched()
        study(app)
        let card = app.webViews.firstMatch
        XCTAssertEqual(
            reading(until: "synthetic front") {
                card.exists
                    ? card.staticTexts.allElementsBoundByIndex.map(\.label).joined(separator: " ")
                    : notShown
            },
            "synthetic front",
            "A6: studying shows the queued card, and its web view shows the synthetic front")
    }

    @MainActor
    func test_a7_answers_the_card_good() {
        let app = launched()
        study(app)
        let newCount = app.staticTexts["new-count"].firstMatch
        let before = reading(until: "1") { label(of: newCount) }
        tapWhenShown(app.buttons["answer-good"].firstMatch)
        let after = reading(until: "0") { label(of: newCount) }
        XCTAssertEqual(
            [before, after], ["1", "0"],
            "A7: answering Good leaves the deck's new count at 0, where it read 1 before the answer")
    }

    // MARK: - Steps

    @MainActor
    private func launched() -> XCUIApplication {
        let app = XCUIApplication()
        app.launch()
        return app
    }

    /// Chooses the current deck and studies it. The iPhone starts on the deck list, so choosing
    /// the deck shows the study pane; the iPad shows both columns, so both taps land in place.
    @MainActor
    private func study(_ app: XCUIApplication) {
        tapWhenShown(app.staticTexts["deck-row-Default"].firstMatch)
        tapWhenShown(app.buttons["study"].firstMatch)
    }

    /// Taps an element once it is shown. One the app never shows is left for the test's assertion
    /// to report through the value it reads.
    @MainActor
    private func tapWhenShown(_ element: XCUIElement) {
        if element.waitForExistence(timeout: engineWait) {
            element.tap()
        }
    }

    /// An element's label, or `notShown` when the app does not show it.
    @MainActor
    private func label(of element: XCUIElement) -> String {
        element.exists ? element.label : notShown
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
