// SPEC-349 A5 to A8: the planted suite. Every card is shown first in a reference view with every
// layer off, where it must reach its probe, then in the card view the factory ships, where it
// must reach nothing; then with one layer removed at a time, where it must reach exactly when the
// schematic gives that layer alone. Each view gets its own listeners, so an arrival names the view
// and the card that sent it.
import UIKit
import WebKit
import XCTest

@testable import CardIsolation

/// Counts what a reference or single-layer view's own delegates see, for a layer that is off:
/// every navigation it allows, every window it creates, every message the `bridge` handler gets.
@MainActor
final class Recorder: NSObject, WKNavigationDelegate, WKUIDelegate, WKScriptMessageHandler {
    private(set) var allowed = 0
    private(set) var windows = 0
    private(set) var messages = 0
    private var created: [WKWebView] = []

    func webView(
        _ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy) -> Void
    ) {
        allowed += 1
        decisionHandler(.allow)
    }

    func webView(
        _ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
        for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures
    ) -> WKWebView? {
        windows += 1
        let window = WKWebView(frame: .zero, configuration: configuration)
        created.append(window)
        return window
    }

    func userContentController(
        _ userContentController: WKUserContentController, didReceive message: WKScriptMessage
    ) {
        messages += 1
    }

    /// Stops every window this recorder created.
    func close() {
        for window in created {
            window.stopLoading()
        }
        created = []
    }
}

/// What one view showed of one card.
struct Reading: Sendable {
    var loaded = false
    var arrivals = Arrivals()
    var allowedAfterFirst = 0
    var windows = 0
    var messages = 0
    var marker = false
    var body = ""
    var fileWidth = 0
    var webkit = ""

    /// Whether this reading shows `card`'s channel open.
    func reached(_ card: Planted) -> Bool {
        switch card.observable {
        case .path: return arrivals.paths.contains { $0.hasPrefix("/\(card.id)/") }
        case .connection: return arrivals.connections > 0
        case .datagram: return arrivals.datagrams > 0
        case .marker: return marker
        case .body(let text): return body.contains(text)
        case .window: return windows > 0
        case .bridge: return messages > 0
        case .allowed: return allowedAfterFirst > 0
        case .fileImage: return fileWidth > 0
        }
    }

    /// One line for the run's log, every probe the reading holds.
    var summary: String {
        "loaded=\(loaded) paths=\(arrivals.paths) connections=\(arrivals.connections)"
            + " datagrams=\(arrivals.datagrams) allowed=\(allowedAfterFirst) windows=\(windows)"
            + " messages=\(messages) marker=\(marker) fileWidth=\(fileWidth) webkit=\(webkit)"
    }
}

/// A snapshot's decoded pixels: drawn into an RGBA8 bitmap of its own pixel size in one colour
/// space, so two snapshots compare by what they show and never by how a PNG encoder wrote them.
struct Pixels: Equatable, Sendable {
    let width: Int
    let height: Int
    let bytes: Data

    /// The pixel dimensions, as one value an assertion can print.
    var size: String { "\(width)x\(height)" }

    init?(_ image: CGImage) {
        guard let space = CGColorSpace(name: CGColorSpace.sRGB),
              let context = CGContext(
                data: nil, width: image.width, height: image.height, bitsPerComponent: 8,
                bytesPerRow: image.width * 4, space: space,
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
        else { return nil }
        context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
        guard let drawn = context.makeImage(), let data = drawn.dataProvider?.data else { return nil }
        width = image.width
        height = image.height
        bytes = data as Data
    }
}

/// One snapshot of a view: the PNG it encodes to, kept to attach and to print, and its pixels.
struct ViewSnapshot: Sendable {
    var png = Data()
    var pixels: Pixels?
}

/// The page state the suite reads with the app's own script, which runs whatever L2 says.
private struct PageState: Decodable {
    let loaded: Bool
    let ran: Bool
    let body: String
    let file: Int
    let webkit: String
}

/// Which view a card is shown in.
enum Variant: Hashable, Sendable {
    /// `make(layers: [])`: every layer off, page JavaScript on.
    case reference
    /// `makeCardWebView(html:)`: the card view the factory ships.
    case shipped
    /// `make(layers:)` with every layer but one.
    case without(CardLayer)

