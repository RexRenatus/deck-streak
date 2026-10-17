//! The undo of the review's own last answer (SPEC-371 A4 to A14).
//!
//! The write tests drive a web dispatcher over a synthetic collection of two Basic notes, the way
//! the web engine does: the answer through the owner's press, the record read from the engine's
//! undo status and its newest review row, and the undo through the owner's gesture. The rule's
//! tests judge `judge` and `returns_to` on fixed inputs, each differing from an admitted one in one
//! input alone.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use anki_proto::collection::UndoStatus;
use anki_proto::scheduler::bury_or_suspend_cards_request::Mode;
use anki_proto::scheduler::card_answer::Rating;
use anki_proto::scheduler::scheduling_state::{self, Filtered, Normal};
use anki_proto::scheduler::{
    BuryOrSuspendCardsRequest, CardAnswer, GetQueuedCardsRequest, QueuedCards, SchedulingState,
};
use deck_streak_engine_core::answer::{Grade, OwnerAnswer};
use deck_streak_engine_core::dispatch::{Dispatcher, Read};
use deck_streak_engine_core::gesture::{GestureRefusal, OwnerGesture, Target};
use deck_streak_engine_core::table::{ExemptWrite, Transport};
use deck_streak_engine_core::undo_answer::{
    Recorded, Returns, Review, UndoRefusal, judge, returns_to,
};
use prost::Message;
use serde_json::Value;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `CollectionService.GetUndoStatus`.
const GET_UNDO_STATUS: (u32, u32) = (3, 7);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// `SchedulerService.BuryOrSuspendCards`.
const BURY_OR_SUSPEND_CARDS: (u32, u32) = (13, 14);
/// The snapshot's columns: `[id, queue, type, due, ivl, reps, lapses]`.
const QUEUE: usize = 1;
/// The snapshot's column for the card's repetitions.
const REPS: usize = 5;
/// The engine's queue for a card the user buried.
const USER_BURIED: i64 = -3;
/// A request cut short: field 1 says five bytes follow, and one does.
const NO_MESSAGE: [u8; 3] = [0x0a, 0x05, 0x01];

/// A web dispatcher over a new synthetic collection, opened.
fn opened(test: &str) -> (support::Synthetic, Dispatcher) {
    let synthetic = support::synthetic(test);
    let dispatcher =
        Dispatcher::start(Transport::Web, &[]).expect("the engine starts from the default init");
    dispatcher
        .run(
            OPEN_COLLECTION.0,
            OPEN_COLLECTION.1,
            &support::open_request(&synthetic),
        )
        .expect("the web dispatcher opens the collection");
    (synthetic, dispatcher)
}

/// A read's first row, decoded from the engine's JSON reply, or `Null` when no row matched.
fn first_row(dispatcher: &Dispatcher, read: Read) -> Value {
    let bytes = dispatcher.read(read).expect("the fixed read runs");
    serde_json::from_slice::<Value>(&bytes)
        .expect("the engine replies with JSON rows")
        .pointer("/0")
        .cloned()
        .unwrap_or(Value::Null)
}

fn now_millis() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    i64::try_from(millis).expect("the clock's milliseconds fit an i64")
}

/// Answers the queue's head Good through the owner's press, and returns its card and the record
/// the review keeps: the engine's undo status right after, and the newest review row's id.
fn answer_head(dispatcher: &Dispatcher) -> (i64, Recorded) {
    let request = GetQueuedCardsRequest {
        fetch_limit: 1,
        intraday_learning_only: false,
    };
    let queued = dispatcher
        .run(
            GET_QUEUED_CARDS.0,
            GET_QUEUED_CARDS.1,
            &request.encode_to_vec(),
        )
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
    dispatcher
        .run_answer(
            OwnerAnswer::from_press(card, Grade::Good),
            &answer.encode_to_vec(),
        )
        .expect("the owner's press records the answer");
    let undo_status = dispatcher
        .run(GET_UNDO_STATUS.0, GET_UNDO_STATUS.1, &[])
        .map(|bytes| UndoStatus::decode(bytes.as_slice()).expect("the undo status decodes"))
        .expect("the undo status is admitted on the web");
    let newest = first_row(dispatcher, Read::NewestReview);
    assert_eq!(
        newest.pointer("/1").and_then(Value::as_i64),
        Some(card),
        "the newest review row is the answer's: {newest}"
    );
    let review = newest
        .pointer("/0")
        .and_then(Value::as_i64)
        .expect("the newest review row has an id");
    (
        card,
        Recorded {
            status: Some(undo_status),
            review,
            ..Recorded::default()
        },
    )
}

/// The other card of the two.
fn other(synthetic: &support::Synthetic, card: i64) -> i64 {
    let [first, second] = synthetic.cards;
    if card == first { second } else { first }
}

