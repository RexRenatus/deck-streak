//! Round trips through the adapter's one entry point against a synthetic collection (SPEC-336 A1
//! to A6).
//!
//! Each test builds its own collection with the engine's own API before the adapter runs: one
//! Basic note in the default deck, and a second, empty deck. It then drives the adapter the way a
//! native client does, with each request encoded as protobuf bytes and each response decoded from
//! them. The pairs below are read from the engine's generated dispatch, not from the adapter's
//! table, so a wrong entry in the table fails here. The wire helpers encode and decode only what
//! these six tests send and read.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;
use anki::decks::DeckId;
use deck_streak_ffi::engine::{Engine, EngineRefusal};

/// `BackendCollectionService.OpenCollection`, as the engine's generated dispatch numbers it.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `BackendCollectionService.CloseCollection`: a real engine call the allow-list leaves out.
const CLOSE_COLLECTION: (u32, u32) = (3, 1);
/// `CollectionService.Undo`, reached through the backend collection service.
const UNDO: (u32, u32) = (3, 8);
/// `DecksService.GetDeckNames`.
const GET_DECK_NAMES: (u32, u32) = (7, 13);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// `SchedulerService.AnswerCard`.
const ANSWER_CARD: (u32, u32) = (13, 4);
/// The engine's rating for Good (`CardAnswer.Rating.GOOD`).
const GOOD: u64 = 2;
/// The engine's queue for a new card (`QueuedCards.Queue.NEW`).
const NEW_QUEUE: u64 = 0;
/// The second deck the synthetic collection holds, beside the engine's own default deck.
const SECOND_DECK: &str = "Synthetic";
/// The engine's name for the operation an answer records, which its undo reports.
const ANSWER_OPERATION: &str = "Answer Card";

/// A collection built for one test, closed, and ready for the adapter to open.
struct Synthetic {
    dir: PathBuf,
    collection: PathBuf,
    note_id: i64,
    card_id: i64,
}

/// Builds the synthetic collection in a directory of its own under the target's scratch space.
fn synthetic(test: &str) -> Synthetic {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-round-trip")
        .join(format!("{test}-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(dir.join("collection.media")).expect("a scratch directory");
    let collection = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&collection)
        .build()
        .expect("the engine creates the collection");
    col.get_or_create_normal_deck(SECOND_DECK)
        .expect("the engine creates the second deck");
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let mut note = basic.new_note();
    note.set_field(0, "synthetic front")
        .expect("the front is set");
    note.set_field(1, "synthetic back")
        .expect("the back is set");
    col.add_note(&mut note, DeckId(1))
        .expect("the engine adds the note to the default deck");
    let note_id = note.id.0;
    let card_id: i64 = col
        .storage
        .db()
        .query_row("select id from cards where nid = ?", [note_id], |row| {
            row.get(0)
        })
        .expect("the note has one card");
    col.close(None).expect("the engine closes the collection");
    Synthetic {
        dir,
        collection,
        note_id,
        card_id,
    }
}

fn engine() -> Arc<Engine> {
    Engine::new(Vec::new()).expect("the engine starts from the default init message")
}

fn call(
    engine: &Engine,
    (service, method): (u32, u32),
    input: Vec<u8>,
) -> Result<Vec<u8>, EngineRefusal> {
    engine.run(service, method, input)
}

fn text(path: &Path) -> &str {
    path.to_str().expect("a scratch path is UTF-8")
}

fn open_request(synthetic: &Synthetic) -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_bytes(&mut out, 1, text(&synthetic.collection).as_bytes());
    wire::put_bytes(
        &mut out,
        2,
        text(&synthetic.dir.join("collection.media")).as_bytes(),
    );
    wire::put_bytes(
        &mut out,
        3,
        text(&synthetic.dir.join("collection.media.db")).as_bytes(),
    );
    out
}

/// `GetDeckNamesRequest { skip_empty_default: false, include_filtered: true }`.
fn deck_names_request() -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_varint_field(&mut out, 2, 1);
    out
}

/// `GetQueuedCardsRequest { fetch_limit: 1 }`.
fn queue_request() -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_varint_field(&mut out, 1, 1);
    out
}

/// The decks a `DeckNames` response names, in its order.
fn deck_names(bytes: &[u8]) -> Vec<String> {
    wire::repeated(bytes, 1)
        .iter()
        .map(|deck| String::from_utf8(wire::bytes(deck, 2)).expect("a deck name is UTF-8"))
        .collect()
}

/// The first queued card of a `QueuedCards` response and the queue's counts.
#[derive(Debug, PartialEq, Eq, Default)]
struct Queued {
    card_id: i64,
    note_id: i64,
    deck_id: i64,
    queue: u64,
    new: u64,
    learning: u64,
    review: u64,
}