    var layers: Set<CardLayer> {
        switch self {
        case .reference: return []
        case .shipped: return Set(CardLayer.allCases)
        case .without(let layer): return Set(CardLayer.allCases).subtracting([layer])
        }
    }
}

/// Builds the suite's views, shows a card in one, and reads every probe.
@MainActor
final class Probe {
    let ruleList: WKContentRuleList?
    let directory: URL
    let fixture: URL

    private init(ruleList: WKContentRuleList?, directory: URL) {
        self.ruleList = ruleList
        self.directory = directory
        self.fixture = directory.appendingPathComponent("fixture.gif")
    }

    /// The probe, with the rule list compiled from the factory's own source and the fixture image
    /// written to a fresh temporary directory.
    static func make() async throws -> Probe {
        let list = try? await WKContentRuleListStore.default().compileContentRuleList(
            forIdentifier: RuleList.identifier, encodedContentRuleList: RuleList.source)
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("card-probe-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let probe = Probe(ruleList: list, directory: directory)
        try fixtureGIF.write(to: probe.fixture)
        return probe
    }

    /// The host app's window, where every view is mounted so it lays out and paints.
    static var window: UIWindow? {
        UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .flatMap { $0.windows }
            .first
    }

    /// Mounts `view` at the suite's one size.
    static func mount(_ view: WKWebView) {
        view.frame = CGRect(x: 0, y: 0, width: 320, height: 480)
        window?.addSubview(view)
    }

    /// Runs the app's own script and returns its string result.
    static func evaluate(_ view: WKWebView, _ script: String) async -> String? {
        await withCheckedContinuation { (continuation: CheckedContinuation<String?, Never>) in
            view.evaluateJavaScript(script) { result, _ in
                continuation.resume(returning: result as? String)
            }
        }
    }

    /// Polls `check` until it holds or `seconds` pass; returns the seconds it took, or nil.
    static func poll(_ seconds: TimeInterval, _ check: () async -> Bool) async -> TimeInterval? {
        let start = Date()
        while Date().timeIntervalSince(start) < seconds {
            if await check() { return Date().timeIntervalSince(start) }
            try? await Task.sleep(nanoseconds: 50_000_000)
        }
        return await check() ? Date().timeIntervalSince(start) : nil
    }

    /// Whether the card document `view` was handed has finished loading.
    static func loaded(_ view: WKWebView) async -> Bool {
        await poll(10) {
            await evaluate(
                view,
                "String(document.readyState === 'complete' && !!document.querySelector('meta[name=\"planted-card\"]'))"
            ) == "true"
        } != nil
    }

    /// Takes one snapshot of `view` and decodes its pixels.
    static func snapshot(_ view: WKWebView) async -> ViewSnapshot {
        await withCheckedContinuation { (continuation: CheckedContinuation<ViewSnapshot, Never>) in
            view.takeSnapshot(with: nil) { image, _ in
                continuation.resume(
                    returning: ViewSnapshot(
                        png: image?.pngData() ?? Data(), pixels: image?.cgImage.flatMap { Pixels($0) }))
            }
        }
    }

    /// Snapshots `view` until two snapshots in a row decode to the same pixels, or `seconds` pass.
    /// Returns the last snapshot, how many were taken, and whether two in a row matched.
    static func settle(
        _ view: WKWebView, _ seconds: TimeInterval
    ) async -> (snapshot: ViewSnapshot, polls: Int, settled: Bool) {
        var last = ViewSnapshot()
        var polls = 0
        let took = await Probe.poll(seconds) {
            let next = await Probe.snapshot(view)
            polls += 1
            let same = polls > 1 && next.pixels != nil && next.pixels == last.pixels
            last = next
            return same
        }
        return (last, polls, took != nil)
    }

    private static let stateScript = """
        JSON.stringify({
          loaded: !!document.querySelector('meta[name="planted-card"]'),
          ran: document.documentElement.dataset.ran === '1',
          body: document.body ? document.body.innerText : '',
          file: (function () { var i = document.getElementById('planted-file'); return i ? i.naturalWidth : 0; })(),
          webkit: document.documentElement.dataset.webkit || ''
        })
        """

    /// Shows `card` in `variant` and reads every probe: polling until it reaches, up to
    /// `deadline` seconds, when `window` is nil (a reference), else once after `window` seconds.
    /// Returns the reading and, when it reached while polling, how long it took.
    func show(
        _ card: Planted, in variant: Variant, window: TimeInterval?, deadline: TimeInterval = 5
    ) async throws -> (Reading, TimeInterval?) {
        let listeners = try Listeners()
        try await listeners.start()
        defer { listeners.cancel() }
        let address = Address(tcpPort: listeners.tcpPort, udpPort: listeners.udpPort, fixture: fixture)
        let html = card.html(address)
        let recorder = Recorder()
        let layers = variant.layers
        let view: WKWebView
        if variant == .shipped {
            view = try await CardWebViewFactory.makeCardWebView(html: html)
        } else {
            view = CardWebViewFactory.make(layers: layers, ruleList: layers.contains(.L3) ? ruleList : nil)
        }
        // The probe setting every view gets, the shipped one too: a planted click is the app's own
        // script activating the element, the stand-in for a tap (ADR-360 D5).
        view.configuration.preferences.javaScriptCanOpenWindowsAutomatically = true
        if variant != .shipped {
            if !layers.contains(.L5) { view.navigationDelegate = recorder }
            if !layers.contains(.L6) { view.uiDelegate = recorder }
            if !layers.contains(.L4) { view.configuration.userContentController.add(recorder, name: "bridge") }
        }
        Probe.mount(view)
        defer {
            view.stopLoading()
            view.configuration.userContentController.removeAllScriptMessageHandlers()
            view.removeFromSuperview()
            recorder.close()
        }
        if variant != .shipped {
            if card.id == "file" && !layers.contains(.L7) {
                let page = directory.appendingPathComponent("\(UUID().uuidString).html")
                try Data(html.utf8).write(to: page)
                view.loadFileURL(page, allowingReadAccessTo: directory)
            } else {
                view.loadHTMLString(html, baseURL: nil)
            }
        }
        var reading = Reading()
        reading.loaded = await Probe.loaded(view)
        if card.click {
            _ = await Probe.evaluate(
                view,
                "(function () { var e = document.querySelector('[data-planted-click]'); if (!e) { return 'none'; } e.click(); return 'clicked'; })()"
            )
        }
        var took: TimeInterval?
        if let window {
            try? await Task.sleep(nanoseconds: UInt64(window * 1_000_000_000))
            reading = await read(view, listeners, recorder, reading.loaded)
        } else {
            took = await Probe.poll(deadline) {
                reading = await self.read(view, listeners, recorder, reading.loaded)
                return reading.reached(card)
            }
        }
        return (reading, took)
    }

    private func read(
        _ view: WKWebView, _ listeners: Listeners, _ recorder: Recorder, _ loaded: Bool
    ) async -> Reading {
        var reading = Reading()
        reading.loaded = loaded
        reading.arrivals = listeners.arrivals()
        if let gate = view.navigationDelegate as? GateAdapter {
            reading.allowedAfterFirst = max(0, gate.decisions.filter { $0 == .allow }.count - 1)
        } else if view.navigationDelegate === recorder {
            reading.allowedAfterFirst = max(0, recorder.allowed - 1)
        }
        reading.windows = view.uiDelegate === recorder ? recorder.windows : 0
        reading.messages = recorder.messages
        if let text = await Probe.evaluate(view, Probe.stateScript),
           let state = try? JSONDecoder().decode(PageState.self, from: Data(text.utf8)) {
            reading.marker = state.ran
            reading.body = state.body
            reading.fileWidth = state.file
            reading.webkit = state.webkit
        }
        return reading
    }
}

final class PlantedCardTests: XCTestCase {
    /// Prints how many were judged, and refuses zero (the tdd pack's contract).
    func examined<T>(_ what: String, _ items: [T]) -> [T] {
        print("examined \(items.count) \(what)")
        XCTAssertFalse(items.isEmpty, "examined 0 \(what): the population is empty, so nothing was judged")
        return items
    }

