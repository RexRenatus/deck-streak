import SwiftUI

/// The app's entry point (SPEC-347 R6, ADR-358): one window over one model. The entry decides
/// nothing, so the split view that does is `ShellView`, beside the deck list. The window opens
/// at a default size and resizes, and the Review menu carries the review's actions with their
/// keys, which a Mac shows in its menu bar and an iPad reads as key commands (SPEC-679).
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
        .defaultSize(width: 1024, height: 768)
        .windowResizability(.contentMinSize)
        .commands {
            CommandMenu("Review") {
                Group {
                    Button("Show Answer") {
                        Task { await model.review?.perform(.showAnswer) }
                    }
                    .keyboardShortcut(.return)
                    Button("Again") {
                        Task { await model.review?.perform(.rate(.again)) }
                    }
                    .keyboardShortcut("1")
                    Button("Good") {
                        Task { await model.review?.perform(.rate(.good)) }
                    }
                    .keyboardShortcut("2")
                }
                .disabled(model.review == nil)
            }
        }
    }
}
