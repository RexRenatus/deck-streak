// SPEC-349 A3: the factory's view carries every layer its configuration and delegates can show.
// The rule list and the absence of a message handler are not readable through this API; the
// planted suite (A5, A6) is their proof.
// SPEC-355 A4: the card view's scripts run only when the switch is on and the factory reads every
// control back from the view it built; each control removed in turn leaves the scripts off.
// SPEC-361 A5: the same over SPEC-361 R2's eleven controls. L12 is read back from the string the
// factory's own load handed the view, and L13 from the view and its UI delegate (R8).
// SPEC-355 section 7: the switch defaults off on iOS pending a measured containment layer, so the
// view the factory builds by default is the scripts-off card view.
// SPEC-361 A17: WebKit has started before any measured wait, once per process, and a card that
// never loads reads not loaded only after the whole wait (R16, #681).
// SPEC-393 A17: the factory's view plays a card's video in the card, on the iPhone as on the iPad.
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
    /// SPEC-361 R16: WebKit has started before any measured wait, once per process (ADR-372 D11).
    override func setUp() async throws {
        try await super.setUp()
        let warmedUp = await Probe.warmUp()
        XCTAssertTrue(warmedUp, "the card probe warm-up: WebKit never started, so no card could load")
    }

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

    /// The controls a removal also takes from the read-back, each with why (SPEC-361 R8). Each makes
    /// its row's expected verdict more exact: it names one more missing control.
    static let alsoTakes: [CardLayer: (controls: Set<CardLayer>, why: String)] = [
        .L6: (controls: [.L13], why: "R8 reads L13 through the UI delegate, which must be L6's WindowRefusal"),
        .L7: (controls: [.L12], why: "L12's read-back is written only by the factory's load, which this card is handed past"),
    ]

    @MainActor
    func test_the_card_view_runs_scripts_only_with_every_control() async throws {
        let probe = try await Probe.make()
        let html = Planted.document(id: "factory", head: "", body: "<script>\(ranMarker)</script><p>a card</p>")
        let bridge = Recorder()
        // What a row's view is made from: every control with the switch set, or every control but
        // one with the switch on.
        enum Setting { case everyControl(switchedOn: Bool), without(CardLayer) }
        // How a row's card is handed to its view: by the factory's load, with a base URL, or past
        // the factory's load with none.
        enum Load { case factory, baseURL, pastTheFactory }
        // A row carries what to build, not a built view. Its view is built and its load started
        // just before its mount, and torn down before the next row's view is built, so every
        // row's wait measures its own load and no other view's start (ADR-372 D11).
        typealias Row = (name: String, setting: Setting, load: Load, verdict: CardScripts.Verdict, runs: Bool)
        func build(_ row: Row) throws -> WKWebView {
            switch row.setting {
            case .everyControl(switchedOn: let switchedOn):
                // The factory builds and loads it, so its load is the factory's.
                return try CardWebViewFactory.build(html: html, ruleList: probe.ruleList, switchedOn: switchedOn)
            case .without(let control):
                // L4, L7 and L12 are removed after the build, as a careless caller would: a named
                // handler, a card handed over with a base URL, and a card handed over past the
                // factory's load, so with no policy. Every other control is left out of the build.
                let leftOut: Set<CardLayer> = [.L4, .L7, .L12].contains(control) ? [] : [control]
                let layers = Set(CardLayer.allCases).subtracting(leftOut)
                let view = CardWebViewFactory.make(
                    layers: layers, ruleList: layers.contains(.L3) ? probe.ruleList : nil, switchedOn: true)
                if control == .L4 {
                    view.configuration.userContentController.add(bridge, name: "bridge")
                }
                switch row.load {
                case .baseURL:
                    view.loadHTMLString(html, baseURL: URL(string: "https://card.invalid/"))
                case .pastTheFactory:
                    view.loadHTMLString(html, baseURL: nil)
                case .factory:
                    CardWebViewFactory.load(html, into: view)
                }
                return view
            }
        }
        var rows: [Row] = [
            (name: "every control, the switch on", setting: .everyControl(switchedOn: true), load: .factory,
             verdict: .run, runs: true),
            (name: "every control, the switch off", setting: .everyControl(switchedOn: false), load: .factory,
             verdict: .off(missing: []), runs: false),
        ]
        for control in examined("required controls removed in turn", CONTROLS) {
            let load: Load
            switch control {
            case .L7:
                load = .baseURL
            case .L12:
                load = .pastTheFactory
            default:
                load = .factory
            }
            var missing: Set<CardLayer> = [control]
            var name = "without \(control.rawValue), the switch on"
            if let also = Self.alsoTakes[control] {
                missing.formUnion(also.controls)
                name += ", which also takes \(also.controls.map(\.rawValue).sorted()): \(also.why)"
            }
            rows.append((name: name, setting: .without(control), load: load, verdict: .off(missing: missing), runs: false))
        }
        // WebKit's GPU and networking processes exit when no page holds them, so a row whose wait
        // followed the previous row's teardown paid their relaunch. One kept card, loaded before
        // the first row's view is built and taken down after the last row, holds them. It is not a
        // row: nothing is read from it, and it starts no load inside any row's wait (ADR-372 D11).
        let keeper = await Probe.keeper(ruleList: probe.ruleList)
        XCTAssertNotNil(keeper, "the keeper never loaded, so WebKit's shared processes were not held across the rows")
        defer { keeper?.removeFromSuperview() }
        for row in rows {
            let view = try build(row)
            Probe.mount(view)
            let took = await Probe.loadTime(view)
            let loaded = took != nil
            let ran = await Probe.evaluate(view, "String(document.documentElement.dataset.ran === '1')") == "true"
            let verdict = CardWebViewFactory.verdict(of: view)
            view.configuration.userContentController.removeAllScriptMessageHandlers()
            view.removeFromSuperview()
            let tookText = took.map { String(format: "%.2f", $0) } ?? "none"
            print("factory \(row.name): loaded=\(loaded) took=\(tookText) ran=\(ran) verdict=\(verdict.map { String(describing: $0) } ?? "none")")
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

    /// SPEC-361 A17 (R16, #681): a card view the factory builds, handed a document without the
    /// planted-card `meta`, never reports that it has loaded, and the wait reads it not loaded only
    /// after its whole bound, so the wait a cold first test holds cannot hide a card that never loads.
    @MainActor
    func test_the_load_wait_reads_a_card_that_never_loads_as_not_loaded() async throws {
        let view = try await CardWebViewFactory.makeCardWebView(html: "<p>a card with no planted-card meta</p>")
        Probe.mount(view)
        defer { view.removeFromSuperview() }
        let start = Date()
        let loaded = await Probe.loaded(view)
        let elapsed = Date().timeIntervalSince(start)
        let line = "card probe never loads: loaded=\(loaded) elapsed=\(String(format: "%.2f", elapsed))"
            + " bound=\(Probe.loadSeconds)"
        print(line)
        XCTAssertFalse(loaded, "a card with no planted-card meta read loaded: \(line)")
        XCTAssertGreaterThanOrEqual(
            elapsed, Probe.loadSeconds, "the wait read the card not loaded before its whole bound: \(line)")
    }

    /// SPEC-393 A17 (R10): a view the factory makes reads `allowsInlineMediaPlayback` as true, so a
    /// card's video plays inside the card on the iPhone, whose default is off, as on the iPad.
    @MainActor
    func test_the_factory_plays_media_inline() async throws {
        let view = try await CardWebViewFactory.makeCardWebView(
            html: Planted.document(id: "inline", head: "", body: "<p>a card</p>"))
        let inline = view.configuration.allowsInlineMediaPlayback
        print("factory inline playback: \(inline)")
        XCTAssertTrue(inline, "A17: the factory's view plays media inline")
    }
}