    /// Every card's reference reading, measured once per run and shared by A5 and A6.
    @MainActor private static var references: [String: (Reading, TimeInterval?)] = [:]

    @MainActor
    private func reference(_ card: Planted, _ probe: Probe) async throws -> (Reading, TimeInterval?) {
        if let known = Self.references[card.id] {
            return known
        }
        let measured = try await probe.show(card, in: .reference, window: nil)
        print("reference \(card.id): reached=\(measured.0.reached(card)) took=\(measured.1.map { String(format: "%.2f", $0) } ?? "-") \(measured.0.summary)")
        Self.references[card.id] = measured
        return measured
    }

    /// The window a non-reference view is watched for: 1.5 s, or three times the reference's latency.
    private func window(_ took: TimeInterval?) -> TimeInterval {
        max(1.5, 3 * (took ?? 0.5))
    }

    @MainActor
    func test_a_planted_card_reaches_from_the_reference_view_and_nothing_from_the_card_view() async throws {
        let probe = try await Probe.make()
        var shippedReached: [String] = []
        var notLoaded: [String] = []
        var silent: Set<String> = []
        var connections: [String: Int] = [:]
        for card in examined("planted cards", PLANTED) {
            let (reference, took) = try await reference(card, probe)
            if !reference.reached(card) {
                silent.insert(card.id)
            }
            let (shipped, _) = try await probe.show(card, in: .shipped, window: window(took))
            print("shipped \(card.id): reached=\(shipped.reached(card)) \(shipped.summary)")
            connections[card.id] = shipped.arrivals.connections
            print("connections \(card.id): \(shipped.arrivals.connections)")
            if shipped.reached(card) {
                shippedReached.append(card.id)
            }
            if !shipped.loaded {
                notLoaded.append(card.id)
            }
        }
        // The behaviour first: nothing reaches from the card view.
        XCTAssertEqual(shippedReached, [], "cards that reached their probe from the card view")
        // Every card's connections from the card view are counted. A followed link opens one
        // connection to its host with no request read, and no layer of this delivery holds it
        // (ADR-360 D8, #677): `nav-self` and `nav-blank` may each read one, every other card none.
        let residual = ["nav-self", "nav-blank"]
        for name in residual {
            let count = connections[name]
            let reading = count.map { String($0) } ?? "none"
            print("residual \(name): connections=\(reading)")
            XCTAssertNotNil(count, "\(name) is not planted, so its connections were not counted")
            XCTAssertLessThanOrEqual(count ?? 0, 1, "the connections the card view opened for \(name)")
        }
        XCTAssertEqual(
            connections.filter { !residual.contains($0.key) && $0.value != 0 }, [:],
            "the cards whose card view opened a connection, the residual aside")
        XCTAssertEqual(notLoaded, [], "cards the card view never finished loading, so nothing was judged")
        XCTAssertEqual(
            silent, Set(UNOBSERVABLE.keys),
            "the cards whose reference view reached nothing, against the declared UNOBSERVABLE")
    }

