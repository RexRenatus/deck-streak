// A new install's first run on an iPhone and an iPad simulator (SPEC-680 R2). The app launches with
// no fixture, so its engine creates a collection holding its default deck alone, as a fresh install
// does. Once the decks are listed, the deck list offers to sync the collection from the account,
// and the offer opens the account sheet, where the sign-in is. A test waits for what it reads with
// a bounded wait and then asserts it.
import XCTest

/// How long a test waits for the app to show what the engine answered before it reads it.
private let engineWait: TimeInterval = 30

final class FirstRunFlowTests: XCTestCase {
    @MainActor
    func test_a_fresh_install_offers_sync_from_the_account_sheet() {
        let app = XCUIApplication()
        app.launch()
        let offer = app.buttons["first-run-sync"].firstMatch
        XCTAssertTrue(
            offer.waitForExistence(timeout: engineWait),
            "a fresh install offers to sync the collection once its decks are listed")
        XCTAssertFalse(
            app.activityIndicators["first-run-loading"].exists,
            "the loading state has gone once the offer shows")
        if offer.exists {
            offer.tap()
        }
        XCTAssertTrue(
            app.buttons["sign-in"].firstMatch.waitForExistence(timeout: engineWait),
            "the offer opens the account sheet, where the sign-in is")
    }
}
