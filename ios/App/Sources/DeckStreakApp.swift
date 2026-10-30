import SwiftUI

/// The app's entry point (SPEC-347 R6, ADR-358): one window over one model. The entry decides
/// nothing, so the split view that does is `ShellView`, beside the deck list.
@main
struct DeckStreakApp: App {
    @State private var model: AppModel

    init() {
        _model = State(initialValue: AppModel())
    }

    var body: some Scene {
        WindowGroup {
            ShellView(model: model)
        }
    }
}
