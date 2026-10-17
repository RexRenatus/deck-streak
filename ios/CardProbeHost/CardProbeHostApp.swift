// The card probe's test host (SPEC-349 R5, R9): an empty app whose only purpose is to give the
// planted suite a window and the local-networking exception its listeners need. It is never
// archived and never shipped.
import SwiftUI

@main
struct CardProbeHostApp: App {
    var body: some Scene {
        WindowGroup {
            Color.clear
        }
    }
}
