//! The native review's undo slot as the core reads and judges it (SPEC-515 R2 to R5; ADR-533 D2).
//!
//! The write tests drive a native dispatcher over a synthetic collection of two Basic notes, the
//! way the native adapter does: the press or the flag through its allow-list, the engine's undo
//! status read right after, and the step the core keeps from them. Each expected record is built
//! here from the engine's own status and fixed reads, never read back from the module under test.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and the shared support module prints what it examined"
)]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use anki_proto::collection::UndoStatus;
use anki_proto::scheduler::card_answer::Rating;
use anki_proto::scheduler::{CardAnswer, GetQueuedCardsRequest, QueuedCards, SchedulingState};
use deck_streak_engine_core::answer::{Grade, OwnerAnswer};
use deck_streak_engine_core::dispatch::{Dispatcher, Read};
use deck_streak_engine_core::native_undo::{self, FlagChange, Step};
use deck_streak_engine_core::review::{RED, flag_request};
use deck_streak_engine_core::table::Transport;
use deck_streak_engine_core::undo_answer::{Kind, Recorded, Returns, UndoRefusal};
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
/// The engine's number for a card with no user flag.
const NO_FLAG: u32 = 0;
/// The engine's number for the blue flag, which the review's red toggle replaces.
const BLUE: u32 = 4;

/// A native dispatcher over a new synthetic collection, opened.
fn opened(test: &str) -> (support::Synthetic, Dispatcher) {
    let synthetic = support::synthetic(test);
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    dispatcher
        .run(
            OPEN_COLLECTION.0,
            OPEN_COLLECTION.1,
            &support::open_request(&synthetic),
        )
        .expect("the native dispatcher opens the collection");
    (synthetic, dispatcher)
}

/// The engine's undo status now, as the native adapter reads it: the reply's bytes.
fn status(dispatcher: &Dispatcher) -> Vec<u8> {
    dispatcher
        .run(GET_UNDO_STATUS.0, GET_UNDO_STATUS.1, &[])
        .expect("the undo status is admitted natively")
}

/// The undo status `bytes` hold, decoded here.
fn decoded(bytes: &[u8]) -> UndoStatus {
    UndoStatus::decode(bytes).expect("the undo status decodes")
}

/// Sets `flag` on `card` as the native review does.
fn set_flag(dispatcher: &Dispatcher, card: i64, flag: u32) {
    dispatcher
        .run(SET_FLAG.0, SET_FLAG.1, &flag_request(card, flag))
        .expect("the flag is admitted natively");
}

/// The newest review-log row's id and card, by the core's fixed read.
fn newest(dispatcher: &Dispatcher) -> (i64, i64) {
    let bytes = dispatcher
        .read(Read::NewestReview)
        .expect("the fixed read runs");
    let rows: Value = serde_json::from_slice(&bytes).expect("the engine replies with JSON rows");
    let column = |at: &str| {
        rows.pointer(at)
            .and_then(Value::as_i64)
            .expect("the newest review row has its id and its card")
    };
    (column("/0/0"), column("/0/1"))
}

fn now_millis() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    i64::try_from(millis).expect("the clock's milliseconds fit an i64")
}

/// Answers the queue's head Good through the owner's press, and returns its card and the state it
/// was shown in.
fn press_head(dispatcher: &Dispatcher) -> (i64, Option<SchedulingState>) {
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
        .expect("the queue is admitted natively");
    let first = QueuedCards::decode(queued.as_slice())
        .expect("the queue decodes")
        .cards
        .into_iter()
        .next()
        .expect("a card is queued");
    let card = first.card.expect("a queued card carries its card").id;
    let states = first.states.expect("a queued card carries its states");
    let current = states.current.clone();
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
    (card, current)
}

/// The other card of the two.
fn other(synthetic: &support::Synthetic, card: i64) -> i64 {
    let [first, second] = synthetic.cards;
    if card == first { second } else { first }
}

