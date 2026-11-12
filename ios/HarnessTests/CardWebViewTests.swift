// The card's web view isolation, as far as its configuration exposes it (SPEC-339 A8, R6). The
// content rule list and the absence of a script message handler are not observable through this
// API; the planted-card proof of all four layers is #619's.
import WebKit
import XCTest

// A plain import: what this test reads is public, so no build of it needs the app's testability.
import Harness

/// The two layers a configuration exposes, read together so a red quotes both.
struct Isolation: Equatable {
    var persistentDataStore: Bool
    var pageJavaScript: Bool
}

final class CardWebViewTests: XCTestCase {
    @MainActor
    func test_a8_the_card_web_view_is_isolated() {
        let configuration = CardWebView.makeConfiguration()
        XCTAssertEqual(
            Isolation(
                persistentDataStore: configuration.websiteDataStore.isPersistent,
                pageJavaScript: configuration.defaultWebpagePreferences.allowsContentJavaScript),
            Isolation(persistentDataStore: false, pageJavaScript: false),
            "A8: the card's web view keeps nothing past the view and runs no page script")
    }
}
