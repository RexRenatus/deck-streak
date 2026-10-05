import ObjectiveC
import WebKit

/// The card view's layers: the seven of SPEC-349 R2 (the schematic's section 4), then L8, the
/// peer-connection removal, and L9, the connection hold (SPEC-355 R3 and R4, section 7).
public enum CardLayer: String, CaseIterable, Sendable {
    case L1, L2, L3, L4, L5, L6, L7, L8, L9
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
        configuration(layers: [.L1, .L2], ruleList: nil)
    }

    /// The card view around a compiled list, or a refusal when there is none.
    static func build(html: String, ruleList: WKContentRuleList?) throws -> WKWebView {
        guard let ruleList else {
            throw CardViewRefusal.ruleListDidNotCompile
        }
        let view = make(layers: Set(CardLayer.allCases), ruleList: ruleList)
        // L7, no file access: the card is handed over as a string with no base URL, never as a
        // file, so its document has no origin that can read one.
        // It does NOT stop a load the card's markup names; the rule list (L3) does.
        view.loadHTMLString(html, baseURL: nil)
        return view
    }

    /// The probe's door: a view with only `layers` on, for the planted suite's reference and
    /// single-layer-off views. Internal, so only a test target reaches it.
    static func make(layers: Set<CardLayer>, ruleList: WKContentRuleList?) -> WKWebView {
        let view = WKWebView(
            frame: .zero, configuration: configuration(layers: layers, ruleList: ruleList))
        if layers.contains(.L5) {
            // L5, the navigation gate: only the first main-frame load is allowed.
            // It does NOT stop a subresource load.
            let gate = GateAdapter()
            view.navigationDelegate = gate
            Retained.keep(gate, by: view, under: Retained.gate)
        }
        if layers.contains(.L6) {
            // L6, the window refusal: no request for a new window gets one.
            // It does NOT stop a navigation in the same view.
            let refusal = WindowRefusal()
            view.uiDelegate = refusal
            Retained.keep(refusal, by: view, under: Retained.refusal)
        }
        // L4, no script message handler: nothing is added under a name to the user content
        // controller, so no script in the frame has a bridge into Swift.
        // It does NOT stop the page running script inside its own frame, were JavaScript on.
        return view
    }

    private static func configuration(
        layers: Set<CardLayer>, ruleList: WKContentRuleList?
    ) -> WKWebViewConfiguration {
        let configuration = WKWebViewConfiguration()
        if layers.contains(.L1) {
            // L1, a non-persistent data store: no cookie, cache or storage outlives the view.
            // It does NOT stop a script from reading what the page itself holds.
            configuration.websiteDataStore = WKWebsiteDataStore.nonPersistent()
        }
        if layers.contains(.L2) {
            // L2, page JavaScript off: the card's own scripts never run.
            // It does NOT stop the app's own evaluated script, or markup's own loads.
            configuration.defaultWebpagePreferences.allowsContentJavaScript = false
        }
        if layers.contains(.L3), let ruleList {
            // L3, the compiled rule list that blocks every load of every type.
            // It does NOT stop script running, a peer connection, or a navigation the app starts.
            configuration.userContentController.add(ruleList)
        }
        return configuration
    }
}

/// The navigation gate as the view's navigation delegate, layer L5 (SPEC-349 R3, ADR-360 D1).
@MainActor
final class GateAdapter: NSObject, WKNavigationDelegate {
    /// The gate this adapter asks.
    private(set) var gate = NavigationGate()
    /// Every decision the gate gave, in order: the planted suite reads it.
    private(set) var decisions: [NavigationDecision] = []

    func webView(
        _ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy) -> Void
    ) {
        // A new-window action has no target frame, so it is judged as a subframe's: cancelled.
        let request = NavigationRequest(
            isMainFrame: navigationAction.targetFrame?.isMainFrame ?? false,
            kind: NavigationKind(navigationAction.navigationType))
        let decision = gate.decide(request)
        decisions.append(decision)
        decisionHandler(decision == .allow ? .allow : .cancel)
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

    @MainActor
    static func keep(_ delegate: NSObject, by view: WKWebView, under key: Retained) {
        objc_setAssociatedObject(
            view, UnsafeRawPointer(Unmanaged.passUnretained(key).toOpaque()), delegate,
            .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
    }
}
