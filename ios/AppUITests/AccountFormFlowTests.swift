// The account form on an iPhone and an iPad simulator (SPEC-680 R7). The app launches with no
// fixture, as a fresh install does. The password field's return key starts the sign-in, as the
// sign-in button does: the engine refuses the planted login over the network, and the sheet shows
// its message. A test waits for what it reads with a bounded poll and then asserts the value it
// read, so a red quotes what the sheet showed when the wait ran out.
import XCTest

/// How long a test waits for the app to show what the engine answered before it reads it.
private let engineWait: TimeInterval = 30

/// How long a login waits for the engine's refusal: the engine resolves a reserved host and meets
/// its own network timeouts, which take longer than a local call.
private let loginWait: TimeInterval = 90

/// What a reading records for an element the app does not show.
private let notShown = "(not shown)"

/// The sentences the engine's network refusals begin with, one of which a refused login shows.
private let networkSentences = [
    "A network error occurred.", "Please check your internet connection.", "Connection timed out.",
]

/// Whether a message begins with one of the engine's network sentences.
private func startsWithANetworkSentence(_ message: String) -> Bool {
    networkSentences.contains { sentence in message.hasPrefix(sentence) }
}

final class AccountFormFlowTests: XCTestCase {
    @MainActor
    func test_the_return_key_on_the_password_field_signs_in() {
        let app = XCUIApplication()
        app.launch()
        tapWhenShown(app.buttons["account"].firstMatch)
        let password = app.secureTextFields["password"].firstMatch
        tapWhenShown(password)
        // The newline is the keyboard's return key, whatever its title reads.
        password.typeText("plantedplanted\n")
        let shown = reading(within: loginWait, until: startsWithANetworkSentence) {
            label(of: app.staticTexts["account-message"].firstMatch)
        }
        XCTAssertTrue(
            startsWithANetworkSentence(shown),
            "the password field's return key starts the sign-in, and the refused login's message "
                + "shows; the sheet read: " + shown)
    }

    // MARK: - Steps

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
