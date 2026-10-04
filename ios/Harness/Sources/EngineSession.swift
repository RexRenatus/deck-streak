import DeckStreakFFI
import Foundation
import HarnessWire

/// The one owner of the engine: every engine call runs here, off the main actor (SPEC-339 R3, R8).
/// A stub: it holds no engine yet and calls nothing, but naming the type links the engine into the
/// app.
actor EngineSession {
    private var engine: Engine?

    func open() async throws {}
}
