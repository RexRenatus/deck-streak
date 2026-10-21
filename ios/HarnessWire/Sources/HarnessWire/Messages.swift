// The nine requests the harness and the app send and the six responses they read (SPEC-339 R9,
// SPEC-347 R10, SPEC-348 R19), field by field as the engine's messages number them at the pinned
// rev. An encoder writes exactly the fields its request carries; a decoder reads exactly the
// fields its caller shows and skips the rest whole.

/// A card's rating, as the engine's `CardAnswer.Rating` numbers it: Again and Good alone, the two
/// grades a press records (SPEC-365 R13).
public enum Rating: Int32, Sendable {
    case again = 0
    case good = 2
}

/// `CardAnswer`: the scheduling states travel as the encoded messages the queue gave.
public struct CardAnswer: Equatable, Sendable {
    public var cardID: Int64
    public var currentState: [UInt8]
    public var newState: [UInt8]
    public var rating: Rating
    public var answeredAtMillis: Int64
    public var millisecondsTaken: UInt32

    public init(
        cardID: Int64, currentState: [UInt8], newState: [UInt8], rating: Rating,
        answeredAtMillis: Int64, millisecondsTaken: UInt32
    ) {
        self.cardID = cardID
        self.currentState = currentState
        self.newState = newState
        self.rating = rating
        self.answeredAtMillis = answeredAtMillis
        self.millisecondsTaken = millisecondsTaken
    }
}

/// The request encoders, one per allow-listed call.
public enum Requests {
    /// `OpenCollectionRequest`: the collection's path, its media folder and its media database.
    public static func openCollection(
        collectionPath: String, mediaFolderPath: String, mediaDatabasePath: String
    ) -> [UInt8] {
        var writer = WireWriter()
        writer.stringField(1, collectionPath)
        writer.stringField(2, mediaFolderPath)
        writer.stringField(3, mediaDatabasePath)
        return writer.bytes
    }

    /// `GetDeckNamesRequest`: `skip_empty_default` (1) false and so omitted, `include_filtered` (2).
    public static func deckNames(includeFiltered: Bool) -> [UInt8] {
        var writer = WireWriter()
        writer.varintField(2, includeFiltered ? 1 : 0)
        return writer.bytes
    }

    /// `GetQueuedCardsRequest`: `fetch_limit` (1).
    public static func queuedCards(fetchLimit: UInt32) -> [UInt8] {
        var writer = WireWriter()
        writer.varintField(1, UInt64(fetchLimit))
        return writer.bytes
    }

    /// `DeckId`: `did` (1), the deck (7,22) makes the current one, so the queue is its own
    /// (SPEC-348 R9).
    public static func setCurrentDeck(_ deckID: Int64) -> [UInt8] {
        var writer = WireWriter()
        writer.int64Field(1, deckID)
        return writer.bytes
    }

    /// `SchedulingStates`: `current` (1), `again` (2), `hard` (3), `good` (4) and `easy` (5), each
    /// the opaque state the queue gave, for (13,24) to describe (SPEC-348 R11).
    public static func describeNextStates(_ card: QueuedCard) -> [UInt8] {
        var writer = WireWriter()
        writer.bytesField(1, card.currentState)
        writer.bytesField(2, card.againState)
        writer.bytesField(3, card.hardState)
        writer.bytesField(4, card.goodState)
        writer.bytesField(5, card.easyState)
        return writer.bytes
    }

    /// `RenderExistingCardRequest`: `card_id` (1), with `browser` (2) and `partial_render` (3)
    /// false and so omitted, so the engine renders the whole template.
    public static func renderExistingCard(cardID: Int64) -> [UInt8] {
        var writer = WireWriter()
        writer.int64Field(1, cardID)
        return writer.bytes
    }

