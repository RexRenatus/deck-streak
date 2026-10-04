// The six requests the harness sends and the three responses it reads (SPEC-339 R9), field by
// field as the engine's messages number them at the pinned rev. An encoder writes exactly the
// fields its request carries; a decoder reads exactly the fields the harness shows and skips the
// rest whole.

/// A card's rating, as the engine's `CardAnswer.Rating` numbers it.
public enum Rating: Int32, Sendable {
    case again = 0
    case hard = 1
    case good = 2
    case easy = 3
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

/// One card of `QueuedCards`, with the two states an answer of Good sends back.
public struct QueuedCard: Equatable, Sendable {
    public var cardID: Int64
    public var noteID: Int64
    public var queue: Int32
    public var currentState: [UInt8]
    public var goodState: [UInt8]

    public init(cardID: Int64, noteID: Int64, queue: Int32, currentState: [UInt8], goodState: [UInt8]) {
        self.cardID = cardID
        self.noteID = noteID
        self.queue = queue
        self.currentState = currentState
        self.goodState = goodState
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

    /// `QueuedCards`: `cards` (1), `new_count` (2), `learning_count` (3), `review_count` (4).
    public static func queue(_ bytes: [UInt8]) throws -> Queue {
        let queue = try WireMessage(bytes)
        return Queue(
            cards: try queue.repeated(1).map(queuedCard),
            newCount: UInt32(truncatingIfNeeded: queue.varint(2)),
            learningCount: UInt32(truncatingIfNeeded: queue.varint(3)),
            reviewCount: UInt32(truncatingIfNeeded: queue.varint(4)))
    }

    /// `QueuedCard`: `card` (1), a `Card` whose `id` (1) and `note_id` (2) the harness reads;
    /// `queue` (2); and `states` (3), a `SchedulingStates` whose `current` (1) and `good` (4) an
    /// answer of Good sends back as they came.
    static func queuedCard(_ bytes: [UInt8]) throws -> QueuedCard {
        let queued = try WireMessage(bytes)
        let card = try WireMessage(queued.lengthDelimited(1) ?? [])
        let states = try WireMessage(queued.lengthDelimited(3) ?? [])
        return QueuedCard(
            cardID: Int64(bitPattern: card.varint(1)),
            noteID: Int64(bitPattern: card.varint(2)),
            queue: Int32(truncatingIfNeeded: queued.varint(2)),
            currentState: states.lengthDelimited(1) ?? [],
            goodState: states.lengthDelimited(4) ?? [])
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
