import WebKit

/// The card view's seven layers (SPEC-349 R2, the schematic's section 4).
public enum CardLayer: String, CaseIterable, Sendable {
    case L1, L2, L3, L4, L5, L6, L7
}

/// Why the factory built no view.
public enum CardViewRefusal: Error, Equatable, Sendable {
    /// The rule list did not compile, so no view is built: fail closed.
    case ruleListDidNotCompile
}

/// The one public way to build a card's web view (SPEC-349 R1, ADR-360 D2).
@MainActor
public enum CardWebViewFactory {
    /// A web view showing `html`, a card face the app did not write.
    public static func makeCardWebView(html: String) async throws -> WKWebView {
        let view = make(layers: Set(CardLayer.allCases), ruleList: nil)
        view.loadHTMLString(html, baseURL: nil)
        return view
    }

    /// The probe's door: a view with only `layers` on, for the planted suite's reference and
    /// single-layer-off views. Internal, so only a test target reaches it.
    static func make(layers: Set<CardLayer>, ruleList: WKContentRuleList?) -> WKWebView {
        WKWebView(frame: .zero, configuration: WKWebViewConfiguration())
    }
}

/// The navigation gate as the view's navigation delegate, layer L5 (SPEC-349 R3, ADR-360 D1).
@MainActor
final class GateAdapter: NSObject, WKNavigationDelegate {
    /// The gate this adapter asks.
    private(set) var gate = NavigationGate()
    /// Every decision the gate gave, in order: the planted suite reads it.
    private(set) var decisions: [NavigationDecision] = []
}