    @MainActor
    func test_removing_one_layer_opens_exactly_its_own_channels() async throws {
        let probe = try await Probe.make()
        let cards = examined("planted cards", PLANTED)
        var opened: [CardLayer: Set<String>] = [:]
        var expected: [CardLayer: Set<String>] = [:]
        for layer in LAYERS {
            opened[layer] = []
            expected[layer] = Set(
                cards.filter { $0.alone == layer && UNOBSERVABLE[$0.id] == nil }.map { $0.id })
        }
        var variants = 0
        for card in cards {
            let (_, took) = try await reference(card, probe)
            for layer in LAYERS {
                let (reading, _) = try await probe.show(card, in: .without(layer), window: window(took))
                variants += 1
                print("without \(layer.rawValue) \(card.id): reached=\(reading.reached(card)) \(reading.summary)")
                if reading.reached(card) && UNOBSERVABLE[card.id] == nil {
                    opened[layer, default: []].insert(card.id)
                }
            }
        }
        print("examined \(variants) single-layer variants")
        // The behaviour first: each layer removed alone opens exactly its own channels.
        XCTAssertEqual(opened, expected, "the channels each layer opened when removed alone")
        let nothing = Set(LAYERS.filter { opened[$0, default: []].isEmpty })
        XCTAssertEqual(nothing, DEPTH, "the layers whose removal alone opened nothing, against DEPTH")
    }

