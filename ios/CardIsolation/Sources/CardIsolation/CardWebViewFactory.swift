import ObjectiveC
import WebKit

/// The card view's layers: the seven of SPEC-349 R2 (the schematic's section 4), then L8, the
/// peer-connection removal (SPEC-355 R3, section 7), then L10 to L13, the containment layer for
/// card scripts (SPEC-361 R3 to R6, section 8). L9 is retired (SPEC-361 R2), and its number is
/// not reused.
public enum CardLayer: String, CaseIterable, Sendable {
    case L1, L2, L3, L4, L5, L6, L7, L8, L10, L11, L12, L13
}

/// Why the factory built no view.
public enum CardViewRefusal: Error, Equatable, Sendable {
    /// The rule list did not compile, so no view is built: fail closed.
    case ruleListDidNotCompile
}

/// The one public way to build a card's web view (SPEC-349 R1, ADR-360 D2). Every layer is set
/// here and nowhere else; beside each is what it does NOT stop.
@MainActor
public enum CardWebViewFactory {
    /// A web view showing `html`, a card face the app did not write. The rule list is compiled
    /// first; when it does not compile, this throws `CardViewRefusal.ruleListDidNotCompile` and
    /// builds no view.
    public static func makeCardWebView(html: String) async throws -> WKWebView {
        let compiled = await RuleList.compile()
        return try build(html: html, ruleList: compiled)
    }

    /// The configuration L1 and L2 set, for a reader of the layers (SPEC-339's A8). It is not a
    /// card view: the rule list and the delegates come only with `makeCardWebView(html:)`.
    public static func configuration() -> WKWebViewConfiguration {
        configuration(
            layers: [.L1, .L2], ruleList: nil,
            built: Built(switchedOn: false, scriptsFollowVerdict: true))
    }

    /// The card view around a compiled list, or a refusal when there is none. `switchedOn` is the
    /// switch the verdict reads (SPEC-355 R1, R2).
    static func build(
        html: String, ruleList: WKContentRuleList?, switchedOn: Bool = CardScripts.switchedOn
    ) throws -> WKWebView {
        guard let ruleList else {
            throw CardViewRefusal.ruleListDidNotCompile
        }
        let view = make(layers: Set(CardLayer.allCases), ruleList: ruleList, switchedOn: switchedOn)
        load(html, into: view)
        return view
    }

    /// Hands `html` to `view` the one way a card is loaded (SPEC-361 R5, R8): the document policy
    /// first, then the card, as a string with no base URL. What it handed is kept with the view,
    /// and is the one record L12 is read back from. Internal, so only a test target reaches it
    /// beside the factory.
    static func load(_ html: String, into view: WKWebView) {
        // L12, the document policy: its element goes ahead of the card's markup, so the parser
        // places it first in the card's `head`.
        // It does NOT stop a resource hint's early connection or a navigation; L3 and L5 do.
        let html = DocumentPolicy.prefixed(html)
        Retained.keep(Handed(html: html), by: view, under: Retained.handed)
        // L7, no file access: the card is handed over as a string with no base URL, never as a
        // file, so its document has no origin that can read one.
        // It does NOT stop a load the card's markup names; the rule list (L3) does.
        view.loadHTMLString(html, baseURL: nil)
    }

