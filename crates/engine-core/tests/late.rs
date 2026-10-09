//! Whether a review cannot count toward the streak for its card's due day (SPEC-376 A1 to A7).
//!
//! The rule's tests build each card field by field and give the engine's day as literal numbers,
//! so no expected answer is computed by `past_due_day`. The day read's test compares the core's
//! read with the engine's own timing of today on the same collection file, read through the
//! engine's own interface and never through the core.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and the shared support module prints what an \
              enumerating test examined"
)]

mod support;

use anki::collection::CollectionBuilder;
use anki_proto::cards::Card;
use anki_proto::collection::CloseCollectionRequest;
use deck_streak_engine_core::dispatch::Dispatcher;
use deck_streak_engine_core::late::{EngineDay, past_due_day};
use deck_streak_engine_core::table::Transport;
use prost::Message;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `BackendCollectionService.CloseCollection`.
const CLOSE_COLLECTION: (u32, u32) = (3, 1);

/// The engine's queues, as its `CardQueue` numbers them.
const NEW: i32 = 0;
const LEARN: i32 = 1;
const REVIEW: i32 = 2;
const DAY_LEARN: i32 = 3;
const PREVIEW: i32 = 4;
const SUSPENDED: i32 = -1;
const SCHED_BURIED: i32 = -2;
const USER_BURIED: i32 = -3;

/// The engine's day the rule's tests judge in: day 100 of the collection, whose next day begins
/// at instant `1_800_086_400`, so the day began at instant `1_800_000_000`.
const DAY: EngineDay = EngineDay {
    days_elapsed: 100,
    next_day_at: 1_800_086_400,
};
/// Today, as a review card's due names it.
const TODAY: i32 = 100;
/// The instant the engine's day began.
const DAY_BEGAN: i32 = 1_800_000_000;
/// A filtered deck's position for a card it holds, as the engine numbers them from `-100_000`.
const POSITION: i32 = -100_000;
/// The home deck a filtered card came from.
const HOME_DECK: i64 = 1;
/// How far the A7 collection's creation is moved back, so its day count is not 0.
const DAYS_BACK: i64 = 10;

/// A card in its home deck: its queue and its due, and no home deck due.
fn card(queue: i32, due: i32) -> Card {
    Card {
        queue,
        due,
        original_due: 0,
        original_deck_id: 0,
        ..Card::default()
    }
}

/// A card in a filtered deck: its queue, its due there, and the due it keeps for its home deck.
fn filtered(queue: i32, due: i32, home_due: i32) -> Card {
    Card {
        queue,
        due,
        original_due: home_due,
        original_deck_id: HOME_DECK,
        ..Card::default()
    }
}

#[test]
fn a_review_card_due_yesterday_is_past_its_due_day() {
    assert_eq!(
        [TODAY - 1, TODAY - 7].map(|due| past_due_day(&card(REVIEW, due), DAY)),
        [true, true],
        "a review card due yesterday, or a week ago, is past its due day"
    );
}

#[test]
fn a_review_card_due_today_is_not_past_its_due_day() {
    assert_eq!(
        [TODAY, TODAY + 1].map(|due| past_due_day(&card(REVIEW, due), DAY)),
        [false, false],
        "a review card due today, or tomorrow, is not past its due day"
    );
}

#[test]
fn a_day_learning_card_is_judged_by_its_due_day() {
    assert_eq!(
        [TODAY - 1, TODAY].map(|due| past_due_day(&card(DAY_LEARN, due), DAY)),
        [true, false],
        "a day-learning card due yesterday is past its due day, and one due today is not"
    );
}

#[test]
fn a_filtered_card_is_judged_by_its_home_due() {
    assert_eq!(
        [
            filtered(REVIEW, POSITION, TODAY - 1),
            filtered(REVIEW, POSITION, TODAY),
            filtered(DAY_LEARN, POSITION, TODAY - 1),
            filtered(LEARN, POSITION, DAY_BEGAN - 1),
            filtered(LEARN, POSITION, DAY_BEGAN),
        ]
        .map(|card| past_due_day(&card, DAY)),
        [true, false, true, true, false],
        "a filtered card is judged by its home deck due, never by its filtered position"
    );
    assert_eq!(
        [
            filtered(REVIEW, TODAY - 1, 0),
            filtered(REVIEW, TODAY, 0),
            Card {
                original_due: TODAY - 1,
                ..card(REVIEW, TODAY)
            },
        ]
        .map(|card| past_due_day(&card, DAY)),
        [true, false, false],
        "a filtered card with no home due set, and a card in its home deck, are judged by their own due"
    );
}

#[test]
fn an_intraday_learning_card_is_judged_by_the_days_start() {
    assert_eq!(
        [DAY_BEGAN - 1, DAY_BEGAN, DAY_BEGAN + 3_600]
            .map(|due| past_due_day(&card(LEARN, due), DAY)),
        [true, false, false],
        "a learning card due before the engine's day began is past its due day, and one due at its \
         start or later is not"
    );
}

#[test]
fn a_new_or_preview_card_is_never_past_its_due_day() {
    assert_eq!(
        [
            card(NEW, 1),
            card(NEW, TODAY - 1),
            card(PREVIEW, DAY_BEGAN - 1),
            card(PREVIEW, TODAY - 1),
            card(SUSPENDED, TODAY - 1),
            card(SCHED_BURIED, TODAY - 1),
            card(USER_BURIED, TODAY - 1),
        ]
        .map(|card| past_due_day(&card, DAY)),
        [false; 7],
        "a new, preview, suspended or buried card is never past its due day, whatever its due"
    );
}

/// The engine's own timing of today on `synthetic`'s collection, read through the engine's own
/// interface with the collection open and closed again, never through the core.
fn timing_today(synthetic: &support::Synthetic) -> EngineDay {
    let mut col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine opens the collection");
    let timing = col.timing_today().expect("the engine reads its timing");
    col.close(None).expect("the engine closes the collection");
    EngineDay {
        days_elapsed: timing.days_elapsed,
        next_day_at: timing.next_day_at.0,
    }
}

#[test]
fn the_core_reads_the_engines_own_day() {
    let synthetic = support::synthetic("the_core_reads_the_engines_own_day");
    let col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine opens the collection");
    col.storage
        .db()
        .execute("update col set crt = crt - ?", [DAYS_BACK * 86_400])
        .expect("the collection's creation moves back");
    col.close(None).expect("the engine closes the collection");

    let before = timing_today(&synthetic);
    let dispatcher =
        Dispatcher::start(Transport::Web, &[]).expect("the engine starts from the default init");
    dispatcher
        .run(
            OPEN_COLLECTION.0,
            OPEN_COLLECTION.1,
            &support::open_request(&synthetic),
        )
        .expect("the web dispatcher opens the collection");
    let read = dispatcher.engine_day();
    dispatcher
        .run(
            CLOSE_COLLECTION.0,
            CLOSE_COLLECTION.1,
            &CloseCollectionRequest::default().encode_to_vec(),
        )
        .expect("the web dispatcher closes the collection");
    let after = timing_today(&synthetic);

    // The engine's day is read on each side of the core's read, so a rollover between the reads
    // leaves the core equal to one of them; with no rollover the two are one day.
    assert!(
        read == Ok(before) || read == Ok(after),
        "the core's engine day {read:?} is not the engine's own timing of today, {before:?} \
         before the read and {after:?} after it"
    );
    assert!(
        i64::from(before.days_elapsed) >= DAYS_BACK && before.next_day_at > 0,
        "the oracle's day {before:?} is not the collection's moved-back day"
    );
}