#[test]
fn a_kept_answer_names_its_card_its_step_and_its_record() {
    let (synthetic, dispatcher) = opened("native-undo-answer");
    // two flag steps before the press, so the engine's last step is past 0 and 1
    set_flag(&dispatcher, synthetic.cards[1], BLUE);
    set_flag(&dispatcher, synthetic.cards[1], NO_FLAG);
    let (card, current) = press_head(&dispatcher);
    let bytes = status(&dispatcher);
    let now = decoded(&bytes);
    assert!(
        now.last_step > 1,
        "the engine's last step is past 0 and 1: {now:?}"
    );
    let (review, reviewed) = newest(&dispatcher);
    assert_eq!(reviewed, card, "the newest review row is the press's");
    let another = other(&synthetic, card);

    let kept = native_undo::answered(&dispatcher, &bytes, card, Grade::Good, current.as_ref());

    assert_eq!(
        kept.as_ref().map(|kept| {
            kept.as_ref()
                .map(|kept| (kept.card(), kept.step(), kept.did(), kept.request()))
        }),
        Ok(Some((
            card,
            now.last_step,
            Step::Answer {
                grade: Grade::Good,
                returns: Returns::New
            },
            Recorded {
                status: Some(now.clone()),
                review,
                kind: Kind::Answer.into(),
                flag: 0,
                card: 0,
            }
            .encode_to_vec(),
        ))),
        "the newest review is the pressed card's, so the answer is kept on that card, at the \
         engine's last step, and its record is the status and the newest review row"
    );
    let kept = kept.expect("the reads run").expect("the answer is kept");
    assert_eq!(
        [
            kept.names(card, now.last_step),
            kept.names(another, now.last_step),
            kept.names(card, now.last_step + 1),
        ],
        [true, false, false],
        "a confirmation names the step only by its card and its step together"
    );
    assert_eq!(
        native_undo::answered(&dispatcher, &bytes, another, Grade::Good, current.as_ref()),
        Ok(None),
        "an answer whose newest review is another card's is not kept"
    );
}

#[test]
fn a_kept_answer_is_judged_against_the_engine_now() {
    let (synthetic, dispatcher) = opened("native-undo-judged");
    let (card, current) = press_head(&dispatcher);
    let kept = native_undo::answered(
        &dispatcher,
        &status(&dispatcher),
        card,
        Grade::Good,
        current.as_ref(),
    )
    .expect("the reads run")
    .expect("the answer is kept");

    let at_once = native_undo::judged(&dispatcher, &status(&dispatcher), &kept);
    set_flag(&dispatcher, other(&synthetic, card), BLUE);
    let after_a_change = native_undo::judged(&dispatcher, &status(&dispatcher), &kept);

    assert_eq!(
        (at_once, after_a_change),
        (Ok(Ok(())), Ok(Err(UndoRefusal::Changed))),
        "the answer may be undone while it is the engine's last step and its review row stands, \
         and not after a later change"
    );
}

#[test]
fn a_kept_flag_is_the_change_its_cards_mark_shows() {
    let (synthetic, dispatcher) = opened("native-undo-flag");
    let card = synthetic.cards[0];
    set_flag(&dispatcher, card, RED);
    let bytes = status(&dispatcher);
    let now = decoded(&bytes);

    let kept = native_undo::flagged(&dispatcher, &bytes, card, NO_FLAG, RED);
    let unshown = native_undo::flagged(&dispatcher, &bytes, card, NO_FLAG, BLUE);

    assert_eq!(
        kept.map(|kept| kept.map(|kept| (kept.card(), kept.did(), kept.request()))),
        Ok(Some((
            card,
            Step::Flag {
                change: FlagChange::Added
            },
            Recorded {
                status: Some(now),
                review: 0,
                kind: Kind::Flag.into(),
                flag: RED,
                card,
            }
            .encode_to_vec(),
        ))),
        "the red flag the card's mark shows is kept, with its record"
    );
    assert_eq!(
        unshown,
        Ok(None),
        "a flag the card's mark does not show is not kept"
    );
}

#[test]
fn a_flag_change_is_named_by_the_flag_before_and_after() {
    assert_eq!(
        [
            FlagChange::of(NO_FLAG, RED),
            FlagChange::of(RED, NO_FLAG),
            FlagChange::of(BLUE, RED),
        ],
        [FlagChange::Added, FlagChange::Removed, FlagChange::Replaced],
        "from no flag is an addition, to no flag a removal, and any other change a replacement"
    );
}

#[test]
fn the_undo_offer_names_a_card_by_its_question_as_one_line() {
    let (synthetic, dispatcher) = opened("native-undo-question-line");
    let [first, second] = synthetic.cards;
    assert_eq!(
        [
            native_undo::question_line(&dispatcher, first),
            native_undo::question_line(&dispatcher, second),
        ],
        [
            Ok(String::from("synthetic front 0")),
            Ok(String::from("synthetic front 1")),
        ],
        "each card's question, as the engine renders its front, in one line"
    );
}
