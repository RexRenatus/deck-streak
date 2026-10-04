import SwiftUI

/// The harness's entry point (SPEC-339, ADR-350): one window over one model.
@main
struct HarnessApp: App {
    @State private var model = HarnessModel()

    var body: some Scene {
        WindowGroup {
            HarnessView(model: model)
        }
    }
}

/// The deck list beside the study pane (SPEC-339 R4 to R8). The iPhone starts on the deck list and
/// shows the study pane once a deck is chosen; the iPad shows both columns side by side. The
/// columns' layout is the view's own state; the review session's is the model's.
struct HarnessView: View {
    @Bindable var model: HarnessModel
    @State private var columnVisibility = NavigationSplitViewVisibility.all
    @State private var compactColumn = NavigationSplitViewColumn.sidebar

    var body: some View {
        NavigationSplitView(
            columnVisibility: $columnVisibility, preferredCompactColumn: $compactColumn
        ) {
            DeckListView(model: model)
        } detail: {
            ReviewView(model: model)
        }
        .navigationSplitViewStyle(.balanced)
        .onChange(of: model.chosenDeck) {
            if model.chosenDeck != nil {
                compactColumn = .detail
            }
        }
        .task { await model.start() }
    }
}
