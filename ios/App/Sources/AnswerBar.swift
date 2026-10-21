import SwiftUI

/// A gesture on the review screen, which `ReviewModel.perform` takes (SPEC-348 R16); bury and
/// flag act on the shown card (SPEC-358 R5).
enum ReviewAction: Equatable, Sendable { case showAnswer, rate(Rating), replay, stop, bury, flag }

/// The bottom bar, the width of the detail pane (SPEC-348 R10, R11, R16): Show Answer, one
/// button the width of the bar; then Again and Good, the two grades a press records, Good the
/// primary one, each named by its title with its interval as its value (SPEC-365 R13); and at the
/// designed end, the way back to the deck list. A button is disabled unless the phase is the one
/// its gesture starts from.
struct AnswerBar: View {
    let model: ReviewModel
    let back: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            if model.phase == .finished {
                Button(action: back) {
                    Text("Back to Decks").frame(maxWidth: .infinity, minHeight: 44)
                }
                .buttonStyle(.borderedProminent)
            } else if model.revealed {
                rating(.again, "Again", .bordered)
                rating(.good, "Good", .borderedProminent)
            } else {
                Button {
                    Task { await model.perform(.showAnswer) }
                } label: {
                    Text("Show Answer").frame(maxWidth: .infinity, minHeight: 44)
                }
                .buttonStyle(.borderedProminent)
                .disabled(model.phase != .question)
            }
        }
        .padding(.horizontal)
        .padding(.vertical, 8)
        .background(.bar)
    }

    /// One rating's button: its title is its accessible name, and its interval, shown under the
    /// title, is its accessible value and nothing more (R11).
    private func rating<Style: PrimitiveButtonStyle>(
        _ rating: Rating, _ title: String, _ style: Style
    ) -> some View {
        let interval = model.intervals.dropFirst(Int(rating.rawValue)).prefix(1).joined()
        return Button {
            Task { await model.perform(.rate(rating)) }
        } label: {
            VStack(spacing: 2) {
                Text(title).font(.headline)
                Text(interval).font(.caption).accessibilityHidden(true)
            }
            .frame(maxWidth: .infinity, minHeight: 44)
        }
        .buttonStyle(style)
        .accessibilityValue(interval)
        .disabled(model.phase != .answer)
    }
}