fn read_queue(bytes: &[u8]) -> Queued {
    let head = wire::repeated(bytes, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    let card = wire::bytes(&head, 1);
    Queued {
        card_id: wire::signed(&card, 1),
        note_id: wire::signed(&card, 2),
        deck_id: wire::signed(&card, 3),
        queue: wire::varint(&head, 2),
        new: wire::varint(bytes, 2),
        learning: wire::varint(bytes, 3),
        review: wire::varint(bytes, 4),
    }
}

/// What the synthetic collection's queue holds before any answer: its one new card.
fn untouched(synthetic: &Synthetic) -> Queued {
    Queued {
        card_id: synthetic.card_id,
        note_id: synthetic.note_id,
        deck_id: 1,
        queue: NEW_QUEUE,
        new: 1,
        learning: 0,
        review: 0,
    }
}

/// A `CardAnswer` rating the first queued card Good, with the states the queue gave it. A queue
/// that gave none (a refused call) yields an answer with empty states, which the adapter must
/// still route by its pair alone.
fn good_answer(synthetic: &Synthetic, queued: &Result<Vec<u8>, EngineRefusal>) -> Vec<u8> {
    let states = queued
        .as_ref()
        .map(|bytes| {
            let head = wire::repeated(bytes, 1)
                .into_iter()
                .next()
                .unwrap_or_default();
            wire::bytes(&head, 3)
        })
        .unwrap_or_default();
    let answered_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    let mut out = Vec::new();
    wire::put_varint_field(
        &mut out,
        1,
        u64::try_from(synthetic.card_id).expect("a card id is positive"),
    );
    wire::put_bytes(&mut out, 2, &wire::bytes(&states, 1));
    wire::put_bytes(&mut out, 3, &wire::bytes(&states, 4));
    wire::put_varint_field(&mut out, 4, GOOD);
    wire::put_varint_field(
        &mut out,
        5,
        u64::try_from(answered_at).expect("the time fits 64 bits"),
    );
    wire::put_varint_field(&mut out, 6, 4_000);
    out
}

/// The two flags of an `OpChanges` a scheduling change sets: `card` and `study_queues`.
fn card_and_queues(changes: &[u8]) -> (u64, u64) {
    (wire::varint(changes, 1), wire::varint(changes, 10))
}

/// Who refused a call, for an assertion that reads both kinds.
fn refused_by(result: &Result<Vec<u8>, EngineRefusal>) -> &'static str {
    match result {
        Ok(_) => "nobody",
        Err(EngineRefusal::NotAllowed { .. }) => "the allow-list",
        Err(EngineRefusal::Engine { .. }) => "the engine",
        Err(EngineRefusal::Start { .. }) => "the engine's start",
    }
}

#[test]
fn a1_opens_a_synthetic_collection() {
    let synthetic = synthetic("a1");
    let engine = engine();
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    // A second open is refused by the engine itself only while the first one holds the collection.
    let again = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    assert_eq!(
        (opened, refused_by(&again)),
        (Ok(Vec::new()), "the engine"),
        "A1: OpenCollection is answered with the engine's empty message, and the engine then holds the collection"
    );
}

#[test]
fn a2_lists_the_collections_decks() {
    let synthetic = synthetic("a2");
    let engine = engine();
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    let names = call(&engine, GET_DECK_NAMES, deck_names_request()).map(|bytes| deck_names(&bytes));
    assert_eq!(
        (opened, names),
        (
            Ok(Vec::new()),
            Ok(vec!["Default".to_owned(), SECOND_DECK.to_owned()])
        ),
        "A2: GetDeckNames names the synthetic collection's two decks"
    );
}

#[test]
fn a3_gets_the_next_card() {
    let synthetic = synthetic("a3");
    let engine = engine();
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    let next = call(&engine, GET_QUEUED_CARDS, queue_request()).map(|bytes| read_queue(&bytes));
    assert_eq!(
        (opened, next),
        (Ok(Vec::new()), Ok(untouched(&synthetic))),
        "A3: GetQueuedCards gives the synthetic note's new card, the one the collection holds"
    );
}

#[test]
fn a4_answers_the_card() {
    let synthetic = synthetic("a4");
    let engine = engine();
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    let queued = call(&engine, GET_QUEUED_CARDS, queue_request());
    let answered = call(&engine, ANSWER_CARD, good_answer(&synthetic, &queued))
        .map(|bytes| card_and_queues(&bytes));
    let after =
        call(&engine, GET_QUEUED_CARDS, queue_request()).map(|bytes| read_queue(&bytes).new);
    assert_eq!(
        (opened, answered, after),
        (Ok(Vec::new()), Ok((1, 1)), Ok(0)),
        "A4: AnswerCard rates the card Good, reports a card and queue change, and the card leaves the new queue"
    );
}

