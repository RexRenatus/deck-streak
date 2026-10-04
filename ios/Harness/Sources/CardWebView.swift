import SwiftUI
import WebKit

/// A card face's HTML, which the app did not write, in an isolated web view (SPEC-339 R6). A stub:
/// its configuration is the default one until the isolation is built.
public struct CardWebView: UIViewRepresentable {
    let html: String

    /// The configuration every card face's web view starts from.
    public static func makeConfiguration() -> WKWebViewConfiguration {
        WKWebViewConfiguration()
    }

    public func makeUIView(context: Context) -> WKWebView {
        WKWebView(frame: .zero, configuration: Self.makeConfiguration())
    }

    public func updateUIView(_ view: WKWebView, context: Context) {}
}
