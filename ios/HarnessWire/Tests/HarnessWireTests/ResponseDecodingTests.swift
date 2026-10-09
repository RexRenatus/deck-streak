// The codec's response decoders against bytes spelled by hand (SPEC-339 A3). Every input byte below
// is written from the wire format and the engine's messages at the pinned rev, never produced by
// the codec under test. Each response also carries fields the harness never reads, of each wire
// type the engine's messages use (a varint, a length-delimited field, a 32-bit float and a 64-bit
// value), so the decoders must skip what they do not know.
import HarnessWire
import XCTest

final class ResponseDecodingTests: XCTestCase {
    func test_a3_each_response_decodes_from_its_literal_bytes() throws {
        // DeckNames { entries (1): DeckNameId { id (1), name (2) } }, two entries in the engine's
        // order: `Default` (id 1), then `Synthetic` (id 300, `ac 02`).
        let deckNames = bytes(
            [0x0a, 0x0b], [0x08, 0x01], [0x12, 0x07, 0x44, 0x65, 0x66, 0x61, 0x75, 0x6c, 0x74],
            [0x0a, 0x0e], [0x08, 0xac, 0x02],
            [0x12, 0x09, 0x53, 0x79, 0x6e, 0x74, 0x68, 0x65, 0x74, 0x69, 0x63])
        XCTAssertEqual(
            try Responses.deckNames(deckNames),
            [DeckName(id: 1, name: "Default"), DeckName(id: 300, name: "Synthetic")],
            "DeckNames")

        // QueuedCards { cards (1), new_count (2), learning_count (3), review_count (4) }, with one
        // head card in the new queue: QueuedCard { card (1), queue (2) NEW = 0 and so omitted,
        // states (3), context (4) }. The card is Card { id (1) 300, note_id (2) 200 `c8 01`,
        // deck_id (3) 1, mtime_secs (5) 5, desired_retention (21) the float 0.9 `66 66 66 3f`
        // under key (21 << 3) | 5 = 173 `ad 01` }. The states are SchedulingStates { current (1),
        // again (2), hard (3), good (4), easy (5) }, each an opaque state the answer sends back;
        // the context is SchedulingContext { deck_name (1) `Default`, seed (2) 7 }. The response
        // ends with a 64-bit field 9 (key (9 << 3) | 1 = `49`) no engine message the harness
        // reads holds. Counts: new 1, learning and review 0 and so omitted.
        let newQueue = bytes(
            [0x0a, 0x30],
            [0x0a, 0x10], [0x08, 0xac, 0x02], [0x10, 0xc8, 0x01], [0x18, 0x01], [0x28, 0x05],
            [0xad, 0x01, 0x66, 0x66, 0x66, 0x3f],
            [0x1a, 0x0f], [0x0a, 0x01, 0x01], [0x12, 0x01, 0x02], [0x1a, 0x01, 0x03],
            [0x22, 0x01, 0x04], [0x2a, 0x01, 0x05],
            [0x22, 0x0b], [0x0a, 0x07, 0x44, 0x65, 0x66, 0x61, 0x75, 0x6c, 0x74], [0x10, 0x07],
            [0x10, 0x01],
            [0x49, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])
        XCTAssertEqual(
            try Responses.queue(newQueue),
            Queue(
                cards: [
                    QueuedCard(
                        cardID: 300, noteID: 200, queue: 0, currentState: [0x01],
                        againState: [0x02], hardState: [0x03], goodState: [0x04],
                        easyState: [0x05])
                ],
                newCount: 1, learningCount: 0, reviewCount: 0),
            "QueuedCards, a new card")

        // The same message with every count and the queue set: a learning card (queue 1) whose
        // card is { id 301 `ad 02`, note_id 201 `c9 01` }, its states only current and good, and
        // counts new 0 (omitted), learning 2, review 3.
        let learningQueue = bytes(
            [0x0a, 0x12],
            [0x0a, 0x06], [0x08, 0xad, 0x02], [0x10, 0xc9, 0x01],
            [0x10, 0x01],
            [0x1a, 0x06], [0x0a, 0x01, 0x11], [0x22, 0x01, 0x14],
            [0x18, 0x02], [0x20, 0x03])
        XCTAssertEqual(
            try Responses.queue(learningQueue),
            Queue(
                cards: [
                    QueuedCard(
                        cardID: 301, noteID: 201, queue: 1, currentState: [0x11], goodState: [0x14])
                ],
                newCount: 0, learningCount: 2, reviewCount: 3),
            "QueuedCards, a learning card")

        // RenderCardResponse { question_nodes (1), answer_nodes (2), css (3) }, each node a
        // RenderedTemplateNode with one of text (1) or replacement (2). The question holds a text
        // node, `synthetic front`, then a replacement, RenderedTemplateReplacement { field_name
        // (1) `Front`, current_text (2) `x`, filters (3) [`text`] }, which must decode as a
        // replacement and never as text. The answer's node and the css are not the question's.
        let rendered = bytes(
            [0x0a, 0x11], [0x0a, 0x0f],
            [0x73, 0x79, 0x6e, 0x74, 0x68, 0x65, 0x74, 0x69, 0x63, 0x20, 0x66, 0x72, 0x6f, 0x6e, 0x74],
            [0x0a, 0x12], [0x12, 0x10], [0x0a, 0x05, 0x46, 0x72, 0x6f, 0x6e, 0x74], [0x12, 0x01, 0x78],
            [0x1a, 0x04, 0x74, 0x65, 0x78, 0x74],
            [0x12, 0x06], [0x0a, 0x04, 0x62, 0x61, 0x63, 0x6b],
            [0x1a, 0x03, 0x78, 0x7b, 0x7d])
        XCTAssertEqual(
            try Responses.questionNodes(rendered),
            [.text("synthetic front"), .replacement(fieldName: "Front")],
            "RenderCardResponse's question")
    }

