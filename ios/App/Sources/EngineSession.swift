import DeckStreakFFI
import Foundation
import HarnessWire

/// The engine calls the app makes, by the pairs the adapter's allow-list holds (SPEC-336,
/// SPEC-347 R1, R7, R8, SPEC-348 R1). The adapter refuses any other pair before the engine sees
/// it. An answer is not one of them: it goes through the adapter's answer door (SPEC-365 R12).
enum EngineCall {
    static let openCollection: (service: UInt32, method: UInt32) = (3, 0)
    static let deckNames: (service: UInt32, method: UInt32) = (7, 13)
    static let setCurrentDeck: (service: UInt32, method: UInt32) = (7, 22)
    static let queuedCards: (service: UInt32, method: UInt32) = (13, 3)
    static let describeNextStates: (service: UInt32, method: UInt32) = (13, 24)
    static let syncLogin: (service: UInt32, method: UInt32) = (1, 3)
}

/// Why the app could not do what it was asked, as the sentence it shows: for a refusal of the
/// engine's, the engine's own message (R7, R8).
struct Refusal: Error, Equatable, CustomStringConvertible {
    let sentence: String

    /// The sentence, so a refusal reads as itself wherever an error is described.
    var description: String { sentence }
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
/// adapter's Rust side is `Send + Sync`. The voice choices live here too, beside the collection
/// and never inside it, because the engine's types stop at this file (SPEC-348 R6, R14).
actor EngineSession {
    /// The engine's red flag, read once from the adapter, so the app holds no copy of the
    /// core's rule (SPEC-358 R3, ADR-369 O4).
    nonisolated let red = redFlag()
    private var engine: Engine?
    private var choices = VoiceChoices.open(path: "")

    /// The adapter's voice qualities, in the order the app numbers them: 0 default, 1 enhanced,
    /// 2 premium.
    private static let qualities: [VoiceQuality] = [.default, .enhanced, .premium]

    /// Opens the collection in its directory, beside its media folder, creating the folder when it
    /// is not there yet (R7). The directory is the one fixed path under Application Support unless
    /// the launch names another with `-DSCollectionDirectory`, which the adapter judges (SPEC-348
    /// R7); its refusal reads as its name. The engine creates a collection that is not there yet,
    /// so a fresh install opens one holding the engine's default deck alone.
    func open(arguments: [String] = ProcessInfo.processInfo.arguments) throws {
        let fallback = URL.applicationSupportDirectory.appending(
            path: "DeckStreak", directoryHint: .isDirectory)
        let path = try collectionDirectory(
            fallback: fallback.path(percentEncoded: false), arguments: arguments)
        let directory = URL(filePath: path, directoryHint: .isDirectory)
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
        choices = VoiceChoices.open(
            path: directory.appending(path: "voice-choices.tsv").path(percentEncoded: false))
    }

    /// The collection's decks with their ids, filtered decks included, in the engine's order (R7).
    func decks() throws -> [Deck] {
        try Responses.deckNames(
            call(EngineCall.deckNames, Requests.deckNames(includeFiltered: true))
        )
        .map { Deck(id: $0.id, name: $0.name) }
    }

    /// Makes `deck` the current deck (7,22), so the queue is its own (SPEC-348 R9).
    func setCurrentDeck(_ deck: Deck) throws {
        _ = try call(EngineCall.setCurrentDeck, Requests.setCurrentDeck(deck.id))
    }

    /// The queue's head card and its three counts (13,3), fetched one card at a time (R10).
    func queue() throws -> Queue {
        try Responses.queue(call(EngineCall.queuedCards, Requests.queuedCards(fetchLimit: 1)))
    }

    /// The four ratings' intervals for `card`, in the ratings' order (13,24) (R11).
    func intervals(_ card: QueuedCard) throws -> [String] {
        try Responses.stringList(
            call(EngineCall.describeNextStates, Requests.describeNextStates(card)))
    }

    /// Records the press of `rating` on `card` through the adapter's answer door (R10, SPEC-365
    /// R12): the press names its card and its grade and hands over the states the card was shown
    /// with, and the adapter picks the grade's state from them, so the app sends no next state of
    /// its own.
    func answer(_ card: QueuedCard, rating: Rating, millisecondsTaken: UInt32) throws {
        let engine = try started()
        _ = try refused {
            try engine.answer(
                card: card.cardID, grade: Self.grades[Int(rating.rawValue) / 2],
                states: Data(Requests.describeNextStates(card)),
                millisecondsTaken: millisecondsTaken)
        }
    }

    /// Buries `card` as the user's bury of one card, through the adapter (SPEC-358 R3).
    func bury(_ card: QueuedCard) throws {
        let engine = try started()
        try refused { try engine.bury(cardId: card.cardID) }
    }

    /// Toggles red on `card` from the flag it carries, through the adapter, and answers the
    /// card's new flag (SPEC-358 R3): the app builds no request and computes no flag.
    func flag(_ card: QueuedCard) throws -> UInt32 {
        let engine = try started()
        return try refused { try engine.flag(cardId: card.cardID, flag: card.flag) }
    }

    /// The face of `cardID`: its question, or its answer when `answer` is set, with the clips to
    /// play by themselves when `autoplay` is wished, each line of speech in the voice chosen for
    /// its language when that voice is installed, else in the installed voice its tag asks for
    /// (SPEC-348 R5, R13; SPEC-393 R9). It asks for the day face: the screen sets no night classes
    /// in this part.
    func face(
        _ cardID: Int64, answer: Bool, autoplay: Bool, installed: [InstalledVoice]
    ) throws -> ReviewFace {
        let engine = try started()
        let face = try refused {
            try engine.face(cardId: cardID, answer: answer, night: false, autoplay: autoplay)
        }
        let voices = installed.map(Self.voice)
        let replay = face.replay.map { clip($0, voices) }
        return ReviewFace(
            document: face.document,
            autoplay: face.autoplay.map { clip($0, voices).clip },
            replay: replay.map { $0.clip },
            omitted: face.omitted,
            languages: NSOrderedSet(array: replay.compactMap { $0.language }).array
                .compactMap { $0 as? String })
    }

    /// For each language, the installed voices offered for it and the one chosen (SPEC-348 R14).
    func voices(_ languages: [String], installed: [InstalledVoice]) -> [VoiceLanguage] {
        let voices = installed.map(Self.voice)
        let byIdentifier = Dictionary(
            installed.map { ($0.identifier, $0) }, uniquingKeysWith: { first, _ in first })
        return languages.map { language in
            VoiceLanguage(
                language: language,
                options: choices.options(language: language, installed: voices)
                    .compactMap { byIdentifier[$0.identifier] },
                chosen: choices.chosen(language: language, installed: voices))
        }
    }

    /// Records `identifier` as the voice for `language`, or clears the choice given nil (R14).
    func choose(voice identifier: String?, language: String) throws {
        try choices.choose(language: language, identifier: identifier)
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

    /// One allowed call through the adapter: the request's bytes in, the response's out.
    private func call(
        _ pair: (service: UInt32, method: UInt32), _ request: [UInt8]
    ) throws -> [UInt8] {
        let engine = try started()
        return try refused {
            [UInt8](
                try engine.run(service: pair.service, method: pair.method, input: Data(request)))
        }
    }

    /// The engine's own refusal becomes its message, the sentence the app shows (R7, R8), whether
    /// it refused a call or a press's answer (SPEC-365 R12).
    private func refused<Value>(_ work: () throws -> Value) throws -> Value {
        do {
            return try work()
        } catch EngineRefusal.Engine(let error), PressRefusal.Engine(let error) {
            throw Refusal(sentence: try Responses.engineMessage([UInt8](error)).message)
        }
    }

    /// A clip as the player plays it, and the language it speaks, if it speaks.
    private func clip(_ clip: Clip, _ voices: [Voice]) -> (clip: ReviewClip, language: String?) {
        switch clip {
        case .sound(let name, let bytes):
            return (.sound(name: name, bytes: bytes), nil)
        case .speech(let text, let language, let rate, let requested):
            return (
                .speech(
                    text: text, language: language, rate: rate,
                    voice: choices.voiceFor(
                        language: language, requested: requested, installed: voices)),
                language
            )
        }
    }

    /// The adapter's grades for the app's two ratings, which number 0 and 2 as the engine's Again
    /// and Good do: half a rating's number indexes this list, so a press picks its grade with no
    /// branch of the app's own (SPEC-365 R12).
    private static let grades: [PressedGrade] = [.again, .good]

    /// An installed voice in the adapter's terms.
    private static func voice(_ voice: InstalledVoice) -> Voice {
        Voice(
            identifier: voice.identifier, name: voice.name, language: voice.language,
            quality: qualities[min(max(voice.quality, 0), 2)])
    }
}
