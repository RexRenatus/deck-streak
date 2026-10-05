import WebKit

/// The card view's UI delegate, layer L6 (SPEC-349 R2, ADR-360 D1).
@MainActor
public final class WindowRefusal: NSObject, WKUIDelegate {
    override public init() {
        super.init()
    }

    /// Every request for a new window gets none: a card's `target=_blank` link, or a script's
    /// `window.open`, opens nothing.
    /// It does NOT stop a navigation in the same view; the gate (L5) does.
    public func webView(
        _ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
        for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures
    ) -> WKWebView? {
        nil
    }
}