/// An undo of `recorded` through the owner's gesture on `card`.
fn undo(dispatcher: &Dispatcher, card: i64, input: &[u8]) -> Result<(), GestureRefusal> {
    let gesture = OwnerGesture::from_tap(ExemptWrite::Undo, Target::Card(card))
        .expect("an undo takes a card");
    dispatcher.run_exempt(gesture, input).map(|_| ())
}

/// The record's undo status: its label, and the step it was taken at.
fn status(undo: &str, last_step: u32) -> UndoStatus {
    UndoStatus {
        undo: undo.to_owned(),
        redo: String::new(),
        last_step,
    }
}

/// The admitted inputs each rule test differs from in one input: card 7's answer, recorded at
/// step 4 under the engine's label for it, its row unsynced, and the engine unchanged since.
fn admitted() -> (Recorded, UndoStatus, Review, i64) {
    let recorded = Recorded {
        status: Some(status("Answer Card", 4)),
        review: 1_700_000_000_000,
        ..Recorded::default()
    };
    (
        recorded,
        status("Answer Card", 4),
        Review { cid: 7, usn: -1 },
        7,
    )
}

#[test]
fn an_undo_reverts_the_offered_answer() {
    let (_synthetic, dispatcher) = opened("undo-reverts");
    let request = GetQueuedCardsRequest {
        fetch_limit: 1,
        intraday_learning_only: false,
    };
    let head = dispatcher
        .run(
            GET_QUEUED_CARDS.0,
            GET_QUEUED_CARDS.1,
            &request.encode_to_vec(),
        )
        .map(|bytes| QueuedCards::decode(bytes.as_slice()).expect("the queue decodes"))
        .expect("the queue is admitted");
    let first = head.cards[0].card.as_ref().expect("a queued card").id;
    let before = first_row(&dispatcher, Read::CardSnapshot(first));
    let (card, recorded) = answer_head(&dispatcher);
    let answered = first_row(&dispatcher, Read::CardSnapshot(card));
    let undone = undo(&dispatcher, card, &recorded.encode_to_vec());
    assert_eq!(
        (
            undone,
            first_row(&dispatcher, Read::CardSnapshot(card)),
            first_row(&dispatcher, Read::Review(recorded.review)),
        ),
        (Ok(()), before.clone(), Value::Null),
        "the undo removes the answer's review row and every column of the card returns to the row it left: answered {answered}"
    );
    assert_eq!(
        (card, answered[REPS].as_i64()),
        (first, Some(1)),
        "the answer was of the queue's head, and it had changed the card"
    );
}

#[test]
fn an_undo_after_a_later_change_is_refused_and_changes_nothing() {
    let (synthetic, dispatcher) = opened("undo-after-bury");
    let (card, recorded) = answer_head(&dispatcher);
    let buried = other(&synthetic, card);
    let bury = BuryOrSuspendCardsRequest {
        card_ids: vec![buried],
        note_ids: Vec::new(),
        mode: Mode::BuryUser as i32,
    };
    dispatcher
        .run(
            BURY_OR_SUSPEND_CARDS.0,
            BURY_OR_SUSPEND_CARDS.1,
            &bury.encode_to_vec(),
        )
        .expect("the bury is admitted on the web");
    let undone = undo(&dispatcher, card, &recorded.encode_to_vec());
    let answered = first_row(&dispatcher, Read::CardSnapshot(card));
    let other_card = first_row(&dispatcher, Read::CardSnapshot(buried));
    assert_eq!(
        (undone, other_card[QUEUE].as_i64(), answered[REPS].as_i64()),
        (
            Err(GestureRefusal::NotTheTarget {
                write: ExemptWrite::Undo,
                target: Target::Card(card)
            }),
            Some(USER_BURIED),
            Some(1)
        ),
        "a bury after the answer refuses its undo: the other card stays buried and the answer stands"
    );
}

#[test]
fn an_undo_aimed_at_another_card_is_refused() {
    let (synthetic, dispatcher) = opened("undo-another-card");
    let (card, recorded) = answer_head(&dispatcher);
    let aimed = other(&synthetic, card);
    let undone = undo(&dispatcher, aimed, &recorded.encode_to_vec());
    let answered = first_row(&dispatcher, Read::CardSnapshot(card));
    assert_eq!(
        (undone, answered[REPS].as_i64()),
        (
            Err(GestureRefusal::NotTheTarget {
                write: ExemptWrite::Undo,
                target: Target::Card(aimed)
            }),
            Some(1)
        ),
        "an undo aimed at another card with this answer's record is refused, and the answer stands"
    );
}

#[test]
fn a_record_whose_review_is_gone_is_refused() {
    let (recorded, now, review, card) = admitted();
    assert_eq!(
        (
            judge(&recorded, &now, None, card),
            judge(&recorded, &now, Some(review), card)
        ),
        (Err(UndoRefusal::Gone), Ok(())),
        "a record whose review row is absent is gone; with the row it is admitted"
    );
}