    /// The probe's door: a view with only `layers` on, for the planted suite's reference and
    /// single-layer-off views. Internal, so only a test target reaches it.
    static func make(
        layers: Set<CardLayer>, ruleList: WKContentRuleList?, switchedOn: Bool = false
    ) -> WKWebView {
        let built = Built(switchedOn: switchedOn, scriptsFollowVerdict: layers.contains(.L2))
        let view = WKWebView(
            frame: .zero,
            configuration: configuration(layers: layers, ruleList: ruleList, built: built))
        // What was built stays with the view, so its verdict reads the view (SPEC-355 R2).
        Retained.keep(built, by: view, under: Retained.built)
        if layers.contains(.L5) {
            // L5, the navigation gate: only the first main-frame load is allowed.
            // It does NOT stop a subresource load.
            let gate = GateAdapter()
            view.navigationDelegate = gate
            Retained.keep(gate, by: view, under: Retained.gate)
        }
        if layers.contains(.L6) {
            // L6, the window refusal: no request for a new window gets one, and no dialog,
            // capture or motion request is granted (SPEC-355 R5).
            // It does NOT stop a navigation in the same view.
            let refusal = WindowRefusal()
            view.uiDelegate = refusal
            Retained.keep(refusal, by: view, under: Retained.refusal)
        }
        if layers.contains(.L13) {
            // L13, no link preview: a long press on a link shows no preview of it, and L6's
            // context-menu arm offers no menu item that opens it (SPEC-361 R6).
            // It does NOT stop a tap on the link; L10 refuses its activation.
            view.allowsLinkPreview = false
        }
        // L4, no script message handler: nothing is added under a name to the user content
        // controller, so no script in the frame has a bridge into Swift.
        // It does NOT stop the page running script inside its own frame, were JavaScript on.
        return view
    }

    /// The switch's verdict for a view this factory built, or nil for a view it did not build
    /// (SPEC-355 R2): read from the view as it is now, never from what the factory meant to do.
    static func verdict(of view: WKWebView) -> CardScripts.Verdict? {
        verdict(of: view, requestURL: nil)
    }

    /// The verdict, with the URL of a main-frame navigation the view is about to make, which L7
    /// reads beside the view's own.
    static func verdict(of view: WKWebView, requestURL: URL?) -> CardScripts.Verdict? {
        guard let built = Retained.kept(Built.self, by: view, under: Retained.built) else {
            return nil
        }
        return CardScripts.decide(
            switchedOn: built.switchedOn,
            present: present(in: view, built: built, requestURL: requestURL))
    }

    /// What `view` was built with, or nil for a view this factory did not build.
    static func built(of view: WKWebView) -> Built? {
        Retained.kept(Built.self, by: view, under: Retained.built)
    }

    /// Every required control `view` carries now, each read back from its configuration, its
    /// delegates and its URL (SPEC-355 R2).
    private static func present(
        in view: WKWebView, built: Built, requestURL: URL?
    ) -> Set<CardLayer> {
        let configuration = view.configuration
        let store = configuration.websiteDataStore
        let controller = configuration.userContentController as? CardContentController
        var present: Set<CardLayer> = []
        if !store.isPersistent {
            present.insert(.L1)
        }
        if let controller, controller.ruleLists.contains(RuleList.identifier) {
            present.insert(.L3)
        }
        if let controller, controller.named == 0 {
            present.insert(.L4)
        }
        if view.navigationDelegate is GateAdapter {
            present.insert(.L5)
        }
        if view.uiDelegate is WindowRefusal {
            present.insert(.L6)
        }
        if isBlank(view.url) && isBlank(requestURL) {
            present.insert(.L7)
        }
        let removal = configuration.userContentController.userScripts.contains { script in
            script.source == PeerConnectionRemoval.source && !script.isForMainFrameOnly
                && script.injectionTime != .atDocumentEnd
        }
        if removal {
            present.insert(.L8)
        }
        if installed(LinkActivationRefusal.source, in: configuration) {
            present.insert(.L10)
        }
        if installed(PageGuard.source, in: configuration) {
            present.insert(.L11)
        }
        let handed = Retained.kept(Handed.self, by: view, under: Retained.handed)
        if let handed, handed.html.hasPrefix(DocumentPolicy.prefix) {
            present.insert(.L12)
        }
        if !view.allowsLinkPreview && view.uiDelegate is WindowRefusal {
            present.insert(.L13)
        }
        return present
    }

    /// Whether a user script with `source` is installed to run in every frame before the card's
    /// own script (SPEC-361 R8).
    private static func installed(_ source: String, in configuration: WKWebViewConfiguration) -> Bool {
        configuration.userContentController.userScripts.contains { script in
            script.source == source && !script.isForMainFrameOnly && script.injectionTime != .atDocumentEnd
        }
    }

