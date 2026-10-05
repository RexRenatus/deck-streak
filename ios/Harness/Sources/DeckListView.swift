import SwiftUI

/// The collection's decks by name, in the order the engine gives them, under the open's outcome
/// (SPEC-339 R3, R4). Choosing a deck shows the study pane; the harness studies the collection's
/// current deck whichever is chosen, since no allowed call changes it.
struct DeckListView: View {
    @Bindable var model: HarnessModel

    var body: some View {
        List(model.deckNames, id: \.self, selection: $model.chosenDeck) { name in
            Text(name).accessibilityIdentifier("deck-row-\(name)")
        }
        .accessibilityIdentifier("deck-list")
        .safeAreaInset(edge: .top) {
            Text(model.openStatus).accessibilityIdentifier("open-status")
        }
        .navigationTitle("Decks")
        .onChange(of: model.deckNames) {
            // Section 7's launch interval ends the first time the list shows names.
            if !model.deckNames.isEmpty {
                Signposts.endLaunch()
            }
        }
    }
}
