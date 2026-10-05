import WebKit

/// The card view's UI delegate, layer L6 (SPEC-349 R2, ADR-360 D1), with the refusal arms a
/// scripted card meets (SPEC-355 R5): no window, no dialog shown, no capture and no motion.
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

    /// A script's alert completes with no UI shown.
    /// It does NOT stop the script that asked from running on.
    public func webView(
        _ webView: WKWebView, runJavaScriptAlertPanelWithMessage message: String,
        initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor @Sendable () -> Void
    ) {
        completionHandler()
    }

    /// A script's confirm is answered false, with no UI shown.
    /// It does NOT stop the script that asked from running on.
    public func webView(
        _ webView: WKWebView, runJavaScriptConfirmPanelWithMessage message: String,
        initiatedByFrame frame: WKFrameInfo, completionHandler: @escaping @MainActor @Sendable (Bool) -> Void
    ) {
        completionHandler(false)
    }

    /// A script's prompt is answered with nothing, with no UI shown.
    /// It does NOT stop the script that asked from running on.
    public func webView(
        _ webView: WKWebView, runJavaScriptTextInputPanelWithPrompt prompt: String,
        defaultText: String?, initiatedByFrame frame: WKFrameInfo,
        completionHandler: @escaping @MainActor @Sendable (String?) -> Void
    ) {
        completionHandler(nil)
    }

    /// A request for the camera or the microphone is denied, with no UI shown.
    /// It does NOT stop a script from asking again; each request is denied.
    public func webView(
        _ webView: WKWebView, requestMediaCapturePermissionFor origin: WKSecurityOrigin,
        initiatedByFrame frame: WKFrameInfo, type: WKMediaCaptureType,
        decisionHandler: @escaping @MainActor @Sendable (WKPermissionDecision) -> Void
    ) {
        decisionHandler(.deny)
    }

    #if os(iOS)
    /// A request for the device's orientation and motion is denied, with no UI shown. iOS only:
    /// macOS has no such request.
    /// It does NOT stop a script from asking again; each request is denied.
    public func webView(
        _ webView: WKWebView, requestDeviceOrientationAndMotionPermissionFor origin: WKSecurityOrigin,
        initiatedByFrame frame: WKFrameInfo,
        decisionHandler: @escaping @MainActor @Sendable (WKPermissionDecision) -> Void
    ) {
        decisionHandler(.deny)
    }
    #endif
}
