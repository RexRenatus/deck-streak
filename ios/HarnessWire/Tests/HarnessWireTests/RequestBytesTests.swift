// The codec's request encoders against bytes spelled by hand (SPEC-339 A2). Every expected byte
// below is written from the wire format, never produced by the codec under test: a field's key is
// its number shifted left three bits with its wire type in the low three (0 for a varint, 2 for a
// length-delimited field), and a varint carries seven bits a byte, low group first, with the high
// bit set on every byte but the last. Each request matches what the adapter's Rust tests send for
// the same input (`crates/ffi/tests/support/mod.rs`, `round_trip.rs`, `render.rs`).
import HarnessWire
import XCTest

/// Joins byte runs written one field per run, so each run reads as one field.
func bytes(_ runs: [UInt8]...) -> [UInt8] {
    runs.flatMap { $0 }
}

final class RequestBytesTests: XCTestCase {
    func test_a2_each_request_encodes_to_its_literal_bytes() {
        // OpenCollectionRequest: collection_path (1), media_folder_path (2), media_db_path (3).
        // The 200-byte path needs a two-byte length, 200 = 0x48 + 1 * 128: `c8 01`.
        let longPath = String(repeating: "x", count: 200)
        XCTAssertEqual(
            Requests.openCollection(
                collectionPath: longPath, mediaFolderPath: "/c/m", mediaDatabasePath: "/c/m.db"),
            bytes(
                [0x0a, 0xc8, 0x01], [UInt8](repeating: 0x78, count: 200),
                [0x12, 0x04, 0x2f, 0x63, 0x2f, 0x6d],
                [0x1a, 0x07, 0x2f, 0x63, 0x2f, 0x6d, 0x2e, 0x64, 0x62]),
            "OpenCollectionRequest")

        // GetDeckNamesRequest { skip_empty_default: false, include_filtered: true }: the false
        // field 1 is omitted, field 2 is key `10` and value `01`.
        XCTAssertEqual(Requests.deckNames(includeFiltered: true), [0x10, 0x01], "GetDeckNamesRequest")

        // GetQueuedCardsRequest { fetch_limit: 1 }.
        XCTAssertEqual(Requests.queuedCards(fetchLimit: 1), [0x08, 0x01], "GetQueuedCardsRequest")

        // RenderExistingCardRequest { card_id: 300 }, browser and partial_render false and so
        // omitted. 300 = 0x2c + 2 * 128: `ac 02`, the continuation bit set on the first byte.
        XCTAssertEqual(
            Requests.renderExistingCard(cardID: 300), [0x08, 0xac, 0x02],
            "RenderExistingCardRequest")

        // CardAnswer: card_id (1), current_state (2) and new_state (3) as the queue gave them,
        // rating (4) Good = 2, answered_at_millis (5) and milliseconds_taken (6).
        // 1_700_000_000_000 = 0x00 + 0x50 * 2^7 + 0x15 * 2^14 + 0x7f * 2^21 + 0x3c * 2^28
        // + 0x31 * 2^35: `80 d0 95 ff bc 31`. 4000 = 0x20 + 31 * 128: `a0 1f`.
        let answer = CardAnswer(
            cardID: 300, currentState: [0x0a, 0x00], newState: [0x0a, 0x02, 0x08, 0x01],
            rating: .good, answeredAtMillis: 1_700_000_000_000, millisecondsTaken: 4000)
        XCTAssertEqual(
            Requests.answerCard(answer),
            bytes(
                [0x08, 0xac, 0x02],
                [0x12, 0x02, 0x0a, 0x00],
                [0x1a, 0x04, 0x0a, 0x02, 0x08, 0x01],
                [0x20, 0x02],
                [0x28, 0x80, 0xd0, 0x95, 0xff, 0xbc, 0x31],
                [0x30, 0xa0, 0x1f]),
            "CardAnswer")

        // Empty, the undo call's request: no fields, no bytes.
        XCTAssertEqual(Requests.undo(), [], "Empty")
    }

    func test_a6_the_login_request_writes_user_password_and_endpoint() {
        // SyncLoginRequest: username (1), password (2) and endpoint (3), each a length-delimited
        // string, so their keys are `0a`, `12` and `1a`. `user` is 4 bytes, `pw` 2, and
        // `https://s.invalid/` 18, `12`.
        XCTAssertEqual(
            Requests.syncLogin(username: "user", password: "pw", endpoint: "https://s.invalid/"),
            bytes(
                [0x0a, 0x04, 0x75, 0x73, 0x65, 0x72],
                [0x12, 0x02, 0x70, 0x77],
                [0x1a, 0x12, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x73, 0x2e, 0x69, 0x6e,
                 0x76, 0x61, 0x6c, 0x69, 0x64, 0x2f]),
            "SyncLoginRequest")

        // An empty endpoint is still written, as its key and a zero length, so the engine reads
        // the field the app sent rather than a default it chose.
        XCTAssertEqual(
            Requests.syncLogin(username: "user", password: "pw", endpoint: ""),
            bytes([0x0a, 0x04, 0x75, 0x73, 0x65, 0x72], [0x12, 0x02, 0x70, 0x77], [0x1a, 0x00]),
            "SyncLoginRequest, an empty endpoint")
    }
}