    @MainActor
    func test_with_page_javascript_on_a_peer_connection_reaches_the_udp_listener() async throws {
        let probe = try await Probe.make()
        let card = try XCTUnwrap(PLANTED.first { $0.id == "webrtc" }, "no webrtc card is planted")
        let (reading, _) = try await probe.show(card, in: .without(.L2), window: nil)
        print("javascript on, webrtc: datagrams=\(reading.arrivals.datagrams) typeof window.webkit=\(reading.webkit)")
        XCTAssertGreaterThan(reading.arrivals.datagrams, 0, "the peer connection sent no datagram")
        XCTAssertNil(UNOBSERVABLE["webrtc"], "webrtc is declared UNOBSERVABLE")
    }

    /// How long each render view's snapshots are polled for two in a row that match.
    private static let settleDeadline: TimeInterval = 10

    @MainActor
    func test_the_card_renders_in_the_card_view_as_in_the_reference_view() async throws {
        let reference = CardWebViewFactory.make(layers: [], ruleList: nil)
        reference.loadHTMLString(RENDER, baseURL: nil)
        let shipped = try await CardWebViewFactory.makeCardWebView(html: RENDER)
        let blank = WKWebView(frame: .zero, configuration: WKWebViewConfiguration())
        blank.loadHTMLString(
            Planted.document(id: "blank", head: "", body: ""), baseURL: nil)
        var shown: [String: (text: String, width: String, snapshot: ViewSnapshot)] = [:]
        for (name, view) in examined("render views", [("reference", reference), ("shipped", shipped), ("blank", blank)]) {
            Probe.mount(view)
            let loaded = await Probe.loaded(view)
            // A bounded stability poll, never a fixed wait: the snapshot judged is the first that
            // decodes to the same pixels as the one before it, and a view that never settles fails.
            let (snapshot, polls, settled) = await Probe.settle(view, Self.settleDeadline)
            let line = "settle \(name): polls=\(polls) settled=\(settled) deadline=\(Self.settleDeadline)s"
            print(line)
            XCTAssertTrue(settled, line)
            let text = await Probe.evaluate(view, "document.body ? document.body.innerText : ''") ?? ""
            let width = await Probe.evaluate(
                view,
                "String((document.getElementById('render-image') || { naturalWidth: 0 }).naturalWidth)") ?? ""
            view.removeFromSuperview()
            print(
                "render \(name): loaded=\(loaded) text=\(text.debugDescription) width=\(width)"
                    + " png=\(snapshot.png.count) bytes pixels=\(snapshot.pixels?.size ?? "none")")
            shown[name] = (text, width, snapshot)
        }
        let card = try XCTUnwrap(shown["shipped"])
        let base = try XCTUnwrap(shown["reference"])
        let empty = try XCTUnwrap(shown["blank"])
        XCTAssertEqual(card.text, base.text, "the card view's text against the reference view's")
        XCTAssertGreaterThan(Int(card.width) ?? 0, 0, "the card view's data: image has no natural width")
        let cardPixels = try XCTUnwrap(card.snapshot.pixels, "the card view gave no snapshot")
        let basePixels = try XCTUnwrap(base.snapshot.pixels, "the reference view gave no snapshot")
        let emptyPixels = try XCTUnwrap(empty.snapshot.pixels, "the blank view gave no snapshot")
        // R8's "equals", with no tolerance: the same dimensions and the same decoded bytes.
        XCTAssertEqual(
            cardPixels.size, basePixels.size, "the card view's snapshot dimensions against the reference view's")
        XCTAssertEqual(
            cardPixels.bytes, basePixels.bytes, "the card view's decoded pixels against the reference view's")
        if cardPixels != basePixels {
            for (view, snapshot) in [("shipped", card.snapshot), ("reference", base.snapshot)] {
                let attachment = XCTAttachment(data: snapshot.png, uniformTypeIdentifier: "public.png")
                attachment.name = "A8 \(view) snapshot"
                attachment.lifetime = .keepAlways
                add(attachment)
            }
        }
        XCTAssertNotEqual(cardPixels, emptyPixels, "the card view's snapshot equals a blank view's")
    }
}
