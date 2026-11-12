import CardIsolation
import SwiftUI
import UIKit
import WebKit

/// A card face's HTML, which the app did not write, in the card view (SPEC-339 R6, SPEC-349 R1).
/// Every isolation layer is set by `CardWebViewFactory`, the one place a card's web view is built,
/// and beside each layer there is what it does NOT stop. This view only shows what the factory
/// returns: a card whose rule list did not compile shows nothing.
public struct CardWebView: UIViewRepresentable {
    let html: String

    /// The configuration L1 and L2 set, as the factory builds it; SPEC-339's A8 reads it.
    public static func makeConfiguration() -> WKWebViewConfiguration {
        CardWebViewFactory.configuration()
    }

    public func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    public func makeUIView(context: Context) -> UIView {
        let container = UIView(frame: .zero)
        context.coordinator.show(html, in: container)
        return container
    }

    public func updateUIView(_ container: UIView, context: Context) {
        context.coordinator.show(html, in: container)
    }

    /// Builds each card's view through the factory, one card at a time: a newer card cancels the
    /// build of an older one, and only the newest is shown, filling the container.
    @MainActor
    public final class Coordinator {
        private var shown: String?
        private var building: Task<Void, Never>?

        func show(_ html: String, in container: UIView) {
            guard html != shown else { return }
            shown = html
            building?.cancel()
            building = Task { [weak container] in
                let card = try? await CardWebViewFactory.makeCardWebView(html: html)
                guard !Task.isCancelled, let container else { return }
                for old in container.subviews {
                    old.removeFromSuperview()
                }
                guard let card else { return }
                card.frame = container.bounds
                card.autoresizingMask = [.flexibleWidth, .flexibleHeight]
                container.addSubview(card)
            }
        }
    }
}
