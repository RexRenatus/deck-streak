// SPEC-349 A3: the factory's view carries every layer its configuration and delegates can show.
// The rule list and the absence of a message handler are not readable through this API; the
// planted suite (A5, A6) is their proof.
// SPEC-355 A4: the card view's scripts run only when the switch is on and the factory reads every
// control back from the view it built; each control removed in turn leaves the scripts off.
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
    /// Prints how many were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

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

    @MainActor
    func test_the_card_view_runs_scripts_only_with_every_control() async throws {
        let probe = try await Probe.make()
        let hold = try ConnectionHold()
        try await hold.start()
        defer { hold.cancel() }
        let html = Planted.document(id: "factory", head: "", body: "<script>\(ranMarker)</script><p>a card</p>")
        let bridge = Recorder()
        let on = try CardWebViewFactory.build(html: html, ruleList: probe.ruleList, hold: hold, switchedOn: true)
        let off = try CardWebViewFactory.build(html: html, ruleList: probe.ruleList, hold: hold, switchedOn: false)
        var rows: [(name: String, view: WKWebView, verdict: CardScripts.Verdict, runs: Bool)] = [
            (name: "every control, the switch on", view: on, verdict: .run, runs: true),
            (name: "every control, the switch off", view: off, verdict: .off(missing: []), runs: false),
        ]
        for control in examined("required controls removed in turn", CONTROLS) {
            // L4 and L7 are removed after the build, as a careless caller would: a named handler,
            // and a card handed over with a base URL. Every other control is left out of the build.
            let leftOut: Set<CardLayer> = control == .L4 || control == .L7 ? [] : [control]
            let layers = Set(CardLayer.allCases).subtracting(leftOut)
            let view = CardWebViewFactory.make(
                layers: layers, ruleList: layers.contains(.L3) ? probe.ruleList : nil,
                hold: layers.contains(.L9) ? hold : nil, switchedOn: true)
            if control == .L4 {
                view.configuration.userContentController.add(bridge, name: "bridge")
            }
            view.loadHTMLString(html, baseURL: control == .L7 ? URL(string: "https://card.invalid/") : nil)
            rows.append((name: "without \(control.rawValue), the switch on", view: view, verdict: .off(missing: [control]), runs: false))
        }
        for row in rows {
            Probe.mount(row.view)
            let loaded = await Probe.loaded(row.view)
            let ran = await Probe.evaluate(row.view, "String(document.documentElement.dataset.ran === '1')") == "true"
            let verdict = CardWebViewFactory.verdict(of: row.view)
            row.view.configuration.userContentController.removeAllScriptMessageHandlers()
            row.view.removeFromSuperview()
            print("factory \(row.name): loaded=\(loaded) ran=\(ran) verdict=\(verdict.map { String(describing: $0) } ?? "none")")
            // The behaviour first: the card's script runs exactly when the switch is on and every
            // control is present.
            XCTAssertEqual(ran, row.runs, "\(row.name): whether the card's script ran")
            XCTAssertEqual(verdict, row.verdict, "\(row.name): the switch's verdict")
            XCTAssertTrue(loaded, "\(row.name): the view never finished loading the card")
        }
    }
}
