import SwiftUI

/// The harness's one screen: the deck list beside the study pane (SPEC-339 R4 to R7). The iPhone
/// starts on the deck list, and the iPad shows both columns.
struct ContentView: View {
    var model: HarnessModel
    @State private var columnVisibility = NavigationSplitViewVisibility.all
    @State private var compactColumn = NavigationSplitViewColumn.sidebar

    var body: some View {
        NavigationSplitView(
            columnVisibility: $columnVisibility, preferredCompactColumn: $compactColumn
        ) {
            List(model.deckNames, id: \.self) { name in
                Text(name).accessibilityIdentifier("deck-row-\(name)")
            }
            .accessibilityIdentifier("deck-list")
            .safeAreaInset(edge: .top) {
                Text(model.openStatus).accessibilityIdentifier("open-status")
            }
        } detail: {
            VStack {
                Button("Study") { Task { await model.study() } }
                    .accessibilityIdentifier("study")
                CardWebView(html: model.questionHTML)
                    .accessibilityIdentifier("card-web-view")
                Button("Good") { Task { await model.answerGood() } }
                    .accessibilityIdentifier("answer-good")
                Text(model.newCount).accessibilityIdentifier("new-count")
            }
        }
        .task { await model.start() }
    }
}