    func test_a7_the_host_key_and_the_engines_message_decode() throws {
        // SyncAuth { hkey (1), endpoint (2), io_timeout_secs (3) }, its host key last: the
        // endpoint `e`, a timeout of 30 (`1e`), and a 64-bit field 9 no engine message the app
        // reads holds (key (9 << 3) | 1 = `49`) come first, so the host key `k1` is read past all
        // three.
        let auth = bytes(
            [0x12, 0x01, 0x65], [0x18, 0x1e],
            [0x49, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            [0x0a, 0x02, 0x6b, 0x31])
        XCTAssertEqual(try Responses.syncAuth(auth), "k1", "SyncAuth's host key")

        // BackendError { message (1), kind (2), help_page (3), context (4) }: the message
        // `denied`, the kind SYNC_AUTH_ERROR = 7, a help page 5 and the context `ctx`.
        let refused = bytes(
            [0x0a, 0x06, 0x64, 0x65, 0x6e, 0x69, 0x65, 0x64], [0x10, 0x07], [0x18, 0x05],
            [0x22, 0x03, 0x63, 0x74, 0x78])
        XCTAssertEqual(
            try Responses.engineMessage(refused), EngineMessage(message: "denied", kind: 7),
            "BackendError")

        // The context first and the kind omitted: the message is still read past the context,
        // and the kind is proto3's default, 0.
        let unkinded = bytes([0x22, 0x03, 0x63, 0x74, 0x78], [0x0a, 0x02, 0x6e, 0x6f])
        XCTAssertEqual(
            try Responses.engineMessage(unkinded), EngineMessage(message: "no", kind: 0),
            "BackendError, its kind omitted")
    }

    func test_a23_the_intervals_and_the_five_states_decode() throws {
        // StringList { vals (1) }: the four intervals (13,24) gives a new card on the default
        // preset (SPEC-348 A1), `<1m`, `<6m`, `<10m` and `4d`, in the ratings' order, with a varint
        // field 2 no StringList holds between them, which is skipped.
        let intervals = bytes(
            [0x0a, 0x03, 0x3c, 0x31, 0x6d], [0x0a, 0x03, 0x3c, 0x36, 0x6d], [0x10, 0x01],
            [0x0a, 0x04, 0x3c, 0x31, 0x30, 0x6d], [0x0a, 0x02, 0x34, 0x64])
        XCTAssertEqual(
            try Responses.stringList(intervals), ["<1m", "<6m", "<10m", "4d"], "StringList")
        XCTAssertEqual(try Responses.stringList([]), [], "StringList, empty")
        XCTAssertThrowsError(try Responses.stringList(bytes([0x0a, 0x01, 0xff])), "not UTF-8") {
            XCTAssertEqual($0 as? WireError, .invalidUTF8)
        }

        // QueuedCards with one new card, Card { id (1) 7 }, whose SchedulingStates carry all five
        // states, each one byte naming its rating: current `21`, again `22`, hard `23`, good `24`
        // and easy `25`. The card is 4 + 2 + 15 = 21 (`15`) bytes; new_count (2) is 1.
        let queue = bytes(
            [0x0a, 0x15], [0x0a, 0x02, 0x08, 0x07],
            [0x1a, 0x0f], [0x0a, 0x01, 0x21], [0x12, 0x01, 0x22], [0x1a, 0x01, 0x23],
            [0x22, 0x01, 0x24], [0x2a, 0x01, 0x25],
            [0x10, 0x01])
        let decoded = try Responses.queue(queue)
        XCTAssertEqual(
            decoded,
            Queue(
                cards: [
                    QueuedCard(
                        cardID: 7, noteID: 0, queue: 0, currentState: [0x21], againState: [0x22],
                        hardState: [0x23], goodState: [0x24], easyState: [0x25])
                ],
                newCount: 1, learningCount: 0, reviewCount: 0),
            "QueuedCards, five states")
        let card = try XCTUnwrap(decoded.cards.first)
        XCTAssertEqual(
            [card.state(for: .again), card.state(for: .good)], [[0x22], [0x24]],
            "each grade's own state")
    }

    func test_a40_the_queued_card_carries_its_flag() throws {
        // SPEC-358 A40: QueuedCards with two cards. The first is Card { id (1) 7, flags (17) 1 }:
        // field 17's varint tag is 17 << 3 = 136, written `88 01`, and red is the engine's flag 1,
        // so the card is 2 + 3 = 5 bytes and its QueuedCard 2 + 5 = 7. The second is Card { id (1)
        // 8 } with no flag, 2 bytes, its QueuedCard 4.
        let queue = bytes(
            [0x0a, 0x07], [0x0a, 0x05], [0x08, 0x07], [0x88, 0x01, 0x01],
            [0x0a, 0x04], [0x0a, 0x02], [0x08, 0x08])
        let cards = try Responses.queue(queue).cards
        XCTAssertEqual(
            cards.map(\.flag), [1, 0],
            "A40: the red card carries Card.flags (17) as 1, and the card with none carries 0")
        XCTAssertEqual(cards.map(\.cardID), [7, 8], "each card's id decodes beside its flag")
    }
}
