//! The undo of the review's last bury or flag (SPEC-383 A1 to A16).
//!
//! The write tests drive a web dispatcher over a synthetic collection of two Basic notes, the way
//! the web engine does: the bury or the flag through its ordinary call, the record read from the
//! engine's undo status right after, and the undo through the owner's gesture. Each compares the
//! card's two fixed reads, `CardSnapshot` and `CardMark`, every column, before and after. The rule's
//! tests judge `judge_change` on fixed inputs, each differing from an admitted one in one input
//! alone, so that input alone decides the result.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and the shared support module prints what it examined"
)]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use anki_proto::collection::UndoStatus;
use anki_proto::scheduler::card_answer::Rating;
use anki_proto::scheduler::{CardAnswer, GetQueuedCardsRequest, QueuedCards};
use deck_streak_engine_core::answer::{Grade, OwnerAnswer};
use deck_streak_engine_core::dispatch::{Dispatcher, Read};
use deck_streak_engine_core::gesture::{GestureRefusal, OwnerGesture, Target};
use deck_streak_engine_core::review::{RED, bury_of, bury_request, flag_request, toggled_red};
use deck_streak_engine_core::table::{ExemptWrite, Transport};
use deck_streak_engine_core::undo_answer::{Kind, Recorded, UndoRefusal};
use deck_streak_engine_core::undo_change::{Mark, judge_change};
use prost::Message;
use serde_json::Value;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `CollectionService.GetUndoStatus`.
const GET_UNDO_STATUS: (u32, u32) = (3, 7);
/// `CardsService.SetFlag`.
const SET_FLAG: (u32, u32) = (5, 4);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// `SchedulerService.BuryOrSuspendCards`.
const BURY_OR_SUSPEND_CARDS: (u32, u32) = (13, 14);
/// The snapshot's column for the card's repetitions: `[id, queue, type, due, ivl, reps, lapses]`.
const REPS: usize = 5;
/// The mark's column for the card's queue: `[queue, flags, usn]`.
const MARK_QUEUE: usize = 0;
/// The mark's column for the card's flags.
const MARK_FLAGS: usize = 1;
/// The engine's queue for a card the user buried.
const USER_BURIED: i64 = -3;
/// The engine's number for the blue flag, which the review's red toggle replaces.
const BLUE: u32 = 4;
/// A kind no variant of [`Kind`] names.
const UNKNOWN_KIND: i32 = 7;

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

/// The card's two fixed reads, every column: its scheduling snapshot and its mark.
fn reads(dispatcher: &Dispatcher, card: i64) -> (Value, Value) {
    (
        first_row(dispatcher, Read::CardSnapshot(card)),
        first_row(dispatcher, Read::CardMark(card)),
    )
}

/// The engine's undo status now, as the web review reads it.
fn undo_status(dispatcher: &Dispatcher) -> UndoStatus {
    dispatcher
        .run(GET_UNDO_STATUS.0, GET_UNDO_STATUS.1, &[])
        .map(|bytes| UndoStatus::decode(bytes.as_slice()).expect("the undo status decodes"))
        .expect("the undo status is admitted on the web")
}

/// Buries `card` as the review does, the user's bury of that card alone, and returns the record
/// the review keeps: the engine's undo status right after, the kind and the card.
fn bury(dispatcher: &Dispatcher, card: i64) -> Recorded {
    dispatcher
        .run(
            BURY_OR_SUSPEND_CARDS.0,
            BURY_OR_SUSPEND_CARDS.1,
            &bury_request(bury_of(card)),
        )
        .expect("the bury is admitted on the web");
    Recorded {
        status: Some(undo_status(dispatcher)),
        kind: Kind::Bury.into(),
        card,
        ..Recorded::default()
    }
}

