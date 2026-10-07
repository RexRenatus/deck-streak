import SwiftUI

/// A gesture on the review screen, which `ReviewModel.perform` takes (SPEC-348 R16).
enum ReviewAction: Equatable, Sendable { case showAnswer, rate(Rating), replay, stop }

/// The bottom bar: Show Answer, then the four ratings (SPEC-348 R10, R11).
struct AnswerBar: View {
    let model: ReviewModel

    var body: some View {
        EmptyView()
    }
}