    /// `CardAnswer`: `card_id` (1), `current_state` (2), `new_state` (3), `rating` (4),
    /// `answered_at_millis` (5) and `milliseconds_taken` (6).
    public static func answerCard(_ answer: CardAnswer) -> [UInt8] {
        var writer = WireWriter()
        writer.int64Field(1, answer.cardID)
        writer.bytesField(2, answer.currentState)
        writer.bytesField(3, answer.newState)
        writer.varintField(4, UInt64(answer.rating.rawValue))
        writer.int64Field(5, answer.answeredAtMillis)
        writer.varintField(6, UInt64(answer.millisecondsTaken))
        return writer.bytes
    }

    /// `Empty`, the undo call's request.
    public static func undo() -> [UInt8] {
        []
    }

    /// `SyncLoginRequest`: `username` (1), `password` (2) and `endpoint` (3), each written, an
    /// empty endpoint included, because the endpoint is never optional here (SPEC-347 R10).
    public static func syncLogin(username: String, password: String, endpoint: String) -> [UInt8] {
        var writer = WireWriter()
        writer.stringField(1, username)
        writer.stringField(2, password)
        writer.stringField(3, endpoint)
        return writer.bytes
    }
}

/// One entry of `DeckNames`.
public struct DeckName: Equatable, Sendable {
    public var id: Int64
    public var name: String

    public init(id: Int64, name: String) {
        self.id = id
        self.name = name
    }
}

/// One card of `QueuedCards`, with its current state and the four states an answer sends back,
/// one per rating. The three beside Good's default to empty, so the five-argument call the harness
/// makes still reads (SPEC-348 section 10). Its flag is the card's `Card.flags`, the engine's
/// number, red as 1; it defaults to none, 0 (SPEC-358 R3).
public struct QueuedCard: Equatable, Sendable {
    public var cardID: Int64
    public var noteID: Int64
    public var queue: Int32
    public var currentState: [UInt8]
    public var againState: [UInt8]
    public var hardState: [UInt8]
    public var goodState: [UInt8]
    public var easyState: [UInt8]
    public var flag: UInt32

    public init(
        cardID: Int64, noteID: Int64, queue: Int32, currentState: [UInt8],
        againState: [UInt8] = [], hardState: [UInt8] = [], goodState: [UInt8],
        easyState: [UInt8] = [], flag: UInt32 = 0
    ) {
        self.cardID = cardID
        self.noteID = noteID
        self.queue = queue
        self.currentState = currentState
        self.againState = againState
        self.hardState = hardState
        self.goodState = goodState
        self.easyState = easyState
        self.flag = flag
    }

    /// The state an answer of `rating` sends back as its new state: the rating's own (R10). The
    /// states sit in the ratings' own order, so the rating's number picks its state.
    public func state(for rating: Rating) -> [UInt8] {
        [againState, hardState, goodState, easyState][Int(rating.rawValue)]
    }
}

/// `QueuedCards`: the cards and the three counts.
public struct Queue: Equatable, Sendable {
    public var cards: [QueuedCard]
    public var newCount: UInt32
    public var learningCount: UInt32
    public var reviewCount: UInt32

    public init(cards: [QueuedCard], newCount: UInt32, learningCount: UInt32, reviewCount: UInt32) {
        self.cards = cards
        self.newCount = newCount
        self.learningCount = learningCount
        self.reviewCount = reviewCount
    }
}

/// One node of a rendered template: text, or a field replacement the engine left for the client.
public enum RenderedNode: Equatable, Sendable {
    case text(String)
    case replacement(fieldName: String)
}

/// The engine's refusal, read from a `BackendError`: its message, the sentence the app shows, and
/// its kind, as the engine numbers it.
public struct EngineMessage: Equatable, Sendable {
    public var message: String
    public var kind: Int32

    public init(message: String, kind: Int32) {
        self.message = message
        self.kind = kind
    }
}

/// The response decoders.
public enum Responses {
    /// `DeckNames`: `entries` (1), each a `DeckNameId` of `id` (1) and `name` (2), in the order
    /// the engine gives them.
    public static func deckNames(_ bytes: [UInt8]) throws -> [DeckName] {
        try WireMessage(bytes).repeated(1).map { entry in
            let deck = try WireMessage(entry)
            return DeckName(id: Int64(bitPattern: deck.varint(1)), name: try deck.string(2))
        }
    }

