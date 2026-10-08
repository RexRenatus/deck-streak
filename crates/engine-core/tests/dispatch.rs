//! The dispatcher on a synthetic collection (SPEC-345 A3, A4; SPEC-365 A2).
//!
//! Each test builds its own collection of two Basic notes with the engine's own API, then drives a
//! native dispatcher the way the native adapter does: each request encoded as protobuf bytes, each
//! reply decoded from them, and each read named rather than written as SQL. The pairs are the
//! engine's generated dispatch numbers, written here, not read from the core's table.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use anki_proto::scheduler::card_answer::Rating;
use anki_proto::scheduler::{
    CardAnswer, GetQueuedCardsRequest, QueuedCards, ScheduleCardsAsNewRequest,
};
use deck_streak_engine_core::dispatch::{Dispatcher, Read, Refusal};
use deck_streak_engine_core::table::Transport;
use prost::Message;
use serde_json::Value;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// `SchedulerService.AnswerCard`.
const ANSWER_CARD: (u32, u32) = (13, 4);
/// `SchedulerService.ScheduleCardsAsNew`: Forget, an exempt write.
const SCHEDULE_CARDS_AS_NEW: (u32, u32) = (13, 17);
/// The snapshot's column for the card's repetitions: `[id, queue, type, due, ivl, reps, lapses]`.
const REPS: usize = 5;

fn dispatcher() -> Dispatcher {
    Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init")
}

fn run(
    dispatcher: &Dispatcher,
    (service, method): (u32, u32),
    input: &[u8],
) -> Result<Vec<u8>, Refusal> {
    dispatcher.run(service, method, input)
}

/// A read's first row, decoded from the engine's JSON reply.
fn first_row(reply: Result<Vec<u8>, Refusal>) -> Result<Value, Refusal> {
    reply.map(|bytes| {
        serde_json::from_slice::<Value>(&bytes)
            .expect("the engine replies with JSON rows")
            .pointer("/0")
            .cloned()
            .unwrap_or(Value::Null)
    })
}

/// One card's scheduling fields, by the core's fixed read.
fn snapshot(dispatcher: &Dispatcher, card: i64) -> Result<Value, Refusal> {
    first_row(dispatcher.read(Read::CardSnapshot(card)))
}

fn now_millis() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    i64::try_from(millis).expect("the clock's milliseconds fit an i64")
}

#[test]
fn an_ordinary_call_reaches_the_engine_and_an_exempt_one_does_not() {
    let synthetic = support::synthetic("ordinary-and-exempt");
    let dispatcher = dispatcher();
    assert_eq!(
        run(
            &dispatcher,
            OPEN_COLLECTION,
            &support::open_request(&synthetic)
        )
        .map(|_| ()),
        Ok(()),
        "the native dispatcher opens the collection"
    );
    let request = GetQueuedCardsRequest {
        fetch_limit: 1,
        intraday_learning_only: false,
    };
    let queued = run(&dispatcher, GET_QUEUED_CARDS, &request.encode_to_vec())
        .expect("the queue is admitted");
    let queued = QueuedCards::decode(queued.as_slice()).expect("the queue decodes");
    let first = queued
        .cards
        .into_iter()
        .next()
        .expect("a new card is queued");
    let card = first.card.expect("a queued card carries its card").id;
    let states = first.states.expect("a queued card carries its states");
    let before = snapshot(&dispatcher, card).expect("the snapshot reads");
    let answer = CardAnswer {
        card_id: card,
        current_state: states.current,
        new_state: states.good,
        rating: Rating::Good as i32,
        answered_at_millis: now_millis(),
        milliseconds_taken: 1000,
    };
    assert_eq!(
        run(&dispatcher, ANSWER_CARD, &answer.encode_to_vec()).map(|_| ()),
        Ok(()),
        "the answer reaches the engine"
    );
    let answered = snapshot(&dispatcher, card).expect("the snapshot reads");
    assert_eq!(
        (before[REPS].as_i64(), answered[REPS].as_i64()),
        (Some(0), Some(1)),
        "the answer raised the card's repetitions: {before} then {answered}"
    );
    let forget = ScheduleCardsAsNewRequest {
        card_ids: vec![card],
        ..ScheduleCardsAsNewRequest::default()
    };
    assert_eq!(
        run(&dispatcher, SCHEDULE_CARDS_AS_NEW, &forget.encode_to_vec()),
        Err(Refusal::NeedsGesture {
            service: 13,
            method: 17
        }),
        "Forget through run is held for a gesture"
    );
    assert_eq!(
        snapshot(&dispatcher, card),
        Ok(answered),
        "the held write left the card's row as it was"
    );
}

#[test]
fn an_answer_through_run_is_held_for_the_owner_and_leaves_the_card() {
    let synthetic = support::synthetic("answer-through-run");
    let dispatcher = dispatcher();
    run(
        &dispatcher,
        OPEN_COLLECTION,
        &support::open_request(&synthetic),
    )
    .expect("the native dispatcher opens the collection");
    let request = GetQueuedCardsRequest {
        fetch_limit: 1,
        intraday_learning_only: false,
    };
    let queued = run(&dispatcher, GET_QUEUED_CARDS, &request.encode_to_vec())
        .expect("the queue is admitted");
    let first = QueuedCards::decode(queued.as_slice())
        .expect("the queue decodes")
        .cards
        .into_iter()
        .next()
        .expect("a new card is queued");
    let card = first.card.expect("a queued card carries its card").id;
    let states = first.states.expect("a queued card carries its states");
    let answer = CardAnswer {
        card_id: card,
        current_state: states.current,
        new_state: states.good,
        rating: Rating::Good as i32,
        answered_at_millis: now_millis(),
        milliseconds_taken: 1000,
    };
    let through_run = run(&dispatcher, ANSWER_CARD, &answer.encode_to_vec()).map(|_| ());
    let after = snapshot(&dispatcher, card).expect("the snapshot reads");
    assert_eq!(
        (through_run, after[REPS].as_i64()),
        (
            Err(Refusal::NeedsAnswer {
                service: 13,
                method: 4
            }),
            Some(0)
        ),
        "a valid answer through run is held for the owner's press, and the card is not answered: {after}"
    );
}

#[test]
fn a_fixed_read_returns_its_one_card() {
    let synthetic = support::synthetic("fixed-read");
    let [first, second] = synthetic.cards;
    assert_ne!(first, second, "the collection holds two cards");
    let dispatcher = dispatcher();
    let opened = run(
        &dispatcher,
        OPEN_COLLECTION,
        &support::open_request(&synthetic),
    )
    .map(|_| ());
    let card = snapshot(&dispatcher, second).map(|row| row.pointer("/0").cloned());
    let notes = first_row(dispatcher.read(Read::NoteCount)).map(|row| row.pointer("/0").cloned());
    assert_eq!(
        (opened, card, notes),
        (
            Ok(()),
            Ok(Some(Value::from(second))),
            Ok(Some(Value::from(2)))
        ),
        "the snapshot of the second card names it, and the note count reads two"
    );
}
