//! SPEC-460 A1 and A2 (R1 to R5; ADR-477 D2; ADR-376 D3): an answer recorded through `run_answer`
//! on the states its card was shown with, after the card's scheduling state changed between the
//! show and the answer, is refused by the engine, and the refusal leaves the collection as it was.
//!
//! Each test opens a synthetic collection the engine builds, on a native dispatcher, takes the
//! states the queue showed its new cards with, and records an answer as the native adapter does:
//! `answer_request` from the shown states, `OwnerAnswer::from_press` and `run_answer`. Between the
//! show and the answer, A1 records the same request once, so another answer changes the card, and
//! A2 sets the card's due date through the owner's gesture, so another door changes it. Each test
//! asserts the engine's refusal by its kind and its message head, the card's snapshot and the
//! newest review-log row unchanged by it, and A1 also that the other card, shown at the same show,
//! is then recorded.
//!
//! MUTATION COVERAGE, not red first: the refusal is the engine's and is already on dev; rows S46001 and S46002 are its reds.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and the shared support module prints what an \
              enumerating test examined"
)]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::scheduler::{
    GetQueuedCardsRequest, QueuedCards, SchedulingStates, SetDueDateRequest,
};
use deck_streak_engine_core::answer::{AnswerRefusal, Grade, OwnerAnswer, answer_request};
use deck_streak_engine_core::dispatch::{Dispatcher, Read};
use deck_streak_engine_core::gesture::{OwnerGesture, Target};
use deck_streak_engine_core::table::{ExemptWrite, Transport};
use prost::Message;
use serde_json::Value;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// The snapshot's column for the card's repetitions: `[id, queue, type, due, ivl, reps, lapses]`.
const REPS: usize = 5;
/// The head of the engine's refusal of a stale current state, its message up to its first colon,
/// spelt here and never read from the engine.
const STALE: &str = "card was modified";

/// A native dispatcher with `synthetic` open.
fn opened(synthetic: &support::Synthetic) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &support::open_request(synthetic))
        .expect("the native dispatcher opens the collection");
    dispatcher
}

/// The synthetic collection's two new cards, each with the states the queue showed it with, in
/// the queue's order.
fn shown(dispatcher: &Dispatcher) -> Vec<(i64, SchedulingStates)> {
    let request = GetQueuedCardsRequest {
        fetch_limit: 2,
        intraday_learning_only: false,
    };
    let (service, method) = GET_QUEUED_CARDS;
    let queued = dispatcher
        .run(service, method, &request.encode_to_vec())
        .expect("the queue is admitted");
    QueuedCards::decode(queued.as_slice())
        .expect("the queue decodes")
        .cards
        .into_iter()
        .map(|queued| {
            (
                queued.card.expect("a queued card carries its card").id,
                queued.states.expect("a queued card carries its states"),
            )
        })
        .collect()
}

fn now_millis() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    i64::try_from(millis).expect("the clock's milliseconds fit an i64")
}

/// An owner's press of `grade` on `card`, recorded with `request` through the one door.
fn press(
    dispatcher: &Dispatcher,
    card: i64,
    grade: Grade,
    request: &[u8],
) -> Result<(), AnswerRefusal> {
    dispatcher
        .run_answer(OwnerAnswer::from_press(card, grade), request)
        .map(|_| ())
}

/// The request an adapter writes for a press of `grade` on `card`, shown with `states`: the
/// grade's pick of the next state and the grade's own rating.
fn request(card: i64, states: &SchedulingStates, grade: Grade) -> Vec<u8> {
    answer_request(
        card,
        states.current.clone(),
        grade.pick(states.again.clone(), states.good.clone()),
        grade,
        now_millis(),
        1000,
    )
}

/// One card's snapshot row, all seven columns, by the core's fixed read.
fn snapshot(dispatcher: &Dispatcher, card: i64) -> Value {
    let reply = dispatcher
        .read(Read::CardSnapshot(card))
        .expect("the snapshot reads");
    serde_json::from_slice::<Value>(&reply)
        .expect("the engine replies with JSON rows")
        .pointer("/0")
        .cloned()
        .unwrap_or(Value::Null)
}

/// The newest review-log row's id and card: the engine's raw reply to the core's fixed read.
fn newest(dispatcher: &Dispatcher) -> Vec<u8> {
    dispatcher
        .read(Read::NewestReview)
        .expect("the newest review reads")
}