    /// Whether `url` names no document of its own: none, or the blank page a string with no base
    /// URL is shown at.
    private static func isBlank(_ url: URL?) -> Bool {
        guard let url else {
            return true
        }
        return url.absoluteString == "about:blank"
    }

    private static func configuration(
        layers: Set<CardLayer>, ruleList: WKContentRuleList?, built: Built
    ) -> WKWebViewConfiguration {
        let configuration = WKWebViewConfiguration()
        #if os(iOS)
            // A card's HTML video plays inline, as on the desktop, rather than taking the screen
            // on an iPhone, whose default is off; the document writes `playsinline` on each video
            // too (SPEC-393 R10, R11). It loads nothing: a video still reaches the view only as a
            // `data:` URL inside the document, and a tap still starts it.
            configuration.allowsInlineMediaPlayback = true
        #endif
        // The controller records the rule lists and named handlers added to it, which WebKit
        // does not list, so the verdict can read L3 and L4 back.
        configuration.userContentController = CardContentController()
        if layers.contains(.L1) {
            // L1, a non-persistent data store: no cookie, cache or storage outlives the view.
            // It does NOT stop a script from reading what the page itself holds.
            configuration.websiteDataStore = WKWebsiteDataStore.nonPersistent()
        }
        if layers.contains(.L2) {
            // L2, page JavaScript off: the card's own scripts never run, unless the switch's
            // verdict for this view is to run them (the gate sets it per navigation).
            // It does NOT stop the app's own evaluated script, or markup's own loads.
            configuration.defaultWebpagePreferences.allowsContentJavaScript = false
        }
        if layers.contains(.L3), let ruleList {
            // L3, the compiled rule list that blocks every load of every type.
            // It does NOT stop script running, a peer connection, or a navigation the app starts.
            configuration.userContentController.add(ruleList)
        }
        if layers.contains(.L8) {
            // L8, the peer-connection removal: one user script, at document start, in every
            // frame, in the page's own world, deletes every peer-connection global first.
            // It does NOT stop a load, a navigation or a lookup; L3 and L5 hold those.
            configuration.userContentController.addUserScript(
                WKUserScript(
                    source: PeerConnectionRemoval.source, injectionTime: .atDocumentStart,
                    forMainFrameOnly: false, in: .page))
        }
        if layers.contains(.L10) {
            // L10, the link-activation refusal: one user script, in every frame, before the card's
            // own script, in a world the app owns, cancels every link's activation (SPEC-361 R3).
            // It does NOT see a detached link's activation; L11 refuses that.
            configuration.userContentController.addUserScript(LinkActivationRefusal.userScript)
        }
        if layers.contains(.L11) {
            // L11, the page guard: one user script, in every frame, before the card's own script,
            // in the page's world, refuses a detached node's activation and every rewrite of the
            // card's document (SPEC-361 R4).
            // It does NOT stop a connected link's activation; L10 refuses that.
            configuration.userContentController.addUserScript(PageGuard.userScript)
        }
        return configuration
    }
}

/// What the factory's load handed one view, kept with the view (SPEC-361 R8): the policy and the
/// card as one string, which L12 is read back from.
@MainActor
final class Handed: NSObject {
    let html: String

    init(html: String) {
        self.html = html
        super.init()
    }
}

/// What the factory built into one view, kept with the view (SPEC-355 R2): the switch the view
/// was built under, and whether page JavaScript follows the verdict (L2 was built).
@MainActor
final class Built: NSObject {
    let switchedOn: Bool
    let scriptsFollowVerdict: Bool

    init(switchedOn: Bool, scriptsFollowVerdict: Bool) {
        self.switchedOn = switchedOn
        self.scriptsFollowVerdict = scriptsFollowVerdict
        super.init()
    }
}

