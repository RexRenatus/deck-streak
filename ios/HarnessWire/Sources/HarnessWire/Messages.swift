// The six requests the harness sends and the three responses it reads (SPEC-339 R9). A stub: every
// encoder returns no bytes and every decoder returns an empty value, until the codec is built.

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
        []
    }

    /// `GetDeckNamesRequest`.
    public static func deckNames(includeFiltered: Bool) -> [UInt8] {
        []
    }

    /// `GetQueuedCardsRequest`.
    public static func queuedCards(fetchLimit: UInt32) -> [UInt8] {
        []
    }

    /// `RenderExistingCardRequest`, with `browser` and `partial_render` false.
    public static func renderExistingCard(cardID: Int64) -> [UInt8] {
        []
    }

    /// `CardAnswer`.
    public static func answerCard(_ answer: CardAnswer) -> [UInt8] {
        []
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
    /// `DeckNames`.
    public static func deckNames(_ bytes: [UInt8]) throws -> [DeckName] {
        []
    }

    /// `QueuedCards`.
    public static func queue(_ bytes: [UInt8]) throws -> Queue {
        Queue(cards: [], newCount: 0, learningCount: 0, reviewCount: 0)
    }

    /// The question's nodes of a `RenderCardResponse`.
    public static func questionNodes(_ bytes: [UInt8]) throws -> [RenderedNode] {
        []
    }
}
