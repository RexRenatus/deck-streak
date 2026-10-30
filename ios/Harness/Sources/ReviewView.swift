import SwiftUI

/// The study pane (SPEC-339 R5, R7, R8): study the current deck's next card, see its question,
/// answer it Good, and read the deck's new count. A refusal is shown as its own sentence.
struct ReviewView: View {
    var model: HarnessModel

    var body: some View {
        VStack(spacing: 16) {
            Button("Study") { Task { await model.study() } }
                .accessibilityIdentifier("study")
            if let refusal = model.refusal {
                Text(refusal).accessibilityIdentifier("refusal")
            }
            CardWebView(html: model.questionHTML)
                .accessibilityIdentifier("card-web-view")
            Button("Good") { Task { await model.answerGood() } }
                .accessibilityIdentifier("answer-good")
            Text(model.newCount).accessibilityIdentifier("new-count")
        }
        .padding()
        .navigationTitle("Study")
    }
}