/// The card view's user content controller. WebKit lists neither the rule lists nor the named
/// handlers a controller holds, so this one records both as they are added and removed, and the
/// verdict reads L3 and L4 from it.
/// It does NOT see a handler added with a reply; the tree guard refuses that call outside test
/// targets (SPEC-349 A4).
final class CardContentController: WKUserContentController {
    /// The identifiers of the rule lists installed now.
    private(set) var ruleLists: Set<String> = []
    /// How many handlers were ever added under a name. It never falls: a bridge once opened is
    /// read as open, so the verdict fails closed.
    private(set) var named = 0

    override func add(_ contentRuleList: WKContentRuleList) {
        ruleLists.insert(contentRuleList.identifier)
        super.add(contentRuleList)
    }

    override func remove(_ contentRuleList: WKContentRuleList) {
        ruleLists.remove(contentRuleList.identifier)
        super.remove(contentRuleList)
    }

    override func removeAllContentRuleLists() {
        ruleLists.removeAll()
        super.removeAllContentRuleLists()
    }

    override func add(_ scriptMessageHandler: WKScriptMessageHandler, name: String) {
        named += 1
        super.add(scriptMessageHandler, name: name)
    }

    override func add(
        _ scriptMessageHandler: WKScriptMessageHandler, contentWorld world: WKContentWorld,
        name: String
    ) {
        named += 1
        super.add(scriptMessageHandler, contentWorld: world, name: name)
    }
}

/// The navigation gate as the view's navigation delegate, layer L5 (SPEC-349 R3, ADR-360 D1).
/// It also sets L2 per navigation from the switch's verdict (SPEC-355 R2).
@MainActor
final class GateAdapter: NSObject, WKNavigationDelegate {
    /// The gate this adapter asks.
    private(set) var gate = NavigationGate()
    /// Every decision the gate gave, in order: the planted suite reads it.
    private(set) var decisions: [NavigationDecision] = []

    func webView(
        _ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
        preferences: WKWebpagePreferences,
        decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy, WKWebpagePreferences) -> Void
    ) {
        // A new-window action has no target frame, so it is judged as a subframe's: cancelled.
        let isMainFrame = navigationAction.targetFrame?.isMainFrame ?? false
        let request = NavigationRequest(
            isMainFrame: isMainFrame, kind: NavigationKind(navigationAction.navigationType))
        let decision = gate.decide(request)
        decisions.append(decision)
        if let built = CardWebViewFactory.built(of: webView), built.scriptsFollowVerdict {
            // L2 follows the verdict, read back from the view now: page JavaScript is on only
            // when the switch is on and every required control is present.
            let verdict = CardWebViewFactory.verdict(
                of: webView, requestURL: isMainFrame ? navigationAction.request.url : nil)
            preferences.allowsContentJavaScript = verdict == .run
        }
        decisionHandler(decision == .allow ? .allow : .cancel, preferences)
    }
}

extension NavigationKind {
    /// WebKit's navigation type, as the gate names it. A type WebKit adds later is read as a
    /// link, which the gate never allows after the first load.
    init(_ type: WKNavigationType) {
        switch type {
        case .linkActivated: self = .link
        case .formSubmitted: self = .formSubmit
        case .backForward: self = .backForward
        case .reload: self = .reload
        case .formResubmitted: self = .formResubmit
        case .other: self = .other
        @unknown default: self = .link
        }
    }
}

/// A view's delegates are weak references, so the view keeps its own as associated objects,
/// keyed by the address of an object that lives as long as the process.
private final class Retained: Sendable {
    static let gate = Retained()
    static let refusal = Retained()
    static let built = Retained()
    static let handed = Retained()

    @MainActor
    static func keep(_ delegate: NSObject, by view: WKWebView, under key: Retained) {
        objc_setAssociatedObject(
            view, UnsafeRawPointer(Unmanaged.passUnretained(key).toOpaque()), delegate,
            .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
    }

    @MainActor
    static func kept<T>(_ type: T.Type, by view: WKWebView, under key: Retained) -> T? {
        objc_getAssociatedObject(view, UnsafeRawPointer(Unmanaged.passUnretained(key).toOpaque()))
            as? T
    }
}
