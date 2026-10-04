import SwiftUI

/// The harness's entry point (SPEC-339, ADR-350): one window over one model.
@main
struct HarnessApp: App {
    @State private var model = HarnessModel()

    var body: some Scene {
        WindowGroup {
            ContentView(model: model)
        }
    }
}
