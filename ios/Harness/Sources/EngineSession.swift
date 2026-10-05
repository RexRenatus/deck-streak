import DeckStreakFFI
import Foundation
import HarnessWire

/// The engine calls the harness makes, by the pairs the adapter's allow-list holds (SPEC-336,
/// SPEC-339 R10). The adapter refuses any other pair before the engine sees it.
enum EngineCall {
    static let openCollection: (service: UInt32, method: UInt32) = (3, 0)
    static let deckNames: (service: UInt32, method: UInt32) = (7, 13)
    static let queuedCards: (service: UInt32, method: UInt32) = (13, 3)
    static let answerCard: (service: UInt32, method: UInt32) = (13, 4)
    static let renderExistingCard: (service: UInt32, method: UInt32) = (27, 6)
}

/// Why the harness could not do what it was asked, as the sentence it shows (SPEC-339 R8).
struct Refusal: Error, Equatable {
    let sentence: String

    /// The sentence for any failure on the engine's path.
    static func sentence(for error: any Error) -> String {
        switch error {
        case let refusal as Refusal:
            return refusal.sentence
        case let refusal as EngineRefusal:
            switch refusal {
            case .NotAllowed(let service, let method):
                return "The adapter refused service \(service) method \(method): it is not on the allow-list."
            case .Engine(let error):
                return "The engine refused the call with a \(error.count)-byte error."
            case .Start(let reason):
                return "The engine could not start: \(reason)"
            }
        case let wire as WireError:
            return "The engine's answer could not be read: \(wire)."
        default:
            return "The harness could not prepare the collection: \(error.localizedDescription)"
        }
    }
}

/// What a study step shows: the queue's head card, if any, its question's nodes and the deck's
/// new count.
struct Studied: Sendable, Equatable {
    var card: QueuedCard?
    var question: [RenderedNode]
    var newCount: UInt32
}

/// The one owner of the engine: every engine call runs here, off the main actor, one at a time
/// (SPEC-339 R3, R8). The actor serialises the calls; the engine itself is `Sendable` because the
/// adapter's Rust side is `Send + Sync`.
actor EngineSession {
    private var engine: Engine?
    private var head: QueuedCard?
    private var shownAt: ContinuousClock.Instant?

    /// Copies the bundled synthetic collection into a fresh directory under Application Support,
    /// beside an empty media folder, starts one engine from an empty init message and opens the
    /// copy (R3). The bundled file is never opened, so every launch starts from the same card.
    func open() throws {
        guard
            let bundled = Bundle.main.url(forResource: "collection", withExtension: "anki2")
        else {
            throw Refusal(sentence: "The app holds no synthetic collection.")
        }
        let files = FileManager.default
        let support = try files.url(
            for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
        let directory = support.appendingPathComponent(UUID().uuidString, isDirectory: true)
        let media = directory.appendingPathComponent("collection.media", isDirectory: true)
        try files.createDirectory(at: media, withIntermediateDirectories: true)
        let collection = directory.appendingPathComponent("collection.anki2")
        try files.copyItem(at: bundled, to: collection)
        let engine = try Engine(message: Data())
        _ = try engine.run(
            service: EngineCall.openCollection.service, method: EngineCall.openCollection.method,
            input: Data(
                Requests.openCollection(
                    collectionPath: collection.path(percentEncoded: false),
                    mediaFolderPath: media.path(percentEncoded: false),
                    mediaDatabasePath: directory.appendingPathComponent("collection.media.db")
                        .path(percentEncoded: false))))
        self.engine = engine
    }

    /// The collection's decks by name, filtered decks included, in the engine's order (R4).
    func deckNames() throws -> [String] {
        try Responses.deckNames(call(EngineCall.deckNames, Requests.deckNames(includeFiltered: true)))
            .map(\.name)
    }

    /// The current deck's next card and its rendered question (R5).
    func next() throws -> Studied {
        let queue = try Responses.queue(
            call(EngineCall.queuedCards, Requests.queuedCards(fetchLimit: 1)))
        head = queue.cards.first
        guard let card = head else {
            return Studied(card: nil, question: [], newCount: queue.newCount)
        }
        let question = try Responses.questionNodes(
            call(EngineCall.renderExistingCard, Requests.renderExistingCard(cardID: card.cardID)))
        shownAt = .now
        return Studied(card: card, question: question, newCount: queue.newCount)
    }

    /// Answers the shown card Good with the states the queue gave it, then reads the queue again
    /// (R7).
    func answerGood() throws -> Studied {
        guard let card = head else {
            throw Refusal(sentence: "There is no card to answer.")
        }
        let taken = shownAt.map { $0.duration(to: .now) } ?? .zero
        let answer = CardAnswer(
            cardID: card.cardID, currentState: card.currentState, newState: card.goodState,
            rating: .good, answeredAtMillis: Int64(Date().timeIntervalSince1970 * 1000),
            millisecondsTaken: UInt32(clamping: taken.components.seconds * 1000
                + taken.components.attoseconds / 1_000_000_000_000_000))
        _ = try call(EngineCall.answerCard, Requests.answerCard(answer))
        return try next()
    }

    /// One allowed call through the adapter: the request's bytes in, the response's out.
    private func call(
        _ pair: (service: UInt32, method: UInt32), _ request: [UInt8]
    ) throws -> [UInt8] {
        guard let engine else {
            throw Refusal(sentence: "The collection is not open.")
        }
        return [UInt8](try engine.run(service: pair.service, method: pair.method, input: Data(request)))
    }
}
