//! Round trips through the adapter's one entry point against a synthetic collection (SPEC-336 A1
//! to A6), and through its answer entry, an owner's press (SPEC-365 A12, A13). Since SPEC-371
//! (A18), SPEC-336 A5 is the adapter's refusal of an undo through that entry point.
//!
//! Each test builds its own collection with the engine's own API before the adapter runs: one
//! Basic note in the default deck, and a second, empty deck. It then drives the adapter the way a
//! native client does, with each request encoded as protobuf bytes and each response decoded from
//! them. The pairs below are read from the engine's generated dispatch, not from the adapter's
//! table, so a wrong entry in the table fails here. The builder and the wire helpers are the
//! adapter's shared test support (`support/`), which encodes and decodes only what its tests send
//! and read.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

mod support;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;
use anki::decks::DeckId;
use deck_streak_ffi::engine::{Engine, EngineRefusal, PressRefusal, PressedGrade};
use support::synthetic::SECOND_DECK;
use support::{Synthetic, open_request, synthetic, wire};

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
/// The engine's card type for a card in learning (`cards.type`).
const LEARNING: i64 = 1;
/// The review log's button for Again and for Good (`revlog.ease`), the rating plus one.
const AGAIN_EASE: i64 = 1;
/// See [`AGAIN_EASE`].
const GOOD_EASE: i64 = 3;
/// Bytes that are no `SchedulingStates`: a length-delimited field 1 whose length runs past the end.
const NO_MESSAGE: [u8; 3] = [0x0a, 0x05, 0x01];

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

/// The milliseconds since the epoch, as the test's own clock reads them.
fn now_millis() -> i64 {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch");
    i64::try_from(since.as_millis()).expect("the time fits 64 bits")
}

/// Adds a second Basic note to the synthetic collection, before any engine opens it, and answers
/// its one card's id.
fn second_card(synthetic: &Synthetic) -> i64 {
    let mut col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine reopens the synthetic collection");
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the engine reads its note types")
        .expect("the stock Basic note type");
    let mut note = basic.new_note();
    note.set_field(0, "second front").expect("a front");
    note.set_field(1, "second back").expect("a back");
    col.add_note(&mut note, DeckId(1))
        .expect("the engine adds the note");
    let card = col
        .storage
        .db()
        .query_row("select id from cards where nid = ?", [note.id.0], |row| {
            row.get(0)
        })
        .expect("the note has its card");
    col.close(None).expect("the engine closes the collection");
    card
}

/// `GetQueuedCardsRequest { fetch_limit: 2 }`.
fn two_queued() -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_varint_field(&mut out, 1, 2);
    out
}

/// Each queued card of a `QueuedCards` response with the encoded `SchedulingStates` the queue
/// gave it, in the queue's order.
fn shown(bytes: &[u8]) -> Vec<(i64, Vec<u8>)> {
    wire::repeated(bytes, 1)
        .iter()
        .map(|queued| {
            (
                wire::signed(&wire::bytes(queued, 1), 1),
                wire::bytes(queued, 3),
            )
        })
        .collect()
}

/// The remaining learning steps a next state leaves: `SchedulingState.normal.learning
/// .remaining_steps`, read from the states the engine itself gave (`again` is field 2, `good` 4).
fn remaining_steps(states: &[u8], field: u64) -> i64 {
    let state = wire::bytes(states, field);
    let learning = wire::bytes(&wire::bytes(&state, 1), 2);
    i64::try_from(wire::varint(&learning, 1)).expect("a step count fits 64 bits")
}

/// What the engine wrote, read from the collection file once the engine that held it is dropped:
/// each card's `type`, `left` and `reps`, then each review's card, button, time taken and id, in
/// the order they were logged.
fn written(
    synthetic: &Synthetic,
    engine: Arc<Engine>,
    cards: &[i64],
) -> (Vec<[i64; 3]>, Vec<[i64; 4]>) {
    drop(engine);
    let col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine reopens the collection");
    let rows = cards
        .iter()
        .map(|card| {
            col.storage
                .db()
                .query_row(
                    "select type, left, reps from cards where id = ?",
                    [card],
                    |row| Ok([row.get(0)?, row.get(1)?, row.get(2)?]),
                )
                .expect("each card has its row")
        })
        .collect();
    let reviews = col
        .storage
        .db()
        .prepare("select cid, ease, time, id from revlog order by id")
        .expect("the review log reads")
        .query_map([], |row| {
            Ok([row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?])
        })
        .expect("the review log reads")
        .collect::<Result<Vec<_>, _>>()
        .expect("each review reads");
    col.close(None).expect("the engine closes the collection");
    (rows, reviews)
}

