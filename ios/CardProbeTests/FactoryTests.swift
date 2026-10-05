// SPEC-349 A3: the factory's view carries every layer its configuration and delegates can show.
// The rule list and the absence of a message handler are not readable through this API; the
// planted suite (A5, A6) is their proof.
import WebKit
import XCTest

@testable import CardIsolation

/// The layers a built view exposes, read together so a red quotes them all.
struct ObservableLayers: Equatable {
    var persistentStore: Bool
    var pageJavaScript: Bool
    var navigationDelegate: String
    var uiDelegate: String
    var url: String
}

final class FactoryTests: XCTestCase {
    @MainActor
    func test_the_factory_sets_every_observable_layer() async throws {
        let view = try await CardWebViewFactory.makeCardWebView(
            html: Planted.document(id: "factory", head: "", body: "<p>a card</p>"))
        Probe.mount(view)
        defer { view.removeFromSuperview() }
        let loaded = await Probe.loaded(view)
        let observed = ObservableLayers(
            persistentStore: view.configuration.websiteDataStore.isPersistent,
            pageJavaScript: view.configuration.defaultWebpagePreferences.allowsContentJavaScript,
            navigationDelegate: view.navigationDelegate.map { String(describing: type(of: $0)) } ?? "none",
            uiDelegate: view.uiDelegate.map { String(describing: type(of: $0)) } ?? "none",
            url: view.url?.absoluteString ?? "none")
        print("factory view: loaded=\(loaded) \(observed)")
        XCTAssertEqual(
            observed,
            ObservableLayers(
                persistentStore: false, pageJavaScript: false, navigationDelegate: "GateAdapter",
                uiDelegate: "WindowRefusal", url: "about:blank"),
            "A3: the factory's view, layer by layer")
        XCTAssertTrue(loaded, "the factory's view never finished loading the card")
    }
}
