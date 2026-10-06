// SPEC-349 A3: the factory's view carries every layer its configuration and delegates can show.
// The rule list and the absence of a message handler are not readable through this API; the
// planted suite (A5, A6) is their proof.
// SPEC-355 A4: the card view's scripts run only when the switch is on and the factory reads every
// control back from the view it built; each control removed in turn leaves the scripts off.
// SPEC-355 section 7: the switch defaults off on iOS pending a measured containment layer, so the
// view the factory builds by default is the scripts-off card view.
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

    /// The switch's default, read from the view the factory builds when nothing names the switch:
    /// built with the switch off, every control read back, and the card's own script not run. A
    /// reference view with page JavaScript on runs the same card's script, so the marker the
    /// default view leaves unset is one that can be set.
    @MainActor
    func test_the_factory_default_build_is_the_scripts_off_card_view() async throws {
        let html = Planted.document(id: "factory", head: "", body: "<script>\(ranMarker)</script><p>a card</p>")
        let reference = CardWebViewFactory.make(layers: [], ruleList: nil)
        reference.loadHTMLString(html, baseURL: nil)
        let shipped = try await CardWebViewFactory.makeCardWebView(html: html)
        var loaded: [String: Bool] = [:]
        var ran: [String: Bool] = [:]
        for (name, view) in examined("views", [("reference", reference), ("default", shipped)]) {
            Probe.mount(view)
            loaded[name] = await Probe.loaded(view)
            ran[name] = await Probe.evaluate(view, "String(document.documentElement.dataset.ran === '1')") == "true"
            view.removeFromSuperview()
        }
        let switchedOn = CardScripts.switchedOn
        let builtOn = CardWebViewFactory.built(of: shipped)?.switchedOn
        let verdict = CardWebViewFactory.verdict(of: shipped)
        let wanted: CardScripts.Verdict = .off(missing: [])
        let builtText = builtOn.map { String($0) } ?? "none"
        let verdictText = verdict.map { String(describing: $0) } ?? "none"
        let views = ["reference", "default"]
        let readings = views.map { "\($0) loaded=\(loaded[$0] ?? false) ran=\(ran[$0] ?? false)" }
        print("factory default build: switchedOn=\(switchedOn) built=\(builtText) verdict=\(verdictText)")
        print("factory default build: \(readings.joined(separator: ", "))")
        // The behaviour first: the card's script ran in the reference view and not in the view the
        // factory builds by default, and both views finished loading the card.
        XCTAssertEqual(ran, ["reference": true, "default": false], "the views in which the card's script ran")
        XCTAssertEqual(loaded, ["reference": true, "default": true], "the views that finished loading the card")
        XCTAssertFalse(switchedOn, "the switch defaults on")
        XCTAssertEqual(builtOn, false, "the switch the default view was built under")
        XCTAssertEqual(verdict, wanted, "the default view's verdict: the switch off, every control present")
    }
}