/// The owner's press of Good on the first queued card, with the states the queue gave it, through
/// the adapter's answer entry (SPEC-365 R6).
fn press_good(
    engine: &Engine,
    synthetic: &Synthetic,
    queued: &Result<Vec<u8>, EngineRefusal>,
) -> Result<Vec<u8>, PressRefusal> {
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
    engine.answer(synthetic.card_id, PressedGrade::Good, states, 4_000)
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
    let answered = press_good(&engine, &synthetic, &queued).map(|bytes| card_and_queues(&bytes));
    let after =
        call(&engine, GET_QUEUED_CARDS, queue_request()).map(|bytes| read_queue(&bytes).new);
    assert_eq!(
        (opened, answered, after),
        (Ok(Vec::new()), Ok((1, 1)), Ok(0)),
        "A4: an owner's press rates the card Good, reports a card and queue change, and the card leaves the new queue"
    );
}

/// SPEC-371 A18 (R15): an undo is an exempt write, held for the owner's gesture, so the adapter's
/// one entry point refuses it before the engine sees it, and the answered card stays answered.
/// The name is SPEC-336 A5's, kept.
#[test]
fn a5_undoes_the_answer() {
    let synthetic = synthetic("a5");
    let engine = engine();
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    let queued = call(&engine, GET_QUEUED_CARDS, queue_request());
    let answered = press_good(&engine, &synthetic, &queued);
    let undone = call(&engine, UNDO, Vec::new()).map(|_| ());
    let after =
        call(&engine, GET_QUEUED_CARDS, queue_request()).map(|bytes| read_queue(&bytes).new);
    assert_eq!(
        (opened, answered.map(|_| ()), undone, after),
        (
            Ok(Vec::new()),
            Ok(()),
            Err(EngineRefusal::NotAllowed {
                service: 3,
                method: 8
            }),
            Ok(0)
        ),
        "A18: the entry point refuses Undo by its pair, and the answered card stays out of the new queue"
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

#[test]
fn a_native_run_refuses_an_answer_and_leaves_the_card() {
    // SPEC-365 A12: the native `run` holds no answer. AnswerCard is not allowed, so a valid
    // answer with the states the queue gave never reaches the engine, and the card stays new.
    let synthetic = synthetic("native-run-answer");
    let engine = engine();
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    let queued = call(&engine, GET_QUEUED_CARDS, queue_request());
    let answered = call(&engine, ANSWER_CARD, good_answer(&synthetic, &queued));
    let (rows, reviews) = written(&synthetic, engine, &[synthetic.card_id]);
    assert_eq!(
        (opened, answered, rows, reviews),
        (
            Ok(Vec::new()),
            Err(EngineRefusal::NotAllowed {
                service: ANSWER_CARD.0,
                method: ANSWER_CARD.1
            }),
            vec![[0, 0, 0]],
            Vec::new()
        ),
        "A12: the native run refuses AnswerCard as not allowed, and the card is not answered"
    );
}

#[test]
fn a_native_press_answers_only_its_card_with_its_grade() {
    // SPEC-365 A13: a press answers its card with its grade's own next state, the one the engine
    // gave for that grade, and leaves the other card; states that are no `SchedulingStates` are
    // refused before the engine sees them.
    let synthetic = synthetic("native-press");
    let second = second_card(&synthetic);
    let engine = engine();
    let opened = call(&engine, OPEN_COLLECTION, open_request(&synthetic));
    let queued = call(&engine, GET_QUEUED_CARDS, two_queued()).expect("the queue answers");
    let [(first, first_states), (next, second_states)]: [(i64, Vec<u8>); 2] = shown(&queued)
        .try_into()
        .expect("the queue shows the two new cards");
    let before = now_millis();
    let again = engine
        .answer(first, PressedGrade::Again, first_states.clone(), 3_000)
        .map(|_| ());
    let left_new = call(&engine, GET_QUEUED_CARDS, queue_request()).map(|bytes| read_queue(&bytes));
    let good = engine
        .answer(second, PressedGrade::Good, second_states.clone(), 4_000)
        .map(|_| ());
    let after = now_millis();
    let undecodable = engine.answer(first, PressedGrade::Good, NO_MESSAGE.to_vec(), 0);
    let (rows, reviews) = written(&synthetic, engine, &[first, second]);
    let logged: Vec<[i64; 3]> = reviews
        .iter()
        .map(|&[card, ease, time, _]| [card, ease, time])
        .collect();
    let timed = reviews
        .iter()
        .all(|&[.., id]| (before..=after).contains(&id));
    assert_eq!(
        (
            (opened, next, again, good),
            rows,
            logged,
            timed,
            left_new.map(|queued| (queued.card_id, queued.new)),
            undecodable
        ),
        (
            (Ok(Vec::new()), second, Ok(()), Ok(())),
            vec![
                [LEARNING, remaining_steps(&first_states, 2), 1],
                [LEARNING, remaining_steps(&second_states, 4), 1],
            ],
            vec![[first, AGAIN_EASE, 3_000], [second, GOOD_EASE, 4_000]],
            true,
            Ok((second, 1)),
            Err(PressRefusal::Undecodable)
        ),
        "A13: Again and Good each answer their own card once, into the engine's own next state for the grade, at the adapter's time; Again's press leaves the other card new at the queue's head; undecodable states are refused before the engine"
    );
}
