import SwiftUI
import WebKit

/// A card face's HTML, which the app did not write, in an isolated web view (SPEC-339 R6). Four
/// layers isolate it; beside each is what it does NOT stop. The planted-card proof of all four is
/// #619's; A8 asserts the two this configuration exposes.
public struct CardWebView: UIViewRepresentable {
    let html: String

    /// The rule list's one rule: block every load, of every resource type. The card's HTML is
    /// handed to the view as a string, so its own document is no load.
    static let blockEveryLoad = #"[{"trigger":{"url-filter":".*"},"action":{"type":"block"}}]"#

    /// The configuration every card face's web view starts from.
    public static func makeConfiguration() -> WKWebViewConfiguration {
        let configuration = WKWebViewConfiguration()
        // Layer 1, a non-persistent data store: no cookie, cache or storage outlives the view.
        // It does NOT stop a script from reading what the page itself holds.
        configuration.websiteDataStore = .nonPersistent()
        // Layer 2, page JavaScript off: the card's own scripts never run.
        // It does NOT stop a script the app itself injects (the harness injects none).
        configuration.defaultWebpagePreferences.allowsContentJavaScript = false
        // Layer 3, no script message handler: nothing is added to the user content controller,
        // so no script in the frame has a bridge into Swift.
        // It does NOT stop the page running script inside its own frame, were JavaScript on.
        return configuration
    }

    public func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    public func makeUIView(context: Context) -> WKWebView {
        let view = WKWebView(frame: .zero, configuration: Self.makeConfiguration())
        context.coordinator.install(in: view)
        context.coordinator.show(html, in: view)
        return view
    }

    public func updateUIView(_ view: WKWebView, context: Context) {
        context.coordinator.show(html, in: view)
    }

    /// Holds the compiled rule list and the HTML waiting for it. Nothing is loaded until the list
    /// is installed, and nothing at all if it fails to compile.
    @MainActor
    public final class Coordinator {
        private var installed = false
        private var pending: String?
        private var shown: String?

        /// Layer 4, a compiled content rule list that blocks every load, installed before the
        /// first HTML is handed over. It does NOT stop script running, and it does not cover a
        /// navigation the app itself starts.
        func install(in view: WKWebView) {
            Task { [weak self, weak view] in
                let list = try? await WKContentRuleListStore.default().compileContentRuleList(
                    forIdentifier: "card-blocks-every-load",
                    encodedContentRuleList: CardWebView.blockEveryLoad)
                guard let self, let view, let list else { return }
                view.configuration.userContentController.add(list)
                self.installed = true
                if let pending = self.pending {
                    self.load(pending, in: view)
                }
            }
        }

        func show(_ html: String, in view: WKWebView) {
            guard html != shown else { return }
            guard installed else {
                pending = html
                return
            }
            load(html, in: view)
        }

        private func load(_ html: String, in view: WKWebView) {
            pending = nil
            shown = html
            view.loadHTMLString(html, baseURL: nil)
        }
    }
}
