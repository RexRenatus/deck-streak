import SwiftUI

/// The shell (SPEC-347 R6): one split view, the deck list as its sidebar and a detail pane that
/// reads "Choose a deck" until the review screen exists. The iPhone starts on the deck list and
/// shows the detail once a deck is chosen; the iPad shows both columns side by side. The columns'
/// layout is the view's own state; the chosen deck is the model's, so it survives the collapse.
struct ShellView: View {
    @Bindable var model: AppModel
    @State private var columnVisibility = NavigationSplitViewVisibility.all
    @State private var compactColumn = NavigationSplitViewColumn.sidebar

    var body: some View {
        NavigationSplitView(
            columnVisibility: $columnVisibility, preferredCompactColumn: $compactColumn
        ) {
            DeckListView(model: model)
        } detail: {
            Text("Choose a deck").accessibilityIdentifier("detail")
        }
        .navigationSplitViewStyle(.balanced)
        .onChange(of: model.chosenDeck) {
            if model.chosenDeck != nil {
                compactColumn = .detail
            }
        }
        .sheet(isPresented: $model.showingAccount) {
            AccountView(model: model)
        }
        .task { await model.start() }
    }
}

/// The collection's decks by name, in the order the engine gives them (R7). A refusal shows its
/// sentence in place of the list. The toolbar carries the one account button (R6).
struct DeckListView: View {
    @Bindable var model: AppModel

    var body: some View {
        Group {
            if let refusal = model.refusal {
                Text(refusal).accessibilityIdentifier("refusal")
            } else {
                List(model.deckNames, id: \.self, selection: $model.chosenDeck) { name in
                    Text(name).accessibilityIdentifier("deck-row-\(name)")
                }
                .accessibilityIdentifier("deck-list")
            }
        }
        .navigationTitle("Decks")
        .toolbar {
            Button("Account") { model.showingAccount = true }
                .accessibilityIdentifier("account")
        }
    }
}
