import CardIsolation
import SwiftUI
import WebKit

/// The card, in the view the factory makes (SPEC-348 R9, R17, R18): a new view per face, awaited
/// in a task keyed by the face's document, so a face is never loaded into a view that showed
/// another and the factory's navigation gate stays one-shot. Its page zoom is the factory's body
/// text zoom (1 on macOS). This file sets nothing on the view but that zoom; the factory
/// alone builds it. The view object is this view's own state, because it renders the model's
/// document and holds none of the review's state.
struct CardFaceView: View {
    let document: String
    @State private var card: WKWebView?
    @State private var refusal = ""

    var body: some View {
        ZStack {
            card.map { CardHost(card: $0).id(ObjectIdentifier($0)) }
            Text(refusal).font(.footnote).padding()
                .foregroundStyle(Color.tideInk)
        }
        .background(Color.tideBackground)
        .task(id: document) {
            do {
                let made = try await CardWebViewFactory.makeCardWebView(html: document)
                made.pageZoom = CardWebViewFactory.bodyTextZoom
                card = made
                refusal = ""
            } catch {
                card = nil
                // SPEC-505 R7, R8: the refusal in DeckStreak's words, announced once
                refusal = CardRefusalWords.text(of: error)
                CardRefusalWords.announce(refusal)
            }
        }
    }
}

/// Hosts one factory view as it was made; a new view is a new host, so no view is reused. On
/// macOS the host is AppKit's (SPEC-679).
#if os(iOS)
private struct CardHost: UIViewRepresentable {
    let card: WKWebView

    func makeUIView(context: Context) -> WKWebView {
        card
    }

    func updateUIView(_ view: WKWebView, context: Context) {}
}
#else
private struct CardHost: NSViewRepresentable {
    let card: WKWebView

    func makeNSView(context: Context) -> WKWebView {
        card
    }

    func updateNSView(_ view: WKWebView, context: Context) {}
}
#endif
