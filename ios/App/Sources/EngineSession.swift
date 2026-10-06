import DeckStreakFFI
import Foundation
import HarnessWire

/// The engine calls the app makes, by the pairs the adapter's allow-list holds (SPEC-336,
/// SPEC-347 R1, R7, R8). The adapter refuses any other pair before the engine sees it.
enum EngineCall {
    static let openCollection: (service: UInt32, method: UInt32) = (3, 0)
    static let deckNames: (service: UInt32, method: UInt32) = (7, 13)
    static let syncLogin: (service: UInt32, method: UInt32) = (1, 3)
}

/// Why the app could not do what it was asked, as the sentence it shows: for a refusal of the
/// engine's, the engine's own message (R7, R8).
struct Refusal: Error, Equatable {
    let sentence: String
}

/// The one owner of the engine: every engine call runs here, off the main actor, one at a time
/// (SPEC-347 R7, R8). The actor serialises the calls; the engine itself is `Sendable` because the
/// adapter's Rust side is `Send + Sync`. This first version opens nothing and answers nothing, so
/// the app's tests are red against it before the session is written.
actor EngineSession {
    /// Opens the collection at its one fixed path under Application Support (R7).
    func open() throws {}

    /// The collection's decks by name, filtered decks included, in the engine's order (R7).
    func deckNames() throws -> [String] {
        []
    }

    /// Signs in through the engine and returns the host key its `SyncAuth` carries (R8).
    func login(user: String, password: String, endpoint: String) throws -> String {
        ""
    }
}
