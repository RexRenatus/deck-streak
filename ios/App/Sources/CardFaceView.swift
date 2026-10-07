import CardIsolation
import SwiftUI
import UIKit
import WebKit

/// The card, in the view the factory makes (SPEC-348 R9, R17, R18): a new view per face, awaited
/// in a task keyed by the face's document, so a face is never loaded into a view that showed
/// another and the factory's navigation gate stays one-shot. Its page zoom is the body text
/// style's Dynamic Type scale. This file sets nothing on the view but that zoom; the factory
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
        }
        .task(id: document) {
            do {
                let made = try await CardWebViewFactory.makeCardWebView(html: document)
                made.pageZoom = UIFontMetrics(forTextStyle: .body).scaledValue(for: 1)
                card = made
                refusal = ""
            } catch {
                card = nil
                refusal = String(describing: error)
            }
        }
    }
}

/// Hosts one factory view as it was made; a new view is a new host, so no view is reused.
private struct CardHost: UIViewRepresentable {
    let card: WKWebView

    func makeUIView(context: Context) -> WKWebView {
        card
    }

    func updateUIView(_ view: WKWebView, context: Context) {}
}