#[test]
fn a_record_for_another_card_is_refused() {
    let (recorded, now, review, card) = admitted();
    let another = Review {
        cid: card + 1,
        ..review
    };
    assert_eq!(
        (
            judge(&recorded, &now, Some(another), card),
            judge(&recorded, &now, Some(review), card)
        ),
        (Err(UndoRefusal::NotTheCard), Ok(())),
        "a review row of another card is not the card's"
    );
}

#[test]
fn a_synced_answer_is_refused() {
    let (recorded, now, review, card) = admitted();
    let synced = Review { usn: 0, ..review };
    assert_eq!(
        (
            judge(&recorded, &now, Some(synced), card),
            judge(&recorded, &now, Some(review), card)
        ),
        (Err(UndoRefusal::Synced), Ok(())),
        "a review row a sync has sent is synced"
    );
}

#[test]
fn an_empty_undo_queue_is_refused() {
    let (recorded, now, review, card) = admitted();
    let empty = status("", 4);
    assert_eq!(
        (
            judge(&recorded, &empty, Some(review), card),
            judge(&recorded, &now, Some(review), card)
        ),
        (Err(UndoRefusal::Gone), Ok(())),
        "an engine with nothing to undo has lost the answer"
    );
}

#[test]
fn a_later_step_is_refused() {
    let (recorded, now, review, card) = admitted();
    let later = status("Answer Card", 5);
    assert_eq!(
        (
            judge(&recorded, &later, Some(review), card),
            judge(&recorded, &now, Some(review), card)
        ),
        (Err(UndoRefusal::Changed), Ok(())),
        "a later step under the same label is a change after the answer"
    );
}

#[test]
fn another_undo_label_is_refused() {
    let (recorded, now, review, card) = admitted();
    let other_label = status("Bury", 4);
    assert_eq!(
        (
            judge(&recorded, &other_label, Some(review), card),
            judge(&recorded, &now, Some(review), card)
        ),
        (Err(UndoRefusal::Changed), Ok(())),
        "another label at the same step is a change after the answer"
    );
}

#[test]
fn a_record_that_is_not_one_is_refused() {
    let (_synthetic, dispatcher) = opened("undo-not-a-record");
    let (card, _recorded) = answer_head(&dispatcher);
    let undone = undo(&dispatcher, card, &NO_MESSAGE);
    let answered = first_row(&dispatcher, Read::CardSnapshot(card));
    assert_eq!(
        (undone, answered[REPS].as_i64()),
        (
            Err(GestureRefusal::Undecodable {
                write: ExemptWrite::Undo
            }),
            Some(1)
        ),
        "a request that is not a record is refused before anything runs, and the answer stands"
    );
}

#[test]
fn an_undone_answer_returns_its_card_to_the_state_it_left() {
    let normal = |kind| SchedulingState {
        kind: Some(scheduling_state::Kind::Normal(Normal { kind: Some(kind) })),
        custom_data: None,
    };
    let filtered = |kind| SchedulingState {
        kind: Some(scheduling_state::Kind::Filtered(Filtered {
            kind: Some(kind),
        })),
        custom_data: None,
    };
    let review = scheduling_state::Review::default();
    let states = [
        (
            "new",
            normal(scheduling_state::normal::Kind::New(
                scheduling_state::New::default(),
            )),
            Returns::New,
        ),
        (
            "learning",
            normal(scheduling_state::normal::Kind::Learning(
                scheduling_state::Learning::default(),
            )),
            Returns::Learning,
        ),
        (
            "review",
            normal(scheduling_state::normal::Kind::Review(review)),
            Returns::Review,
        ),
        (
            "relearning",
            normal(scheduling_state::normal::Kind::Relearning(
                scheduling_state::Relearning::default(),
            )),
            Returns::Relearning,
        ),
        (
            "a filtered review",
            filtered(scheduling_state::filtered::Kind::Rescheduling(
                scheduling_state::ReschedulingFilter {
                    original_state: Some(Normal {
                        kind: Some(scheduling_state::normal::Kind::Review(review)),
                    }),
                },
            )),
            Returns::Review,
        ),
        (
            "a filtered preview",
            filtered(scheduling_state::filtered::Kind::Preview(
                scheduling_state::Preview::default(),
            )),
            Returns::Preview,
        ),
    ];
    let total = states.len();
    let mut judged = Vec::new();
    for (name, state, expected) in states {
        assert_eq!(returns_to(&state), expected, "{name}");
        judged.push(name);
    }
    let judged = support::examined("scheduling states", judged);
    println!("examined {} of {total} scheduling states", judged.len());
    assert_eq!(judged.len(), 6, "every kind reached its assertion");
}
