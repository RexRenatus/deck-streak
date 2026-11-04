// SPEC-349 A5 to A8: the planted suite. Every card is shown first in a reference view with every
// layer off, where it must reach its probe, then in the card view the factory ships, where it
// must reach nothing; then with one layer removed at a time, where it must reach exactly when the
// schematic gives that layer alone. Each view gets its own listeners, so an arrival names the view
// and the card that sent it. SPEC-355 A6 to A12 add the scripted views: the scripted card view the
// factory builds with the switch on, each scripted card's reference (scripts on, its own controls
// off), and the scripted view with one control removed at a time.
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
    /// Whether this recorder answers yes to a dialog and grants a capture request: a scripted
    /// reference's does, so the reference is never blind to them (SPEC-355 R9); every other
    /// answers no, as a refusal would.
    var permissive = false
    /// Every dialog or capture request this recorder answered yes.
    private(set) var granted = 0
    /// Every dialog or capture request this recorder was asked.
    private(set) var asked = 0

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

    func webView(
        _ webView: WKWebView, runJavaScriptAlertPanelWithMessage message: String,
        initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor @Sendable () -> Void
    ) {
        asked += 1
        completionHandler()
    }

    func webView(
        _ webView: WKWebView, runJavaScriptConfirmPanelWithMessage message: String,
        initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor @Sendable (Bool) -> Void
    ) {
        asked += 1
        if permissive { granted += 1 }
        completionHandler(permissive)
    }

    func webView(
        _ webView: WKWebView, runJavaScriptTextInputPanelWithPrompt prompt: String,
        defaultText: String?, initiatedByFrame frame: WKFrameInfo,
        completionHandler: @escaping @MainActor @Sendable (String?) -> Void
    ) {
        asked += 1
        if permissive { granted += 1 }
        completionHandler(permissive ? "answered" : nil)
    }

    func webView(
        _ webView: WKWebView, requestMediaCapturePermissionFor origin: WKSecurityOrigin,
        initiatedByFrame frame: WKFrameInfo, type: WKMediaCaptureType,
        decisionHandler: @escaping @MainActor @Sendable (WKPermissionDecision) -> Void
    ) {
        asked += 1
        if permissive { granted += 1 }
        decisionHandler(permissive ? .grant : .deny)
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
    /// Multicast DNS queries the witness counted for this view's label.
    var queries = 0
    /// Dialogs and capture requests the recorder answered yes.
    var granted = 0
    /// The card's own record of what its script saw, when it writes one.
    var record = ""
    /// The `link` elements the view holds, in its document, every template's content and every
    /// open shadow root, when `show` was asked to count them (SPEC-392 R5); nil when the count was
    /// not asked for or its answer was not a number.
    var links: Int?

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
        case .query: return queries > 0
        case .granted:
            return granted > 0 || record.contains("\"confirm\":\"true\"")
                || record.contains("\"capture\":\"granted\"")
        }
    }

    /// One line for the run's log, every probe the reading holds.
    var summary: String {
        "loaded=\(loaded) paths=\(arrivals.paths) connections=\(arrivals.connections)"
            + " datagrams=\(arrivals.datagrams) allowed=\(allowedAfterFirst) windows=\(windows)"
            + " messages=\(messages) marker=\(marker) fileWidth=\(fileWidth) webkit=\(webkit)"
            + " queries=\(queries) granted=\(granted) record=\(record)"
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
    let record: String
}

/// Which view a card is shown in.
enum Variant: Hashable, Sendable {
    /// `make(layers: [])`: every layer off, page JavaScript on.
    case reference
    /// `build(html:ruleList:switchedOn: false)`: the scripts-off card view, every layer on.
    case shipped
    /// `make(layers:)` with every layer of SPEC-349's base but one (SPEC-355 R12).
    case without(CardLayer)
    /// `build(html:ruleList:switchedOn: true)`: the scripted card view (SPEC-355 R2).
    case scripted
    /// A scripted card's reference: scripts on, every control on but the ones that hold its
    /// channel.
    case scriptedReference(Set<CardLayer>)
    /// The scripted view's controls, scripts on, with one control removed (SPEC-355 R8).
    case scriptedWithout(CardLayer)
    /// The scripts-off card view with one layer removed (SPEC-361 R10).
    case shippedWithout(CardLayer)
    /// A view the factory does not build whose layers are exactly the set (SPEC-392 R5), so
    /// `referenceWith([.L14])` is the reference with only the link strip on.
    case referenceWith(Set<CardLayer>)

    var layers: Set<CardLayer> {
        switch self {
        case .reference: return []
        case .shipped, .scripted: return Set(CardLayer.allCases)
        case .without(let layer): return Set(BASE).subtracting([layer])
        case .scriptedReference(let held): return Set(CardLayer.allCases).subtracting([.L2]).subtracting(held)
        case .scriptedWithout(let control): return Set(CardLayer.allCases).subtracting([.L2, control])
        case .shippedWithout(let layer): return Set(CardLayer.allCases).subtracting([layer])
        case .referenceWith(let layers): return layers
        }
    }

    /// Whether the factory builds the view and loads the card itself, as it ships.
    var built: Bool {
        self == .shipped || self == .scripted
    }

    /// Whether the view's recorder answers yes: only a scripted reference's.
    var permissive: Bool {
        if case .scriptedReference = self {
            return true
        }
        return false
    }
}

/// A page ask that had no answer within its bound (SPEC-609 R2). It ends the running test, and its
/// description names the ask, the probe it stopped on and the bound in whole seconds.
struct ProbeStall: Error, CustomStringConvertible, LocalizedError {
    /// The label of the ask, such as `state`, `ready` or `snapshot`.
    let label: String
    /// The probe record when the ask was made (SPEC-609 R3).
    let probe: String
    /// The bound the ask had, in seconds.
    let seconds: TimeInterval

    var description: String {
        "card probe stall: the \(label) ask in \(probe) had no answer within \(Int(seconds.rounded())) s"
    }

    var errorDescription: String? {
        description
    }
}

/// The one-shot guard of one bounded ask, on the main actor (SPEC-609 R2): it resumes the ask's
/// continuation once, with whichever of the answer and the bound comes first, cancels the bound's
/// timer when it resumes, and drops whatever comes after.
@MainActor
final class AskGuard<Value: Sendable> {
    private var continuation: CheckedContinuation<Value, any Error>?
    /// The bound's timer, cancelled once the guard has resumed.
    var timer: Task<Void, Never>?

    init(_ continuation: CheckedContinuation<Value, any Error>) {
        self.continuation = continuation
    }

