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

/// A deck as the sidebar lists it: its id, which (7,22) takes, and its name (SPEC-348 R9).
struct Deck: Hashable, Sendable {
    let id: Int64
    let name: String
}

/// A face as the review screen shows it: the engine's document for the factory's view, the clips
/// it plays and replays, the media it could not show, and the languages its speech uses, each
/// once, for the voice picker (SPEC-348 R9, R13, R14).
struct ReviewFace: Equatable, Sendable {
    var document: String
    var autoplay: [ReviewClip]
    var replay: [ReviewClip]
    var omitted: [String]
    var languages: [String]

    /// The face before the first card and after the last: nothing to show and nothing to play.
    static let empty = ReviewFace(
        document: "", autoplay: [], replay: [], omitted: [], languages: [])
}

/// One language the current face speaks, with the installed voices the picker offers for it and
/// the one chosen, nil for the system's default (SPEC-348 R14).
struct VoiceLanguage: Equatable, Sendable {
    var language: String
    var options: [InstalledVoice]
    var chosen: String?
}

/// The one owner of the engine: every engine call runs here, off the main actor, one at a time
/// (SPEC-347 R7, R8). The actor serialises the calls; the engine itself is `Sendable` because the
/// adapter's Rust side is `Send + Sync`.
actor EngineSession {
    private var engine: Engine?

    /// Opens the collection at its one fixed path under Application Support, beside its media
    /// folder, creating the directory on the first launch (R7). The engine creates a collection
    /// that is not there yet, so a fresh install opens one holding the engine's default deck alone.
    func open(arguments: [String] = ProcessInfo.processInfo.arguments) throws {
        let directory = URL.applicationSupportDirectory.appending(
            path: "DeckStreak", directoryHint: .isDirectory)
        let media = directory.appending(path: "collection.media", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: media, withIntermediateDirectories: true)
        _ = try call(
            EngineCall.openCollection,
            Requests.openCollection(
                collectionPath: directory.appending(path: "collection.anki2")
                    .path(percentEncoded: false),
                mediaFolderPath: media.path(percentEncoded: false),
                mediaDatabasePath: directory.appending(path: "collection.media.db")
                    .path(percentEncoded: false)))
    }

    /// The collection's decks by name, filtered decks included, in the engine's order (R7).
    func deckNames() throws -> [String] {
        try Responses.deckNames(call(EngineCall.deckNames, Requests.deckNames(includeFiltered: true)))
            .map(\.name)
    }

    /// The collection's decks with their ids, filtered decks included, in the engine's order.
    func decks() throws -> [Deck] {
        []
    }

    /// Signs in through the engine and returns the host key its `SyncAuth` carries (R8). The
    /// password goes into the request and nowhere else.
    func login(user: String, password: String, endpoint: String) throws -> String {
        try Responses.syncAuth(
            call(
                EngineCall.syncLogin,
                Requests.syncLogin(username: user, password: password, endpoint: endpoint)))
    }

    /// The engine, started from an empty init message by the first call that needs it.
    private func started() throws -> Engine {
        if let engine {
            return engine
        }
        let engine = try Engine(message: Data())
        self.engine = engine
        return engine
    }

    /// One allowed call through the adapter: the request's bytes in, the response's out. The
    /// engine's own refusal becomes its message, the sentence the app shows (R7, R8).
    private func call(
        _ pair: (service: UInt32, method: UInt32), _ request: [UInt8]
    ) throws -> [UInt8] {
        let engine = try started()
        do {
            return [UInt8](
                try engine.run(service: pair.service, method: pair.method, input: Data(request)))
        } catch EngineRefusal.Engine(let error) {
            throw Refusal(sentence: try Responses.engineMessage([UInt8](error)).message)
        }
    }
}
