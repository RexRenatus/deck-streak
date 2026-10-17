// The app's shell on an iPhone and an iPad simulator (SPEC-347 A8, A9, A11). Each test launches the
// app, whose collection a fresh install's engine creates holding its default deck alone. A test
// waits for what it reads with a bounded poll and then asserts the value it read, so a red quotes
// what the app showed when the wait ran out, never only that an element was missing.
import UIKit
import XCTest

/// How long a test waits for the app to show what the engine answered before it reads it.
private let engineWait: TimeInterval = 30

/// How long a login waits for the engine's refusal: the engine resolves a reserved host and meets
/// its own network timeouts, which take longer than a local call.
private let loginWait: TimeInterval = 90

/// What a reading records for an element the app does not show.
private let notShown = "(not shown)"

/// The text A9 types as the password, which no message may repeat.
private let plantedPassword = "plantedplanted"

/// The sentences the engine's network refusals begin with, one of which a refused login shows.
private let networkSentences = [
    "A network error occurred.", "Please check your internet connection.", "Connection timed out.",
]

/// Whether a message begins with one of the engine's network sentences.
private func startsWithANetworkSentence(_ message: String) -> Bool {
    networkSentences.contains { message.hasPrefix($0) }
}

/// What A9 reads on the sheet once a login has been refused.
private struct RefusedLogin: Equatable {
    var showsTheEnginesMessage: Bool
    var namesThePassword: Bool
    var signedIn: Bool
}

final class ShellFlowTests: XCTestCase {
    @MainActor
    func test_a8_a_fresh_install_lists_the_engines_default_deck() {
        let app = launched()
        let rows = app.staticTexts.matching(NSPredicate(format: "identifier BEGINSWITH %@", "deck-row-"))
        XCTAssertEqual(
            reading(until: ["Default"]) { rows.allElementsBoundByIndex.map(\.label) },
            ["Default"],
            "A8: a fresh install lists exactly one deck, the engine's default; the refusal reads "
                + label(of: app.staticTexts["refusal"].firstMatch))
    }

    @MainActor
    func test_a9_a_refused_login_shows_the_engines_message_and_stays_signed_out() {
        let app = launched()
        tapWhenShown(app.buttons["account"].firstMatch)
        let password = app.secureTextFields["password"].firstMatch
        tapWhenShown(password)
        password.typeText(plantedPassword)
        tapWhenShown(app.buttons["sign-in"].firstMatch)
        let shown = reading(within: loginWait, until: startsWithANetworkSentence) {
            label(of: app.staticTexts["account-message"].firstMatch)
        }
        XCTAssertEqual(
            RefusedLogin(
                showsTheEnginesMessage: startsWithANetworkSentence(shown),
                namesThePassword: shown.contains(plantedPassword),
                signedIn: app.staticTexts["account-status"].exists),
            RefusedLogin(showsTheEnginesMessage: true, namesThePassword: false, signedIn: false),
            "A9: a refused login shows the engine's message, which names no password, and stays "
                + "signed out; the sheet read: " + shown)

        // Nothing was stored: a relaunch reads signed out, with the sign-in button offered.
        app.terminate()
        app.launch()
        tapWhenShown(app.buttons["account"].firstMatch)
        XCTAssertEqual(
            [
                app.buttons["sign-in"].waitForExistence(timeout: engineWait),
                app.staticTexts["account-status"].exists,
            ],
            [true, false],
            "A9: after a relaunch the sheet reads signed out, so no item was stored")
    }

    @MainActor
    func test_a11_the_split_view_follows_the_size_class() {
        let app = launched()
        let row = app.staticTexts["deck-row-Default"].firstMatch
        let detail = app.staticTexts["detail"].firstMatch
        if UIDevice.current.userInterfaceIdiom == .pad {
            XCTAssertEqual(
                reading(until: [true, true]) { [shown(row), shown(detail)] },
                [true, true],
                "A11: on the iPad the deck list and the detail show side by side")
        } else {
            let before = reading(until: [true, false]) { [shown(row), shown(detail)] }
            tapWhenShown(row)
            let after = reading(until: [false, true]) { [shown(row), shown(detail)] }
            XCTAssertEqual(
                [before, after], [[true, false], [false, true]],
                "A11: on the iPhone the deck list shows first, and choosing a deck shows the detail")
        }
    }

    // MARK: - Steps

    @MainActor
    private func launched() -> XCUIApplication {
        let app = XCUIApplication()
        app.launch()
        return app
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
        reading(within: engineWait, until: { $0 == expected }, read)
    }

    /// Reads until `done` holds for the reading or `wait` runs out, and returns the last reading.
    @MainActor
    private func reading<Value>(
        within wait: TimeInterval, until done: (Value) -> Bool, _ read: () -> Value
    ) -> Value {
        let deadline = Date().addingTimeInterval(wait)
        var last = read()
        while !done(last), Date() < deadline {
            Thread.sleep(forTimeInterval: 0.5)
            last = read()
        }
        return last
    }
}