    /// Resumes the ask with `outcome` unless it has resumed already; returns whether it did.
    func take(_ outcome: Result<Value, any Error>) -> Bool {
        guard let continuation else {
            return false
        }
        self.continuation = nil
        timer?.cancel()
        timer = nil
        continuation.resume(with: outcome)
        return true
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

    /// The most one page ask may wait for its answer: a page's `evaluateJavaScript` answer or a
    /// view's `takeSnapshot` completion that has not come within it is a stall, which fails the
    /// running test by name (SPEC-609 R1, R2). It is twice `warmUpSeconds`, so WebKit's first
    /// process start, which one ask may wait for, is no stall (SPEC-361 R16, ADR-372 D11).
    static let askSeconds: TimeInterval = 120

    /// The probe the running test is on, for a stall to name: set by `show`, and reset to
    /// `no probe` by each test's `setUp` (SPEC-609 R3).
    static var probing = "no probe"

    /// How many probes the running test has shown (SPEC-609 R3).
    static var probes = 0

    /// How many asks the running test's pages answered (SPEC-609 R5).
    static var answeredAsks = 0

    /// The slowest answered ask of the running test, in seconds (SPEC-609 R5).
    static var slowestAsk: TimeInterval = 0

    /// Resets the probe record and the measurement, from each test's `setUp` (SPEC-609 R3, R5).
    static func resetRecord() {
        probing = "no probe"
        probes = 0
        answeredAsks = 0
        slowestAsk = 0
    }

    /// The line each test's `tearDown` prints: the asks its pages answered and the slowest, against
    /// the ask bound (SPEC-609 R5).
    static func asksLine(_ test: String) -> String {
        "card probe asks: \(test) asks=\(answeredAsks) slowest=\(String(format: "%.2f", slowestAsk))"
            + " bound=\(Int(askSeconds.rounded()))"
    }

    /// The one bounded page ask (SPEC-609 R1, R2): `start` asks the page and hands its answer to the
    /// handler it is given. Returns the answer, or throws `ProbeStall`, labelled `label` and naming
    /// `probing`, when none has come within `seconds`. Each answered ask is counted, and the
    /// slowest kept (R5).
    static func bounded<Value: Sendable>(
        label: String, seconds: TimeInterval = Probe.askSeconds,
        _ start: (@escaping @MainActor @Sendable (Value) -> Void) -> Void
    ) async throws -> Value {
        let asked = Date()
        let value: Value = try await withCheckedThrowingContinuation {
            (continuation: CheckedContinuation<Value, any Error>) in
            let once = AskGuard(continuation)
            start { answer in
                _ = once.take(.success(answer))
            }
        }
        answeredAsks += 1
        slowestAsk = max(slowestAsk, Date().timeIntervalSince(asked))
        return value
    }

    /// The page's string answer to `script`, asked once through `ask` under `seconds` and labelled
    /// `label` in a stall (SPEC-609 R1, R2).
    static func ask(
        asking ask: @escaping Asking, _ script: String, label: String,
        seconds: TimeInterval = Probe.askSeconds
    ) async throws -> String? {
        let answer: String? = try await bounded(label: label, seconds: seconds) { handler in
            ask(script, handler)
        }
        return answer
    }

    /// The label an ask of `script` carries in a stall: the name of one of the suite's own scripts,
    /// or else the script's opening characters (SPEC-609 R2).
    static func label(of script: String) -> String {
        switch script {
        case stateScript:
            return "state"
        case linkScript:
            return "links"
        case readyScript:
            return "ready"
        case answerScript:
            return "answer"
        default:
            return "script " + String(script.prefix(48)).replacingOccurrences(of: "\n", with: " ")
        }
    }

    /// Runs the app's own script and returns its string result, asked under the ask bound
    /// (SPEC-609 R1).
    static func evaluate(_ view: WKWebView, _ script: String) async throws -> String? {
        try await ask(
            asking: { asked, answer in
                view.evaluateJavaScript(asked) { result, _ in answer(result as? String) }
            }, script, label: label(of: script))
    }

    /// The app's own count of the `link` elements a view holds: in the document, in every
    /// template's content and in every open shadow root, each walked in turn (SPEC-392 R5).
    static let linkScript = """
        (function () {
          function count(root) {
            var found = root.querySelectorAll('link').length;
            var all = root.querySelectorAll('*');
            for (var i = 0; i < all.length; i++) {
              if (all[i].localName === 'template' && all[i].content) { found += count(all[i].content); }
              if (all[i].shadowRoot) { found += count(all[i].shadowRoot); }
            }
            return found;
          }
          return String(count(document));
        })()
        """

    /// How many `link` elements `view` holds, read by `linkScript`; nil when the answer is not a
    /// number.
    static func links(_ view: WKWebView) async throws -> Int? {
        guard let text = try await evaluate(view, linkScript) else {
            return nil
        }
        return Int(text)
    }

    /// Polls `check` until it holds or `seconds` pass; returns the seconds it took, or nil. A check
    /// that throws, such as one whose ask stalled, ends the poll with its error (SPEC-609 R2).
    static func poll(
        _ seconds: TimeInterval, _ check: () async throws -> Bool
    ) async rethrows -> TimeInterval? {
        let start = Date()
        while Date().timeIntervalSince(start) < seconds {
            if try await check() { return Date().timeIntervalSince(start) }
            try? await Task.sleep(nanoseconds: 50_000_000)
        }
        return try await check() ? Date().timeIntervalSince(start) : nil
    }

    /// The bound of every measured load wait: how long a card is given to report that it has
    /// loaded, counted from its page's first answer (SPEC-361 R16, SPEC-349 R11).
    static let loadSeconds: TimeInterval = 10

    /// The bound of each of the warm-up's two loads: they hold the host's first start of WebKit on
    /// each path, which a cold first test's measured wait would otherwise pay (SPEC-361 R16,
    /// ADR-372 D11).
    static let warmUpSeconds: TimeInterval = 60

    /// The warm-up's reading, once per process: nil until it has run.
    private static var warmedUp: Bool?

    /// Whether the card document `view` was handed reports that it has finished loading, read once.
    private static func reportsLoaded(_ view: WKWebView) async throws -> Bool {
        try await evaluate(
            view,
            "String(document.readyState === 'complete' && !!document.querySelector('meta[name=\"planted-card\"]'))"
        ) == "true"
    }

    /// The seconds the card document `view` was handed took to finish loading, or nil when it had
    /// not finished within `loadSeconds` (SPEC-361 R16), asked through the view's own script.
    static func loadTime(_ view: WKWebView) async throws -> TimeInterval? {
        try await loadTime(asking: { script, answer in
            view.evaluateJavaScript(script) { result, _ in answer(result as? String) }
        })
    }

    /// How the load wait asks a page a script: the page evaluates it and hands its string result,
    /// or nil, to the answer it is given (SPEC-349 R11).
    typealias Asking = @MainActor (String, @escaping @MainActor @Sendable (String?) -> Void) -> Void

    /// The script the load wait reads a card's load by, the check `reportsLoaded` makes: the
    /// document has finished loading and holds the planted-card `meta`.
    static let readyScript =
        "String(document.readyState === 'complete' && !!document.querySelector('meta[name=\"planted-card\"]'))"

    /// The app's own script a page answers once its process runs, loaded or not (SPEC-349 R11).
    static let answerScript = "'answered'"

    /// The load wait over any page `ask` reaches: it first asks the page `answerScript`, again
    /// whenever an ask comes back without its answer, one ask at a time, under `answerSeconds`;
    /// once the page has answered, it returns the seconds the card then took to finish loading, or
    /// nil when the page never answered or the card had not finished within `loadSeconds`
    /// (SPEC-361 R16, SPEC-349 R11). It prints the seconds the page took to answer. Each ready ask
    /// waits under `askSeconds`, and a stalled one throws `ProbeStall` (SPEC-609 R1, R2).
    static func loadTime(
        asking ask: @escaping Asking, answerSeconds: TimeInterval = Probe.warmUpSeconds,
        loadSeconds: TimeInterval = Probe.loadSeconds, askSeconds: TimeInterval = Probe.askSeconds
    ) async throws -> TimeInterval? {
        let state = AnswerState()
        let answered = try await poll(answerSeconds) {
            await askOnce(ask, state)
        }
        print("card probe answer: took=\(answered.map { String(format: "%.2f", $0) } ?? "none")")
        guard answered != nil else {
            return nil
        }
        return try await poll(loadSeconds) {
            try await reply(ask, readyScript, seconds: askSeconds) == "true"
        }
    }

    /// Asks the page `answerScript` unless an ask is out or it has answered; returns whether it has.
    private static func askOnce(_ ask: @escaping Asking, _ state: AnswerState) async -> Bool {
        if !state.asking && !state.answered {
            state.asking = true
            ask(answerScript) { result in
                state.asking = false
                if result == "answered" {
                    state.answered = true
                }
            }
        }
        return state.answered
    }

    /// The page's string answer to `script`, asked once through `ask` under `seconds`, the ask bound
    /// (SPEC-609 R1).
    private static func reply(
        _ ask: @escaping Asking, _ script: String, seconds: TimeInterval = Probe.askSeconds
    ) async throws -> String? {
        try await Probe.ask(asking: ask, script, label: label(of: script), seconds: seconds)
    }

    /// Whether the card document `view` was handed has finished loading.
    static func loaded(_ view: WKWebView) async throws -> Bool {
        try await loadTime(view) != nil
    }

    /// Starts WebKit once per test process, before any measured wait, on both paths a measured
    /// wait can take: one planted card in a scripts-off view the factory builds, then one card
    /// whose script runs in a scripts-on view, each mounted and waited for under `warmUpSeconds`.
    /// Returns whether both loaded; every later call returns the first reading (SPEC-361 R16,
    /// ADR-372 D11).
    static func warmUp() async throws -> Bool {
        if let reading = warmedUp {
            return reading
        }
        let ruleList = await RuleList.compile()
        let off = try await warmUpLoad(switchedOn: false, body: "<p>a card</p>", ruleList: ruleList)
        let on = try await warmUpLoad(
            switchedOn: true, body: "<script>\(ranMarker)</script><p>a card</p>", ruleList: ruleList)
        let loaded = off && on
        warmedUp = loaded
        return loaded
    }

    /// One warm-up load: a planted card in a view the factory builds with `switchedOn`, mounted,
    /// waited for under `warmUpSeconds` and taken down. Returns whether it loaded.
    private static func warmUpLoad(
        switchedOn: Bool, body: String, ruleList: WKContentRuleList?
    ) async throws -> Bool {
        let view = CardWebViewFactory.make(
            layers: Set(CardLayer.allCases), ruleList: ruleList, switchedOn: switchedOn)
        CardWebViewFactory.load(Planted.document(id: "warm-up", head: "", body: body), into: view)
        mount(view)
        let took = try await poll(Probe.warmUpSeconds) {
            try await reportsLoaded(view)
        }
        view.stopLoading()
        view.removeFromSuperview()
        let loaded = took != nil
        let scripts = switchedOn ? "on" : "off"
        print("card probe warm-up: scripts=\(scripts) loaded=\(loaded) took=\(took.map { String(format: "%.2f", $0) } ?? "none")")
        return loaded
    }

    /// A planted card in a scripts-off view the factory builds, mounted, waited for under
    /// `warmUpSeconds` and left mounted for the caller to take down. While it lives, WebKit's GPU
    /// and networking processes have a page to serve, so a measured wait that follows another
    /// view's teardown does not pay their relaunch (ADR-372 D11). Returns nil when it never loaded.
    static func keeper(ruleList: WKContentRuleList?) async throws -> WKWebView? {
        let view = CardWebViewFactory.make(
            layers: Set(CardLayer.allCases), ruleList: ruleList, switchedOn: false)
        CardWebViewFactory.load(Planted.document(id: "keeper", head: "", body: "<p>a card</p>"), into: view)
        mount(view)
        let took = try await poll(Probe.warmUpSeconds) {
            try await reportsLoaded(view)
        }
        print("card probe keeper: loaded=\(took != nil) took=\(took.map { String(format: "%.2f", $0) } ?? "none")")
        guard took != nil else {
            view.removeFromSuperview()
            return nil
        }
        return view
    }

    /// Takes one snapshot of `view` and decodes its pixels, under the ask bound (SPEC-609 R1).
    static func snapshot(_ view: WKWebView) async throws -> ViewSnapshot {
        let taken: ViewSnapshot = try await bounded(label: "snapshot") { answer in
            view.takeSnapshot(with: nil) { image, _ in
                answer(
                    ViewSnapshot(
                        png: image?.pngData() ?? Data(), pixels: image?.cgImage.flatMap { Pixels($0) }))
            }
        }
        return taken
    }

    /// Snapshots `view` until two snapshots in a row decode to the same pixels, or `seconds` pass.
    /// Returns the last snapshot, how many were taken, and whether two in a row matched.
    static func settle(
        _ view: WKWebView, _ seconds: TimeInterval
    ) async throws -> (snapshot: ViewSnapshot, polls: Int, settled: Bool) {
        var last = ViewSnapshot()
        var polls = 0
        let took = try await Probe.poll(seconds) {
            let next = try await Probe.snapshot(view)
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
          webkit: document.documentElement.dataset.webkit || '',
          record: document.documentElement.dataset.record || ''
        })
        """

    /// Shows `card` in `variant` and reads every probe: polling until it reaches, up to
    /// `deadline` seconds, when `window` is nil (a reference), else once after `window` seconds.
    /// Returns the reading and, when it reached while polling, how long it took. With `countLinks`,
    /// the reading carries the `link` elements the view held once loaded (SPEC-392 R5).
    func show(
        _ card: Planted, in variant: Variant, window: TimeInterval?, deadline: TimeInterval = 5,
        witness: Witness? = nil, countLinks: Bool = false
    ) async throws -> (Reading, TimeInterval?) {
        // The probe record a stall names (SPEC-609 R3).
        Probe.probes += 1
        Probe.probing = "\(card.id) in \(String(describing: variant)) (probe \(Probe.probes))"
        let listeners = try Listeners()
        try await listeners.start()
        defer { listeners.cancel() }
        let label = "v" + UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased()
        let address = Address(
            tcpPort: listeners.tcpPort, udpPort: listeners.udpPort, fixture: fixture, label: label)
        let html = card.html(address)
        let recorder = Recorder()
        recorder.permissive = variant.permissive
        let layers = variant.layers
        let view: WKWebView
        switch variant {
        case .shipped:
            view = try CardWebViewFactory.build(html: html, ruleList: ruleList, switchedOn: false)
        case .scripted:
            view = try CardWebViewFactory.build(html: html, ruleList: ruleList, switchedOn: true)
        default:
            view = CardWebViewFactory.make(layers: layers, ruleList: layers.contains(.L3) ? ruleList : nil)
        }
        // The probe setting every view gets, the shipped one too: a planted click is the app's own
        // script activating the element, the stand-in for a tap (ADR-360 D5).
        view.configuration.preferences.javaScriptCanOpenWindowsAutomatically = true
        if !variant.built {
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
        // L14, the link strip, composed by name for a view the factory does not build: the
        // factory's own build strips the card it loads (SPEC-392 R1, R5).
        let handed = layers.contains(.L14) ? LinkStrip.stripped(html) : html
        if !variant.built {
            if card.id == "file" && !layers.contains(.L7) {
                let page = directory.appendingPathComponent("\(UUID().uuidString).html")
                try Data(handed.utf8).write(to: page)
                view.loadFileURL(page, allowingReadAccessTo: directory)
            } else if layers.contains(.L12) {
                // A view that carries L12 is handed its card the one way the factory hands one.
                CardWebViewFactory.load(handed, into: view)
            } else {
                view.loadHTMLString(handed, baseURL: nil)
            }
        }
        var reading = Reading()
        reading.loaded = try await Probe.loaded(view)
        // Counted once the view has loaded; `read` rebuilds the reading, so the count is carried.
        var links: Int?
        if countLinks {
            links = try await Probe.links(view)
        }
        if card.click {
            _ = try await Probe.evaluate(
                view,
                "(function () { var e = document.querySelector('[data-planted-click]'); if (!e) { return 'none'; } e.click(); return 'clicked'; })()"
            )
        }
        var took: TimeInterval?
        if let window {
            try? await Task.sleep(nanoseconds: UInt64(window * 1_000_000_000))
            reading = try await read(view, listeners, recorder, reading.loaded, witness, label)
        } else {
            took = try await Probe.poll(deadline) {
                reading = try await self.read(view, listeners, recorder, reading.loaded, witness, label)
                return reading.reached(card)
            }
        }
        reading.links = links
        return (reading, took)
    }

    private func read(
        _ view: WKWebView, _ listeners: Listeners, _ recorder: Recorder, _ loaded: Bool,
        _ witness: Witness?, _ label: String
    ) async throws -> Reading {
        var reading = Reading()
        reading.loaded = loaded
        reading.arrivals = listeners.arrivals()
        reading.queries = witness?.count(label) ?? 0
        reading.granted = recorder.granted
        if let gate = view.navigationDelegate as? GateAdapter {
            reading.allowedAfterFirst = max(0, gate.decisions.filter { $0 == .allow }.count - 1)
        } else if view.navigationDelegate === recorder {
            reading.allowedAfterFirst = max(0, recorder.allowed - 1)
        }
        reading.windows = view.uiDelegate === recorder ? recorder.windows : 0
        reading.messages = recorder.messages
        if let text = try await Probe.evaluate(view, Probe.stateScript),
           let state = try? JSONDecoder().decode(PageState.self, from: Data(text.utf8)) {
            reading.marker = state.ran
            reading.body = state.body
            reading.fileWidth = state.file
            reading.webkit = state.webkit
            reading.record = state.record
        }
        return reading
    }
}

/// A page the load wait asks, standing in for one whose process starts late: it answers nothing
/// until `startSeconds` have passed since it was first asked, then answers `Probe.answerScript`,
/// and any other script with whether it has finished loading, which it has `loadSeconds` after
/// its start (SPEC-349 A10, A11).
@MainActor
final class LatePage {
    private let startSeconds: TimeInterval
    private let loadSeconds: TimeInterval
    private var firstAsked: Date?

    init(startSeconds: TimeInterval, loadSeconds: TimeInterval) {
        self.startSeconds = startSeconds
        self.loadSeconds = loadSeconds
    }

    /// Answers `script` through `answer` as such a page would, never before its start.
    func ask(_ script: String, _ answer: @escaping @MainActor @Sendable (String?) -> Void) {
        let first = firstAsked ?? Date()
        firstAsked = first
        let starts = first.addingTimeInterval(startSeconds)
        let loads = starts.addingTimeInterval(loadSeconds)
        Task { @MainActor in
            let wait = starts.timeIntervalSinceNow
            if wait > 0 {
                try? await Task.sleep(nanoseconds: UInt64(wait * 1_000_000_000))
            }
            answer(script == Probe.answerScript ? "answered" : String(Date() >= loads))
        }
    }
}

/// Whether a page has answered the load wait's `Probe.answerScript`, and whether an ask is out
/// (SPEC-349 R11).
@MainActor
final class AnswerState {
    var asking = false
    var answered = false
}

/// A page the bounded ask asks, standing in for one whose process stopped answering: it keeps
/// every answer it is handed and never calls one (SPEC-609 R6, A1).
@MainActor
final class SilentPage {
    private var kept: [@MainActor @Sendable (String?) -> Void] = []

    /// Keeps `answer`, never calling it, as such a page would.
    func ask(_ script: String, _ answer: @escaping @MainActor @Sendable (String?) -> Void) {
        kept.append(answer)
    }
}

/// A page the load wait asks, standing in for one whose process answers and then falls silent: it
/// answers `Probe.answerScript` with `answered` at once, and keeps, never calls, the answer to any
/// other script (SPEC-609 R6, A2).
@MainActor
final class FallsSilentPage {
    private var kept: [@MainActor @Sendable (String?) -> Void] = []

    /// Answers `script` through `answer` as such a page would.
    func ask(_ script: String, _ answer: @escaping @MainActor @Sendable (String?) -> Void) {
        if script == Probe.answerScript {
            answer("answered")
        } else {
            kept.append(answer)
        }
    }
}

/// What a criterion's call under test came back with, once it has: its value or its thrown error,
/// and when (SPEC-609 R6).
@MainActor
final class Outcome<Value> {
    var result: Result<Value, any Error>?
    var finished: Date?
}

final class PlantedCardTests: XCTestCase {
    /// SPEC-361 R16: WebKit has started before any measured wait, once per process (ADR-372 D11).
    /// SPEC-609 R3, R5: each test starts with no probe and no ask counted.
    override func setUp() async throws {
        try await super.setUp()
        await Probe.resetRecord()
        let warmedUp = try await Probe.warmUp()
        XCTAssertTrue(warmedUp, "the card probe warm-up: WebKit never started, so no card could load")
    }

    /// SPEC-609 R5: the asks this test's pages answered, and the slowest, against the ask bound.
    override func tearDown() async throws {
        let line = await Probe.asksLine(self.name)
        print(line)
        try await super.tearDown()
    }

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
        // The behaviour first: nothing reaches from the card view, and no card opens a connection
        // from it.
        XCTAssertEqual(shippedReached, [], "cards that reached their probe from the card view")
        XCTAssertEqual(
            connections.filter { $0.value != 0 }, [:], "the cards whose card view opened a connection")
        // The planted control (SPEC-361 A7): from the scripts-off view with L10 removed, each
        // followed link opens a connection, so the card view's zero is not a blind suite's.
        for name in examined("followed links", ["nav-self", "nav-blank"]) {
            let card = try XCTUnwrap(PLANTED.first { $0.id == name }, "\(name) is not planted")
            let (_, took) = try await reference(card, probe)
            let (open, _) = try await probe.show(card, in: .shippedWithout(.L10), window: window(took))
            print("followed \(name): shipped without L10 connections=\(open.arrivals.connections)")
            XCTAssertGreaterThanOrEqual(
                open.arrivals.connections, 1,
                "\(name) opened no connection from the scripts-off view without L10, so its zero proves nothing")
        }
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

    /// SPEC-392 A5: every planted card is shown from the reference view and the card view, and
    /// the `link` elements each view holds once loaded are counted.
    @MainActor
    func test_a_linked_card_reaches_the_card_view_with_no_link_element() async throws {
        let probe = try await Probe.make()
        var referenceLinks: [String: Int?] = [:]
        var shippedLinks: [String: Int?] = [:]
        for card in PLANTED {
            let (reference, _) = try await probe.show(card, in: .reference, window: nil, countLinks: true)
            let (shipped, _) = try await probe.show(card, in: .shipped, window: nil, countLinks: true)
            referenceLinks.updateValue(reference.links, forKey: card.id)
            shippedLinks.updateValue(shipped.links, forKey: card.id)
            print("links \(card.id): reference=\(reference.links.map { "\($0)" } ?? "nil") card view=\(shipped.links.map { "\($0)" } ?? "nil")")
        }
        // The behaviour first: no card view holds a `link` element, and a count that is not a
        // number differs from 0.
        XCTAssertEqual(
            shippedLinks.filter { $0.value != 0 }, [:], "the cards whose card view holds a link element")
        // The control: the cards whose reference view holds one are exactly LINKED, so the card
        // view's zero is not a blind count's.
        XCTAssertEqual(
            Set(referenceLinks.filter { ($0.value ?? 0) > 0 }.keys), LINKED,
            "the cards whose reference view holds a link element, against LINKED")
        _ = examined("planted cards", PLANTED)
    }

    /// SPEC-392 A6: each observable linked card is shown from the reference view and from the
    /// reference with only the link strip on.
    @MainActor
    func test_the_link_strip_alone_holds_every_observable_linked_card() async throws {
        let probe = try await Probe.make()
        let names = LINKED.subtracting(UNOBSERVABLE.keys).sorted()
        var passed: [String] = []
        var silent: [String] = []
        for name in names {
            let card = try XCTUnwrap(PLANTED.first { $0.id == name }, "\(name) is not planted")
            let (reference, took) = try await reference(card, probe)
            if !reference.reached(card) {
                silent.append(name)
            }
            let (stripped, _) = try await probe.show(card, in: .referenceWith([.L14]), window: window(took))
            print("only L14 \(name): reached=\(stripped.reached(card)) connections=\(stripped.arrivals.connections) \(stripped.summary)")
            if stripped.reached(card) || stripped.arrivals.connections > 0 {
                passed.append(name)
            }
        }
        // The behaviour first: with only L14 on, no observable linked card reaches its probe or
        // opens a connection.
        XCTAssertEqual(passed, [], "the linked cards that reached or opened a connection with only L14 on")
        // The control: each one reaches from the reference view, so the strip's zero is not a
        // blind view's.
        XCTAssertEqual(silent, [], "the linked cards whose reference view reached nothing")
        _ = examined("observable linked cards", names)
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
        // Each view is built as the factory builds the card view (`make`, then the card as a
        // string with no base URL) and mounted in the same main-actor turn, as the harness mounts
        // the card view, so the reference differs from the card view only in its layers.
        let builds: [(name: String, build: @MainActor () async throws -> WKWebView)] = [
            ("reference", {
                let view = CardWebViewFactory.make(layers: [], ruleList: nil)
                view.loadHTMLString(RENDER, baseURL: nil)
                return view
            }),
            ("shipped", { try await CardWebViewFactory.makeCardWebView(html: RENDER) }),
            ("blank", {
                let view = WKWebView(frame: .zero, configuration: WKWebViewConfiguration())
                view.loadHTMLString(Planted.document(id: "blank", head: "", body: ""), baseURL: nil)
                return view
            }),
        ]
        var shown: [String: (text: String, width: String, snapshot: ViewSnapshot)] = [:]
        for (name, build) in examined("render views", builds) {
            let view = try await build()
            Probe.mount(view)
            let loaded = try await Probe.loaded(view)
            // A bounded stability poll, never a fixed wait: the snapshot judged is the first that
            // decodes to the same pixels as the one before it, and a view that never settles fails.
            let (snapshot, polls, settled) = try await Probe.settle(view, Self.settleDeadline)
            let line = "settle \(name): polls=\(polls) settled=\(settled) deadline=\(Self.settleDeadline)s"
            print(line)
            XCTAssertTrue(settled, line)
            let text = try await Probe.evaluate(view, "document.body ? document.body.innerText : ''") ?? ""
            let width = try await Probe.evaluate(
                view,
                "String((document.getElementById('render-image') || { naturalWidth: 0 }).naturalWidth)") ?? ""
            let geometry = "zoom=\(view.scrollView.zoomScale) offset=\(view.scrollView.contentOffset)"
                + " inset=\(view.scrollView.adjustedContentInset)"
            view.removeFromSuperview()
            print(
                "render \(name): loaded=\(loaded) text=\(text.debugDescription) width=\(width)"
                    + " png=\(snapshot.png.count) bytes pixels=\(snapshot.pixels?.size ?? "none")"
                    + " \(geometry)")
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

    // SPEC-355 A7 to A12: the scripted views.

    /// Every scripted card's reference reading, measured once per run and shared by A7, A8, A10.
    @MainActor private static var scriptedReferences: [String: (Reading, TimeInterval?)] = [:]

    @MainActor
    private func scriptedReference(_ scripted: Scripted, _ probe: Probe) async throws -> (Reading, TimeInterval?) {
        if let known = Self.scriptedReferences[scripted.id] {
            return known
        }
        let measured = try await probe.show(scripted.card, in: .scriptedReference(scripted.held), window: nil)
        let off = scripted.held.map(\.rawValue).sorted().joined(separator: ",")
        print("scripted reference \(scripted.id) without \(off): reached=\(measured.0.reached(scripted.card)) took=\(measured.1.map { String(format: "%.2f", $0) } ?? "-") \(measured.0.summary)")
        Self.scriptedReferences[scripted.id] = measured
        return measured
    }

    /// Whether a reference reads that the engine has no such interface (ABSENT_ALLOWED).
    private func absent(_ reading: Reading) -> Bool {
        reading.record == "absent"
    }

    @MainActor
    func test_a_planted_card_reaches_nothing_from_the_scripted_view() async throws {
        let probe = try await Probe.make()
        var opened: [String: String] = [:]
        var notLoaded: [String] = []
        for card in examined("planted cards", PLANTED) {
            let (_, took) = try await reference(card, probe)
            let (reading, _) = try await probe.show(card, in: .scripted, window: window(took))
            let arrivals = reading.arrivals
            print("scripted view \(card.id): connections=\(arrivals.connections) datagrams=\(arrivals.datagrams) \(reading.summary)")
            if !arrivals.paths.isEmpty || arrivals.connections > 0 || arrivals.datagrams > 0 {
                opened[card.id] = "paths=\(arrivals.paths) connections=\(arrivals.connections) datagrams=\(arrivals.datagrams)"
            }
            if !reading.loaded {
                notLoaded.append(card.id)
            }
        }
        // The behaviour first: no planted card, its planted click included, opens a connection, a
        // path or a datagram from the scripted view.
        XCTAssertEqual(opened, [:], "planted cards whose scripted view reached a listener")
        // The planted control: from the scripted view with L10 removed, each followed link opens a
        // connection, so the scripted view's zero is not a blind suite's.
        for name in examined("followed links", ["nav-self", "nav-blank"]) {
            let card = try XCTUnwrap(PLANTED.first { $0.id == name }, "\(name) is not planted")
            let (_, took) = try await reference(card, probe)
            let (open, _) = try await probe.show(card, in: .scriptedWithout(.L10), window: window(took))
            print("followed \(name): scripted without L10 connections=\(open.arrivals.connections)")
            XCTAssertGreaterThanOrEqual(
                open.arrivals.connections, 1,
                "\(name) opened no connection from the scripted view without L10, so its zero proves nothing")
        }
        XCTAssertEqual(notLoaded, [], "planted cards the scripted view never finished loading, so nothing was judged")
    }

    @MainActor
    func test_a_scripted_card_reaches_nothing_from_the_scripted_view() async throws {
        let probe = try await Probe.make()
        var reached: [String] = []
        var opened: [String: String] = [:]
        var notRun: [String] = []
        var blind: [String] = []
        var absentCards: Set<String> = []
        for scripted in examined("scripted cards", SCRIPTED) {
            let (reference, took) = try await scriptedReference(scripted, probe)
            if absent(reference) {
                absentCards.insert(scripted.id)
                print("scripted \(scripted.id): UNOBSERVABLE, the engine has no such interface")
                continue
            }
            if !reference.reached(scripted.card) {
                blind.append(scripted.id)
            }
            let (reading, _) = try await probe.show(scripted.card, in: .scripted, window: window(took))
            print("scripted \(scripted.id): reached=\(reading.reached(scripted.card)) \(reading.summary)")
            if reading.reached(scripted.card) {
                reached.append(scripted.id)
            }
            let arrivals = reading.arrivals
            if !arrivals.paths.isEmpty || arrivals.connections > 0 || arrivals.datagrams > 0 {
                opened[scripted.id] = "paths=\(arrivals.paths) connections=\(arrivals.connections) datagrams=\(arrivals.datagrams)"
            }
            if !reading.marker {
                notRun.append(scripted.id)
            }
        }
        // The behaviour first: no scripted card reaches anything from the scripted view.
        XCTAssertEqual(reached, [], "scripted cards that reached their probe from the scripted view")
        XCTAssertEqual(opened, [:], "scripted cards whose scripted view reached a listener")
        XCTAssertEqual(notRun, [], "scripted cards whose script did not run in the scripted view, so their absence proves nothing")
        // No reference is blind but the pinned residual, counted and printed.
        print("blind scripted references: \(blind.count) \(blind.sorted()), pinned \(BLIND_SCRIPTED.count) \(BLIND_SCRIPTED.sorted())")
        XCTAssertEqual(Set(blind), BLIND_SCRIPTED, "scripted cards whose reference reached nothing, against the pinned BLIND_SCRIPTED")
        XCTAssertTrue(absentCards.isSubset(of: ABSENT_ALLOWED), "cards whose reference reads the interface absent: \(absentCards.sorted())")
    }

    @MainActor
    func test_no_peer_connection_leaves_the_scripted_view_from_any_frame() async throws {
        let probe = try await Probe.make()
        var left: [String: String] = [:]
        var notRun: [String] = []
        var blind: [String] = []
        var absentCards: Set<String> = []
        for scripted in examined("peer-connection cards", SCRIPTED.filter { $0.held.contains(.L8) }) {
            let (reference, took) = try await scriptedReference(scripted, probe)
            if absent(reference) {
                absentCards.insert(scripted.id)
                print("peer \(scripted.id): UNOBSERVABLE, the engine has no such interface")
                continue
            }
            let sent = reference.arrivals
            if sent.datagrams + sent.connections == 0 {
                blind.append(scripted.id)
            }
            let (reading, _) = try await probe.show(scripted.card, in: .scripted, window: window(took))
            let arrivals = reading.arrivals
            print("peer \(scripted.id): reference datagrams=\(sent.datagrams) connections=\(sent.connections); scripted datagrams=\(arrivals.datagrams) connections=\(arrivals.connections) marker=\(reading.marker)")
            if arrivals.datagrams + arrivals.connections > 0 {
                left[scripted.id] = "datagrams=\(arrivals.datagrams) connections=\(arrivals.connections)"
            }
            if !reading.marker {
                notRun.append(scripted.id)
            }
        }
        // The behaviour first: no datagram and no connection leaves the scripted view, from the
        // main frame or any frame a card makes.
        XCTAssertEqual(left, [:], "peer-connection cards whose scripted view sent a datagram or opened a connection")
        XCTAssertEqual(notRun, [], "peer-connection cards whose script did not run in the scripted view, so their zero proves nothing")
        print("blind peer-connection references: \(blind.count) \(blind.sorted())")
        XCTAssertEqual(blind, [], "peer-connection cards whose reference sent nothing, so the scripted view's zero proves nothing")
        XCTAssertTrue(absentCards.isSubset(of: ABSENT_ALLOWED), "cards whose reference reads the interface absent: \(absentCards.sorted())")
    }

    @MainActor
    func test_a_scripted_card_reads_no_state_the_app_holds() async throws {
        let probe = try await Probe.make()
        let planted = "planted" + UUID().uuidString.replacingOccurrences(of: "-", with: "").lowercased()
        let origin = try XCTUnwrap(URL(string: "https://app-state.invalid/"), "the planted origin is not a URL")
        // The app's state, none of it the card's: a cookie and storage in the default store at
        // the planted origin, a file in the app's container, and a stored credential.
        let cookie = try XCTUnwrap(
            HTTPCookie(properties: [.domain: "app-state.invalid", .path: "/", .name: "planted", .value: planted]),
            "the planted cookie was not made")
        await WKWebsiteDataStore.default().httpCookieStore.setCookie(cookie)
        let file = probe.directory.appendingPathComponent("app-state.txt")
        try Data(planted.utf8).write(to: file)
        URLCredentialStorage.shared.set(
            URLCredential(user: "planted", password: planted, persistence: .forSession),
            for: URLProtectionSpace(
                host: "app-state.invalid", port: 443, protocol: "https", realm: nil,
                authenticationMethod: NSURLAuthenticationMethodHTTPBasic))
        let planter = WKWebView(frame: .zero, configuration: WKWebViewConfiguration())
        Probe.mount(planter)
        planter.loadHTMLString(Planted.document(id: "planter", head: "", body: "<p>planter</p>"), baseURL: origin)
        let planterLoaded = try await Probe.loaded(planter)
        let stored = try await Probe.evaluate(planter, "localStorage.setItem('planted', '\(planted)'); localStorage.getItem('planted')")
        planter.removeFromSuperview()
        print("app-state planter: loaded=\(planterLoaded) stored=\(stored == planted)")

        let html = appState(file: file)
        let builds: [(name: String, build: @MainActor () throws -> WKWebView)] = [
            ("reference", {
                // The default store at the planted origin, with a bridge, scripts on.
                let view = WKWebView(frame: .zero, configuration: WKWebViewConfiguration())
                view.configuration.userContentController.add(Recorder(), name: "bridge")
                view.loadHTMLString(html, baseURL: origin)
                return view
            }),
            ("scripted", {
                try CardWebViewFactory.build(html: html, ruleList: probe.ruleList, switchedOn: true)
            }),
        ]
        var records: [String: String] = [:]
        var ran: [String: Bool] = [:]
        for (name, build) in examined("app-state views", builds) {
            let view = try build()
            Probe.mount(view)
            let loaded = try await Probe.loaded(view)
            // The card writes its record after each step; the last step is given three seconds.
            try? await Task.sleep(nanoseconds: 3_000_000_000)
            records[name] = try await Probe.evaluate(view, "document.documentElement.dataset.record || ''") ?? ""
            ran[name] = try await Probe.evaluate(view, "String(document.documentElement.dataset.ran === '1')") == "true"
            view.configuration.userContentController.removeAllScriptMessageHandlers()
            view.removeFromSuperview()
            print("app-state \(name): loaded=\(loaded) ran=\(ran[name] ?? false) record=\(records[name] ?? "")")
        }
        let scripted = records["scripted"] ?? ""
        let reference = records["reference"] ?? ""
        // The behaviour first: the scripted card read none of the app's state, and saw no bridge.
        XCTAssertFalse(scripted.contains(planted), "the scripted card read a planted value: \(scripted)")
        XCTAssertTrue(scripted.contains("\"webkit\":\"undefined\""), "the scripted card's record shows a bridge, or none: \(scripted)")
        XCTAssertEqual(ran["scripted"], true, "the scripted card's script did not run, so its reading proves nothing")
        // The reference is not blind: on the default store at the planted origin it reads the
        // cookie and the storage.
        XCTAssertTrue(reference.contains("planted=\(planted)"), "the reference read no planted cookie: \(reference)")
        XCTAssertTrue(reference.contains("\"storage\":\"\(planted)\""), "the reference read no planted storage: \(reference)")
    }

    @MainActor
    func test_removing_one_control_from_the_scripted_view_opens_exactly_its_own_channels() async throws {
        let probe = try await Probe.make()
        let cards = examined("scripted cards", SCRIPTED)
        var absentCards: Set<String> = []
        for scripted in cards {
            let (reference, _) = try await scriptedReference(scripted, probe)
            if absent(reference) {
                absentCards.insert(scripted.id)
            }
        }
        var opened: [CardLayer: Set<String>] = [:]
        var expected: [CardLayer: Set<String>] = [:]
        for control in CONTROLS {
            opened[control] = []
            expected[control] = Set(
                cards.filter { $0.card.alone == control && !absentCards.contains($0.id) }.map(\.id))
        }
        var variants = 0
        for scripted in cards where !absentCards.contains(scripted.id) {
            let (_, took) = try await scriptedReference(scripted, probe)
            for control in CONTROLS {
                let (reading, _) = try await probe.show(scripted.card, in: .scriptedWithout(control), window: window(took))
                variants += 1
                print("scripted without \(control.rawValue) \(scripted.id): reached=\(reading.reached(scripted.card)) \(reading.summary)")
                if reading.reached(scripted.card) {
                    opened[control, default: []].insert(scripted.id)
                }
            }
        }
        print("examined \(variants) single-control variants")
        XCTAssertGreaterThan(variants, 0, "examined 0 single-control variants: nothing was judged")
        // The behaviour first: each control removed alone opens exactly its own channels.
        XCTAssertEqual(opened, expected, "the channels each control opened when removed alone")
        let nothing = Set(CONTROLS.filter { opened[$0, default: []].isEmpty })
        XCTAssertEqual(nothing, DEPTH_SCRIPTED, "the controls whose removal alone opened nothing, against DEPTH_SCRIPTED")
    }

    @MainActor
    func test_a_lookup_a_card_asks_for_reaches_the_witness_only_from_the_reference() async throws {
        let probe = try await Probe.make()
        let witness = try Witness()
        try await witness.start()
        defer { witness.cancel() }
        let views: [(name: String, variant: Variant)] = [
            ("reference", .reference), ("shipped", .shipped), ("scripted", .scripted),
        ]
        var queries: [String: Int] = [:]
        var ran: [String: Bool] = [:]
        for (name, variant) in examined("lookup views", views) {
            let watch: TimeInterval? = variant == .reference ? nil : 3
            let (reading, _) = try await probe.show(LOOKUP, in: variant, window: watch, witness: witness)
            queries[name] = reading.queries
            ran[name] = reading.marker
            print("lookup \(name): queries=\(reading.queries) witness datagrams=\(witness.total) marker=\(reading.marker)")
        }
        // The behaviour first: neither shipped view's lookup reaches the witness.
        XCTAssertEqual(queries["shipped"], 0, "the scripts-off view's lookup reached the witness")
        XCTAssertEqual(queries["scripted"], 0, "the scripted view's lookup reached the witness")
        XCTAssertEqual(ran["scripted"], true, "the scripted view's script did not run, so its zero proves nothing")
        XCTAssertGreaterThan(queries["reference"] ?? 0, 0, "the witness is blind: no query from the reference view")
    }

    @MainActor
    func test_a_scripted_card_renders_and_its_script_runs() async throws {
        let probe = try await Probe.make()
        let builds: [(name: String, build: @MainActor () throws -> WKWebView)] = [
            ("reference", {
                let view = CardWebViewFactory.make(layers: [], ruleList: nil)
                view.loadHTMLString(RENDER_SCRIPT, baseURL: nil)
                return view
            }),
            ("scripted", {
                try CardWebViewFactory.build(html: RENDER_SCRIPT, ruleList: probe.ruleList, switchedOn: true)
            }),
        ]
        var shown: [String: (text: String, ran: Bool)] = [:]
        for (name, build) in examined("render-script views", builds) {
            let view = try build()
            Probe.mount(view)
            let loaded = try await Probe.loaded(view)
            let ran = try await Probe.poll(5) {
                try await Probe.evaluate(view, "String(document.documentElement.dataset.ran === '1')") == "true"
            } != nil
            let text = try await Probe.evaluate(view, "document.body ? document.body.innerText : ''") ?? ""
            view.removeFromSuperview()
            print("render-script \(name): loaded=\(loaded) ran=\(ran) text=\(text.debugDescription)")
            shown[name] = (text: text, ran: ran)
        }
        let scripted = try XCTUnwrap(shown["scripted"], "the scripted view was not shown")
        let reference = try XCTUnwrap(shown["reference"], "the reference view was not shown")
        // The behaviour first: the card's script ran in the scripted view.
        XCTAssertTrue(scripted.ran, "the scripted card's script did not run in the scripted view")
        XCTAssertEqual(scripted.text, reference.text, "the scripted view's text against the reference view's")
        XCTAssertTrue(reference.ran, "the reference's script did not run, so the comparison proves nothing")
        XCTAssertTrue(reference.text.contains("the dog"), "the reference shows no hint: \(reference.text)")
    }

    /// What the app's script reads of the `permitted` card's three loads.
    struct PermittedReading: Decodable, CustomStringConvertible {
        /// The image's natural width: above zero once it decoded.
        var image = 0
        /// Every font face's status, joined: `loaded` once the one face loaded.
        var font = ""
        /// The audio element's ready state: 1 or more once its metadata is read.
        var audio = 0

        var all: Bool { image > 0 && font == "loaded" && audio >= 1 }
        var description: String { "image=\(image) font=\(font) audio=\(audio)" }
    }

    private static let permittedScript = """
        JSON.stringify({
          image: (document.getElementById('permitted-image') || { naturalWidth: 0 }).naturalWidth,
          font: Array.from(document.fonts).map((face) => face.status).join(','),
          audio: (document.getElementById('permitted-audio') || { readyState: 0 }).readyState
        })
        """

    @MainActor
    func test_the_permitted_loads_load_in_both_card_views() async throws {
        let probe = try await Probe.make()
        // The reference is the control: every layer off, so its reading shows the app's script can
        // see each load at all.
        let builds: [(name: String, build: @MainActor () throws -> WKWebView)] = [
            ("reference", {
                let view = CardWebViewFactory.make(layers: [], ruleList: nil)
                view.loadHTMLString(PERMITTED, baseURL: nil)
                return view
            }),
            ("shipped", {
                try CardWebViewFactory.build(html: PERMITTED, ruleList: probe.ruleList, switchedOn: false)
            }),
            ("scripted", {
                try CardWebViewFactory.build(html: PERMITTED, ruleList: probe.ruleList, switchedOn: true)
            }),
        ]
        var readings: [String: PermittedReading] = [:]
        var notLoaded: [String] = []
        for (name, build) in examined("permitted views", builds) {
            let view = try build()
            Probe.mount(view)
            let loaded = try await Probe.loaded(view)
            _ = try await Probe.evaluate(view, "document.fonts.load('16px permitted'); 'asked'")
            var reading = PermittedReading()
            _ = try await Probe.poll(10) {
                if let text = try await Probe.evaluate(view, Self.permittedScript),
                   let read = try? JSONDecoder().decode(PermittedReading.self, from: Data(text.utf8)) {
                    reading = read
                }
                return reading.all
            }
            view.removeFromSuperview()
            print("permitted \(name): loaded=\(loaded) \(reading)")
            readings[name] = reading
            if !loaded {
                notLoaded.append(name)
            }
        }
        // The behaviour first: each card view loads the card's data: image, font and audio.
        for name in ["shipped", "scripted"] {
            let reading = try XCTUnwrap(readings[name], "the \(name) view was not shown")
            XCTAssertGreaterThan(reading.image, 0, "the \(name) view's data: image did not decode: \(reading)")
            XCTAssertEqual(reading.font, "loaded", "the \(name) view's data: font did not load: \(reading)")
            XCTAssertGreaterThanOrEqual(reading.audio, 1, "the \(name) view's data: audio read no metadata: \(reading)")
        }
        // The control: the reference reads all three, so a card view's zero is never a blind read.
        let reference = try XCTUnwrap(readings["reference"], "the reference view was not shown")
        XCTAssertTrue(reference.all, "the reference read the permitted loads short: \(reference)")
        XCTAssertEqual(notLoaded, [], "views that never finished loading the card, so nothing was judged")
    }

    /// SPEC-349 A10 (R11): a page whose process first answers 4 seconds after it is asked, past
    /// this wait's 3-second load bound, and then finishes loading 2 seconds later, reads loaded
    /// within that bound: the load wait starts at the page's first answer.
    @MainActor
    func test_the_load_wait_starts_when_the_page_first_answers() async throws {
        let page = LatePage(startSeconds: 4, loadSeconds: 2)
        let start = Date()
        let took = try await Probe.loadTime(asking: { page.ask($0, $1) }, answerSeconds: 8, loadSeconds: 3)
        let elapsed = Date().timeIntervalSince(start)
        let line = "card probe late page: took=\(took.map { String(format: "%.2f", $0) } ?? "none")"
            + " elapsed=\(String(format: "%.2f", elapsed))"
        print(line)
        XCTAssertNotNil(took, "a page that answered after 4 s and loaded 2 s later read not loaded: \(line)")
        XCTAssertLessThan(took ?? .infinity, 3, "the load was not read within its own 3 s bound: \(line)")
    }

    /// SPEC-349 A11 (R11): a page whose process first answers 6 seconds after it is asked, past
    /// this wait's 3-second answer bound, reads not loaded, and only after that whole bound.
    @MainActor
    func test_a_page_that_never_answers_reads_not_loaded_after_the_answer_bound() async throws {
        let page = LatePage(startSeconds: 6, loadSeconds: 0)
        let start = Date()
        let took = try await Probe.loadTime(asking: { page.ask($0, $1) }, answerSeconds: 3, loadSeconds: 1)
        let elapsed = Date().timeIntervalSince(start)
        let line = "card probe silent page: took=\(took.map { String(format: "%.2f", $0) } ?? "none")"
            + " elapsed=\(String(format: "%.2f", elapsed))"
        print(line)
        XCTAssertNil(took, "a page that had not answered within its 3 s answer bound read loaded: \(line)")
        XCTAssertGreaterThanOrEqual(
            elapsed, 3, "the wait read the page not loaded before its whole answer bound: \(line)")
    }

    /// SPEC-609 A1 (R1, R2, R3, R6): an ask of a page that never answers ends at its own bound with
    /// a stall that names its label, the probe it stopped on and the bound, inside its watchdog, so
    /// the ask cannot hang its test.
    @MainActor
    func test_an_ask_a_page_never_answers_fails_at_the_ask_bound_and_names_its_probe() async throws {
        let page = SilentPage()
        Probe.probing = "a silent card in a silent view (probe 1)"
        let outcome = Outcome<String?>()
        let start = Date()
        Task { @MainActor in
            do {
                let answer = try await Probe.ask(
                    asking: { page.ask($0, $1) }, "'state'", label: "silent-state", seconds: 2)
                outcome.result = .success(answer)
            } catch {
                outcome.result = .failure(error)
            }
            outcome.finished = Date()
        }
        let returned = try await Probe.poll(10) { outcome.result != nil }
        guard returned != nil, let result = outcome.result else {
            XCTFail(
                "A1 watchdog: the ask of a silent page with `seconds: 2` had not returned by the end of the watchdog, `seconds: 10`, so the ask has no bound")
            return
        }
        let elapsed = (outcome.finished ?? Date()).timeIntervalSince(start)
        var stall: ProbeStall?
        if case .failure(let error) = result {
            stall = error as? ProbeStall
        }
        let line = "card probe silent ask: stalled=\(stall != nil) elapsed=\(String(format: "%.2f", elapsed))"
        print(line)
        let said = try XCTUnwrap(stall, "A1: the ask of a silent page came back with no probe stall: \(line)").description
        XCTAssertTrue(said.contains("silent-state"), "A1: the stall names no ask label: \(said)")
        XCTAssertTrue(
            said.contains("a silent card in a silent view (probe 1)"), "A1: the stall names no probe: \(said)")
        XCTAssertTrue(said.contains("within 2 s"), "A1: the stall names no bound in whole seconds: \(said)")
        XCTAssertGreaterThanOrEqual(elapsed, 2, "A1: the ask stalled before its bound, `seconds: 2`: \(line)")
        XCTAssertLessThan(elapsed, 10, "A1: the ask stalled only at the watchdog, `seconds: 10`: \(line)")
    }

    /// SPEC-609 A2 (R1, R2, R6): the load wait over a page that answers and then falls silent ends at
    /// its ready ask's bound with a stall that names the ready ask, inside its watchdog, so the load
    /// wait cannot hang its test.
    @MainActor
    func test_a_page_that_answers_then_falls_silent_fails_the_load_wait_at_the_ask_bound() async throws {
        let page = FallsSilentPage()
        let outcome = Outcome<TimeInterval?>()
        let start = Date()
        Task { @MainActor in
            do {
                let took = try await Probe.loadTime(
                    asking: { page.ask($0, $1) }, answerSeconds: 3, loadSeconds: 1, askSeconds: 2)
                outcome.result = .success(took)
            } catch {
                outcome.result = .failure(error)
            }
            outcome.finished = Date()
        }
        let returned = try await Probe.poll(12) { outcome.result != nil }
        guard returned != nil, let result = outcome.result else {
            XCTFail(
                "A2 watchdog: the load wait over a page that falls silent, with `askSeconds: 2`, had not returned by the end of the watchdog, `seconds: 12`, so its ready ask has no bound")
            return
        }
        let elapsed = (outcome.finished ?? Date()).timeIntervalSince(start)
        var stall: ProbeStall?
        if case .failure(let error) = result {
            stall = error as? ProbeStall
        }
        let line = "card probe falls silent: stalled=\(stall != nil) elapsed=\(String(format: "%.2f", elapsed))"
        print(line)
        let said = try XCTUnwrap(stall, "A2: the load wait came back with no probe stall: \(line)").description
        XCTAssertTrue(said.contains("the ready ask"), "A2: the stall names no ready ask: \(said)")
        XCTAssertTrue(said.contains("within 2 s"), "A2: the stall names no bound in whole seconds: \(said)")
    }
}