#[test]
fn a5_undoes_the_answer() {
    let synthetic = synthetic("a5");
    let engine = engine();
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    let queued = call(&engine, GET_QUEUED_CARDS, queue_request());
    let answered = call(&engine, ANSWER_CARD, good_answer(&synthetic, &queued));
    let undone = call(&engine, UNDO, Vec::new()).map(|bytes| {
        String::from_utf8(wire::bytes(&bytes, 2)).expect("an operation name is UTF-8")
    });
    let after = call(&engine, GET_QUEUED_CARDS, queue_request()).map(|bytes| read_queue(&bytes));
    assert_eq!(
        (opened, refused_by(&answered), undone, after),
        (
            Ok(Vec::new()),
            "nobody",
            Ok(ANSWER_OPERATION.to_owned()),
            Ok(untouched(&synthetic))
        ),
        "A5: Undo reverts the answer, names it, and the card is new and first again"
    );
}

#[test]
fn a6_refuses_an_unlisted_call_and_keeps_serving() {
    let synthetic = synthetic("a6");
    let engine = engine();
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    // A real engine call the list leaves out (which would close the collection if it reached the
    // engine), a method past a listed service's, and pairs the engine numbers nothing with.
    let unlisted = [CLOSE_COLLECTION, (7, 9_999), (0, 0), (u32::MAX, u32::MAX)];
    let refused: Vec<_> = unlisted
        .iter()
        .map(|&pair| call(&engine, pair, Vec::new()))
        .collect();
    let refusals: Vec<_> = unlisted
        .iter()
        .map(|&(service, method)| Err(EngineRefusal::NotAllowed { service, method }))
        .collect();
    let names = call(&engine, GET_DECK_NAMES, deck_names_request()).map(|bytes| deck_names(&bytes));
    assert_eq!(
        (refused, opened, names),
        (
            refusals,
            Ok(Vec::new()),
            Ok(vec!["Default".to_owned(), SECOND_DECK.to_owned()])
        ),
        "A6: each unlisted call is refused by the allow-list before the engine sees it, and the collection keeps serving the listed calls"
    );
}

/// The protobuf wire format, as far as these tests use it: varints and length-delimited fields
/// out, and every wire type the engine's messages use in.
mod wire {
    /// Appends a base-128 varint.
    pub fn put_varint(out: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            out.push(u8::try_from(value & 0x7f).expect("seven bits") | 0x80);
            value >>= 7;
        }
        out.push(u8::try_from(value).expect("seven bits"));
    }

    /// Appends a varint field.
    pub fn put_varint_field(out: &mut Vec<u8>, field: u64, value: u64) {
        put_varint(out, field << 3);
        put_varint(out, value);
    }

    /// Appends a length-delimited field: a string, bytes or an embedded message.
    pub fn put_bytes(out: &mut Vec<u8>, field: u64, bytes: &[u8]) {
        put_varint(out, (field << 3) | 2);
        put_varint(
            out,
            u64::try_from(bytes.len()).expect("a length fits 64 bits"),
        );
        out.extend_from_slice(bytes);
    }

    #[derive(Debug, Clone)]
    enum Value {
        Varint(u64),
        Bytes(Vec<u8>),
        Fixed,
    }

    fn read_varint(bytes: &[u8], at: &mut usize) -> u64 {
        let mut value = 0_u64;
        let mut shift = 0;
        loop {
            let byte = bytes[*at];
            *at += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return value;
            }
            shift += 7;
        }
    }

    fn fields(bytes: &[u8]) -> Vec<(u64, Value)> {
        let mut at = 0;
        let mut out = Vec::new();
        while at < bytes.len() {
            let key = read_varint(bytes, &mut at);
            let value = match key & 7 {
                0 => Value::Varint(read_varint(bytes, &mut at)),
                1 => {
                    at += 8;
                    Value::Fixed
                }
                2 => {
                    let length = usize::try_from(read_varint(bytes, &mut at))
                        .expect("a length fits the address space");
                    let value = bytes[at..at + length].to_vec();
                    at += length;
                    Value::Bytes(value)
                }
                5 => {
                    at += 4;
                    Value::Fixed
                }
                wire => panic!("wire type {wire} is not one the engine's messages use"),
            };
            out.push((key >> 3, value));
        }
        out
    }

    /// A varint field's value, or zero when the message omits it (proto3's default).
    pub fn varint(bytes: &[u8], field: u64) -> u64 {
        fields(bytes)
            .into_iter()
            .filter_map(|(number, value)| match value {
                Value::Varint(value) if number == field => Some(value),
                _ => None,
            })
            .next_back()
            .unwrap_or_default()
    }

    /// An `int64` field's value: the varint read as two's complement.
    pub fn signed(bytes: &[u8], field: u64) -> i64 {
        i64::from_ne_bytes(varint(bytes, field).to_ne_bytes())
    }

    /// A length-delimited field's bytes, or none when the message omits it.
    pub fn bytes(bytes: &[u8], field: u64) -> Vec<u8> {
        repeated(bytes, field).pop().unwrap_or_default()
    }

    /// Every occurrence of a length-delimited field, in order.
    pub fn repeated(bytes: &[u8], field: u64) -> Vec<Vec<u8>> {
        fields(bytes)
            .into_iter()
            .filter_map(|(number, value)| match value {
                Value::Bytes(value) if number == field => Some(value),
                _ => None,
            })
            .collect()
    }
}