/// An answer's outcome with an engine refusal decoded: its `BackendError`'s kind and its message
/// up to the first colon. The core's own three refusals, which the engine never saw, read `None`.
fn engine_refusal(outcome: Result<(), AnswerRefusal>) -> Result<(), Option<(Kind, String)>> {
    outcome.map_err(|refusal| match refusal {
        AnswerRefusal::Engine { error } => {
            let error = BackendError::decode(error.as_slice())
                .expect("an engine refusal decodes as the engine's error");
            let kind = error.kind();
            let head = match error.message.split_once(':') {
                Some((head, _)) => head.to_owned(),
                None => error.message,
            };
            Some((kind, head))
        }
        AnswerRefusal::Undecodable
        | AnswerRefusal::NotTheCard { .. }
        | AnswerRefusal::NotTheGrade { .. } => None,
    })
}

/// SPEC-460 A1 (R1, R2, R4, R5): the first card's Good press is recorded, and the same request,
/// on the states shown before that answer, is the engine's refusal; the card and the review log
/// read as the first answer left them, and the second card, shown at the same show, is recorded.
#[test]
fn a_second_press_on_the_states_shown_before_the_first_is_refused_by_the_engine() {
    let synthetic = support::synthetic("stale-answered-between");
    let dispatcher = opened(&synthetic);
    let [(card, states), (second, second_states)]: [(i64, SchedulingStates); 2] =
        shown(&dispatcher)
            .try_into()
            .expect("the queue shows the synthetic collection's two new cards");
    let bytes = request(card, &states, Grade::Good);

    let recorded = press(&dispatcher, card, Grade::Good, &bytes);
    let answered = (snapshot(&dispatcher, card), newest(&dispatcher));
    let answered_reps = answered.0.get(REPS).and_then(Value::as_i64);
    let stale = engine_refusal(press(&dispatcher, card, Grade::Good, &bytes));
    let after = (snapshot(&dispatcher, card), newest(&dispatcher));
    let other = press(
        &dispatcher,
        second,
        Grade::Good,
        &request(second, &second_states, Grade::Good),
    );
    let other_reps = snapshot(&dispatcher, second)
        .get(REPS)
        .and_then(Value::as_i64);

    assert_eq!(
        (recorded, answered_reps, stale, after, other, other_reps),
        (
            Ok(()),
            Some(1),
            Err(Some((Kind::InvalidInput, STALE.to_owned()))),
            answered,
            Ok(()),
            Some(1)
        ),
        "the first press is recorded with one repetition; the same request again is the engine's \
         invalid-input refusal headed `{STALE}`; the card's snapshot and the newest review read as \
         the first answer left them; the second card, on the states it was shown with, is recorded \
         with one repetition"
    );
}

/// SPEC-460 A2 (R1, R3, R4): the owner's gesture sets the first card's due date, which changes its
/// snapshot, and the Good press on the states shown before it is the engine's refusal; the card and
/// the review log read as the gesture left them.
#[test]
fn a_press_on_the_states_shown_before_a_due_date_was_set_is_refused_by_the_engine() {
    let synthetic = support::synthetic("stale-due-date-between");
    let dispatcher = opened(&synthetic);
    let (card, states) = shown(&dispatcher)
        .into_iter()
        .next()
        .expect("the queue shows a new card");
    let shown_snapshot = snapshot(&dispatcher, card);

    let set = OwnerGesture::from_tap(ExemptWrite::SetDueDate, Target::Card(card))
        .and_then(|gesture| {
            dispatcher.run_exempt(
                gesture,
                &SetDueDateRequest {
                    card_ids: vec![card],
                    days: String::from("1"),
                    config_key: None,
                }
                .encode_to_vec(),
            )
        })
        .map(|_| ());
    let changed = (snapshot(&dispatcher, card), newest(&dispatcher));
    let stale = engine_refusal(press(
        &dispatcher,
        card,
        Grade::Good,
        &request(card, &states, Grade::Good),
    ));
    let after = (snapshot(&dispatcher, card), newest(&dispatcher));

    assert_eq!(
        (set, changed.0 != shown_snapshot, stale, after),
        (
            Ok(()),
            true,
            Err(Some((Kind::InvalidInput, STALE.to_owned()))),
            changed
        ),
        "the owner's due date is set and changes the card's snapshot; the press on the states \
         shown before it is the engine's invalid-input refusal headed `{STALE}`; the card's \
         snapshot and the newest review read as the gesture left them"
    );
}