/// Sets `flag` on `card` as the review does, and returns the record the review keeps: the engine's
/// undo status right after, the kind, the flag the change left and the card.
fn flag(dispatcher: &Dispatcher, card: i64, flag: u32) -> Recorded {
    dispatcher
        .run(SET_FLAG.0, SET_FLAG.1, &flag_request(card, flag))
        .expect("the flag is admitted on the web");
    Recorded {
        status: Some(undo_status(dispatcher)),
        kind: Kind::Flag.into(),
        flag,
        card,
        ..Recorded::default()
    }
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
        .expect("a card is queued");
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
    let now = undo_status(dispatcher);
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
            status: Some(now),
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
fn undo(dispatcher: &Dispatcher, card: i64, recorded: &Recorded) -> Result<(), GestureRefusal> {
    let gesture = OwnerGesture::from_tap(ExemptWrite::Undo, Target::Card(card))
        .expect("an undo takes a card");
    dispatcher
        .run_exempt(gesture, &recorded.encode_to_vec())
        .map(|_| ())
}

/// The door's refusal of an undo on `card`.
fn refused(card: i64) -> Result<(), GestureRefusal> {
    Err(GestureRefusal::NotTheTarget {
        write: ExemptWrite::Undo,
        target: Target::Card(card),
    })
}

/// The record's undo status: its label, and the step it was taken at.
fn status(undo: &str, last_step: u32) -> UndoStatus {
    UndoStatus {
        undo: undo.to_owned(),
        redo: String::new(),
        last_step,
    }
}

/// The admitted inputs of a bury: card 7's bury, recorded at step 4 under the engine's label for
/// it, the card still user-buried and unsynced, and the engine unchanged since.
fn admitted_bury() -> (Recorded, UndoStatus, Mark, i64) {
    let recorded = Recorded {
        status: Some(status("Bury", 4)),
        kind: Kind::Bury.into(),
        card: 7,
        ..Recorded::default()
    };
    let mark = Mark {
        queue: -3,
        flags: 0,
        usn: -1,
    };
    (recorded, status("Bury", 4), mark, 7)
}

/// The admitted inputs of a flag: card 7's flag, recorded red at step 4 under the engine's label
/// for it, the card's user flag still red and unsynced, and the engine unchanged since.
fn admitted_flag() -> (Recorded, UndoStatus, Mark, i64) {
    let recorded = Recorded {
        status: Some(status("Set Flag", 4)),
        kind: Kind::Flag.into(),
        flag: RED,
        card: 7,
        ..Recorded::default()
    };
    let mark = Mark {
        queue: 0,
        flags: RED,
        usn: -1,
    };
    (recorded, status("Set Flag", 4), mark, 7)
}

/// The record as the shipped code wrote it before kinds: the undo status and the review row, tags
/// 1 and 2, and nothing else.
#[derive(Clone, PartialEq, prost::Message)]
struct Shipped {
    #[prost(message, optional, tag = "1")]
    status: Option<UndoStatus>,
    #[prost(int64, tag = "2")]
    review: i64,
}

#[test]
fn a_confirmed_undo_of_a_bury_restores_the_card_whole() {
    let (synthetic, dispatcher) = opened("undo-bury-whole");
    let card = synthetic.cards[0];
    let before = reads(&dispatcher, card);
    let recorded = bury(&dispatcher, card);
    let buried = reads(&dispatcher, card);
    let undone = undo(&dispatcher, card, &recorded);
    assert_eq!(
        (undone, reads(&dispatcher, card)),
        (Ok(()), before),
        "a confirmed undo of the bury returns every column of the card's snapshot and mark to the \
         row it had before the bury: buried {buried:?}"
    );
    assert_eq!(
        buried.1[MARK_QUEUE].as_i64(),
        Some(USER_BURIED),
        "the bury had buried the card"
    );
}

#[test]
fn a_confirmed_undo_of_a_flag_puts_back_the_flag_the_card_had() {
    let (synthetic, dispatcher) = opened("undo-flag-whole");
    let card = synthetic.cards[0];
    flag(&dispatcher, card, BLUE);
    let before = reads(&dispatcher, card);
    let recorded = flag(&dispatcher, card, toggled_red(BLUE));
    let flagged = reads(&dispatcher, card);
    let undone = undo(&dispatcher, card, &recorded);
    assert_eq!(
        (undone, reads(&dispatcher, card)),
        (Ok(()), before),
        "a confirmed undo of the red toggle puts back flag 4, every column of the card's snapshot \
         and mark as before the toggle: flagged {flagged:?}"
    );
    assert_eq!(
        flagged.1[MARK_FLAGS].as_u64(),
        Some(u64::from(RED)),
        "the toggle had replaced flag 4 with red"
    );
}

#[test]
fn an_undo_of_a_bury_after_an_answer_leaves_the_answer() {
    let (synthetic, dispatcher) = opened("undo-bury-after-answer");
    let (answered, answer) = answer_head(&dispatcher);
    let after_answer = reads(&dispatcher, answered);
    let review = first_row(&dispatcher, Read::Review(answer.review));
    let buried = other(&synthetic, answered);
    let before = reads(&dispatcher, buried);
    let recorded = bury(&dispatcher, buried);
    let undone = undo(&dispatcher, buried, &recorded);
    assert_eq!(
        (
            undone,
            reads(&dispatcher, buried),
            reads(&dispatcher, answered),
            first_row(&dispatcher, Read::Review(answer.review)),
        ),
        (Ok(()), before, after_answer.clone(), review.clone()),
        "the undo of the bury returns the buried card, and the answer before it keeps its review \
         row and its card's state"
    );
    assert_eq!(
        (after_answer.0[REPS].as_i64(), review.is_null()),
        (Some(1), false),
        "the answer had changed its card and written its review row"
    );
}

#[test]
fn an_undo_of_a_bury_after_a_later_change_is_refused_and_changes_nothing() {
    let (synthetic, dispatcher) = opened("undo-bury-after-a-later-change");
    let buried = synthetic.cards[0];
    let recorded = bury(&dispatcher, buried);
    let (answered, answer) = answer_head(&dispatcher);
    let buried_rows = reads(&dispatcher, buried);
    let answered_rows = reads(&dispatcher, answered);
    let review = first_row(&dispatcher, Read::Review(answer.review));
    let undone = undo(&dispatcher, buried, &recorded);
    assert_eq!(
        (
            undone,
            reads(&dispatcher, buried),
            reads(&dispatcher, answered),
            first_row(&dispatcher, Read::Review(answer.review)),
        ),
        (
            refused(buried),
            buried_rows.clone(),
            answered_rows.clone(),
            review.clone()
        ),
        "an undo with the bury's record after a later answer is refused, the buried card stays \
         buried, and the answer keeps its card's state and its review row"
    );
    assert_eq!(
        (
            answered,
            buried_rows.1[MARK_QUEUE].as_i64(),
            answered_rows.0[REPS].as_i64(),
            review.is_null()
        ),
        (other(&synthetic, buried), Some(USER_BURIED), Some(1), false),
        "the bury had buried the card, and the queue's head, the other card, was answered after it"
    );
}

#[test]
fn an_undo_of_a_change_aimed_at_another_card_is_refused() {
    let (synthetic, dispatcher) = opened("undo-change-aimed-elsewhere");
    let buried = synthetic.cards[0];
    let aimed = other(&synthetic, buried);
    let recorded = bury(&dispatcher, buried);
    let buried_rows = reads(&dispatcher, buried);
    let aimed_rows = reads(&dispatcher, aimed);
    let undone = undo(&dispatcher, aimed, &recorded);
    assert_eq!(
        (
            undone,
            reads(&dispatcher, buried),
            reads(&dispatcher, aimed)
        ),
        (refused(aimed), buried_rows.clone(), aimed_rows),
        "a confirmation aimed at the other card with this card's bury record is refused, and the \
         buried card stays buried"
    );
    assert_eq!(
        buried_rows.1[MARK_QUEUE].as_i64(),
        Some(USER_BURIED),
        "the bury had buried the card"
    );
}

#[test]
fn a_change_whose_card_is_gone_is_refused() {
    let (recorded, now, mark, card) = admitted_bury();
    assert_eq!(
        (
            judge_change(&recorded, &now, None, card),
            judge_change(&recorded, &now, Some(mark), card)
        ),
        (Err(UndoRefusal::Gone), Ok(())),
        "a change whose card has no row is gone; with the row it is admitted"
    );
}

#[test]
fn a_change_recorded_for_another_card_is_refused() {
    let (recorded, now, mark, card) = admitted_bury();
    let another = Recorded {
        card: card + 1,
        ..recorded.clone()
    };
    assert_eq!(
        (
            judge_change(&another, &now, Some(mark), card),
            judge_change(&recorded, &now, Some(mark), card)
        ),
        (Err(UndoRefusal::NotTheCard), Ok(())),
        "a record of another card is not the card's"
    );
}

#[test]
fn a_bury_no_longer_buried_is_refused() {
    let (recorded, now, mark, card) = admitted_bury();
    let unburied = Mark { queue: 0, ..mark };
    assert_eq!(
        (
            judge_change(&recorded, &now, Some(unburied), card),
            judge_change(&recorded, &now, Some(mark), card)
        ),
        (Err(UndoRefusal::Changed), Ok(())),
        "a bury whose card is no longer user-buried has changed"
    );
}

#[test]
fn a_flag_since_changed_is_refused() {
    let (recorded, now, mark, card) = admitted_flag();
    let blue = Mark {
        flags: BLUE,
        ..mark
    };
    assert_eq!(
        (
            judge_change(&recorded, &now, Some(blue), card),
            judge_change(&recorded, &now, Some(mark), card)
        ),
        (Err(UndoRefusal::Changed), Ok(())),
        "a flag whose card's user flag is no longer the recorded one has changed"
    );
}

#[test]
fn a_flag_is_judged_by_the_users_three_bits() {
    let (recorded, now, mark, card) = admitted_flag();
    let none = Recorded {
        flag: 0,
        ..recorded.clone()
    };
    assert_eq!(
        (
            judge_change(&recorded, &now, Some(Mark { flags: 9, ..mark }), card),
            judge_change(&none, &now, Some(Mark { flags: 8, ..mark }), card),
            judge_change(&none, &now, Some(Mark { flags: 12, ..mark }), card),
        ),
        (Ok(()), Ok(()), Err(UndoRefusal::Changed)),
        "only the low three bits are the user flag: 9 holds red and 8 none, while 12 holds blue"
    );
}

#[test]
fn a_synced_change_is_refused() {
    let (recorded, now, mark, card) = admitted_flag();
    let synced = Mark { usn: 0, ..mark };
    assert_eq!(
        (
            judge_change(&recorded, &now, Some(synced), card),
            judge_change(&recorded, &now, Some(mark), card)
        ),
        (Err(UndoRefusal::Synced), Ok(())),
        "a card a sync has sent has synced"
    );
}

#[test]
fn an_empty_undo_queue_refuses_a_change() {
    let (recorded, now, mark, card) = admitted_flag();
    let empty = status("", 4);
    assert_eq!(
        (
            judge_change(&recorded, &empty, Some(mark), card),
            judge_change(&recorded, &now, Some(mark), card)
        ),
        (Err(UndoRefusal::Gone), Ok(())),
        "an engine with nothing to undo has lost the change"
    );
}

#[test]
fn a_later_step_refuses_a_change() {
    let (recorded, now, mark, card) = admitted_flag();
    let later = status("Set Flag", 5);
    assert_eq!(
        (
            judge_change(&recorded, &later, Some(mark), card),
            judge_change(&recorded, &now, Some(mark), card)
        ),
        (Err(UndoRefusal::Changed), Ok(())),
        "a later step under the same label is a change after the flag"
    );
}

#[test]
fn another_undo_label_refuses_a_change() {
    let (recorded, now, mark, card) = admitted_flag();
    let other_label = status("Bury", 4);
    assert_eq!(
        (
            judge_change(&recorded, &other_label, Some(mark), card),
            judge_change(&recorded, &now, Some(mark), card)
        ),
        (Err(UndoRefusal::Changed), Ok(())),
        "another label at the same step is a change after the flag"
    );
}

#[test]
fn an_old_record_decodes_as_an_answer() {
    let shipped = Shipped {
        status: Some(status("Answer Card", 4)),
        review: 1_700_000_000_000,
    };
    let decoded = Recorded::decode(shipped.encode_to_vec().as_slice())
        .expect("a shipped record decodes as the record");
    assert_eq!(
        (
            Kind::try_from(decoded.kind),
            decoded.flag,
            decoded.card,
            decoded.status,
            decoded.review
        ),
        (
            Ok(Kind::Answer),
            0,
            0,
            Some(status("Answer Card", 4)),
            1_700_000_000_000
        ),
        "the bytes of a two-field record read as an answer, with no flag and no card"
    );
}

#[test]
fn a_record_of_an_unknown_kind_is_refused_and_leaves_the_answer() {
    let (_synthetic, dispatcher) = opened("undo-unknown-kind");
    let (card, recorded) = answer_head(&dispatcher);
    let unknown = Recorded {
        kind: UNKNOWN_KIND,
        ..recorded.clone()
    };
    let decoded = Recorded::decode(unknown.encode_to_vec().as_slice())
        .expect("the record decodes")
        .kind;
    let undone = undo(&dispatcher, card, &unknown);
    let answered = first_row(&dispatcher, Read::CardSnapshot(card));
    assert_eq!(
        (
            undone,
            answered[REPS].as_i64(),
            first_row(&dispatcher, Read::Review(recorded.review)).is_null()
        ),
        (refused(card), Some(1), false),
        "a real answer's record carrying a kind the engine does not name is refused at the door, \
         and the answer keeps its card's state and its review row"
    );
    assert!(
        Kind::try_from(decoded).is_err(),
        "the record carries a kind no variant names: {decoded}"
    );
}