    /// `SyncAuth`: the host key, `hkey` (1). Its `endpoint` (2) and `io_timeout_secs` (3), and any
    /// field the app never reads, are skipped whole.
    public static func syncAuth(_ bytes: [UInt8]) throws -> String {
        try WireMessage(bytes).string(1)
    }

    /// `BackendError`: its `message` (1) and `kind` (2). Its `help_page` (3), `context` (4) and any
    /// other field are skipped whole.
    public static func engineMessage(_ bytes: [UInt8]) throws -> EngineMessage {
        let error = try WireMessage(bytes)
        return EngineMessage(
            message: try error.string(1), kind: Int32(truncatingIfNeeded: error.varint(2)))
    }

    /// `QueuedCards`: `cards` (1), `new_count` (2), `learning_count` (3), `review_count` (4).
    public static func queue(_ bytes: [UInt8]) throws -> Queue {
        let queue = try WireMessage(bytes)
        return Queue(
            cards: try queue.repeated(1).map(queuedCard),
            newCount: UInt32(truncatingIfNeeded: queue.varint(2)),
            learningCount: UInt32(truncatingIfNeeded: queue.varint(3)),
            reviewCount: UInt32(truncatingIfNeeded: queue.varint(4)))
    }

    /// `QueuedCard`: `card` (1), a `Card` whose `id` (1), `note_id` (2) and `flags` (17) the
    /// harness reads;
    /// `queue` (2); and `states` (3), a `SchedulingStates` whose `current` (1), `again` (2),
    /// `hard` (3), `good` (4) and `easy` (5) an answer sends back as they came.
    static func queuedCard(_ bytes: [UInt8]) throws -> QueuedCard {
        let queued = try WireMessage(bytes)
        let card = try WireMessage(queued.lengthDelimited(1) ?? [])
        let states = try WireMessage(queued.lengthDelimited(3) ?? [])
        return QueuedCard(
            cardID: Int64(bitPattern: card.varint(1)),
            noteID: Int64(bitPattern: card.varint(2)),
            queue: Int32(truncatingIfNeeded: queued.varint(2)),
            currentState: states.lengthDelimited(1) ?? [],
            againState: states.lengthDelimited(2) ?? [],
            hardState: states.lengthDelimited(3) ?? [],
            goodState: states.lengthDelimited(4) ?? [],
            easyState: states.lengthDelimited(5) ?? [],
            flag: UInt32(truncatingIfNeeded: card.varint(17)))
    }

    /// `StringList`: `vals` (1), each a UTF-8 string, in order: the intervals (13,24) gives, one
    /// per rating (SPEC-348 R11).
    /// Each value is read as the one string field of a message of its own, so it meets the
    /// codec's one UTF-8 check.
    public static func stringList(_ bytes: [UInt8]) throws -> [String] {
        try WireMessage(bytes).repeated(1).map { value in
            var field = WireWriter()
            field.bytesField(1, value)
            return try WireMessage(field.bytes).string(1)
        }
    }

    /// The question's nodes of a `RenderCardResponse`: `question_nodes` (1), never the answer's
    /// (2).
    public static func questionNodes(_ bytes: [UInt8]) throws -> [RenderedNode] {
        try WireMessage(bytes).repeated(1).compactMap(renderedNode)
    }

    /// A `RenderedTemplateNode`, a oneof of `text` (1) and `replacement` (2), a
    /// `RenderedTemplateReplacement` whose `field_name` (1) names the field the engine left for
    /// the client. A node holding neither is dropped.
    static func renderedNode(_ bytes: [UInt8]) throws -> RenderedNode? {
        let node = try WireMessage(bytes)
        if let replacement = node.lengthDelimited(2) {
            return .replacement(fieldName: try WireMessage(replacement).string(1))
        }
        if node.lengthDelimited(1) != nil {
            return .text(try node.string(1))
        }
        return nil
    }
}
