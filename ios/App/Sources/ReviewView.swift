import SwiftUI

/// Where a review stands (SPEC-348 R10, the schematic's state machine). A call in flight has its
/// own phase, written before the call is awaited, so the bar is disabled while it runs.
enum ReviewPhase: Equatable, Sendable {
    case loading, question, revealing, answer, answering, finished, refused
}

/// A rating as the answer bar names it, in the bar's order (SPEC-348 R10). The codec's own
/// `Rating` numbers the engine's; this one is the app's, so a view or the model can name a
/// rating without importing the codec.
enum Rating: Int32, CaseIterable, Sendable { case again, hard, good, easy }

/// The review screen in the detail pane (SPEC-348 R9).
struct ReviewView: View {
    let model: ReviewModel
    let back: () -> Void

    var body: some View {
        EmptyView()
    }
}
