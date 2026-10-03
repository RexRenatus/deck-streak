//! The skip day's take on a working copy, against the engine's own sync server behind the recording
//! layer (SPEC-083 A5, A9, A25, A26, A28, A29, A34, A36, A38 to A40, A44, A47 to A49, A53, A54 and
//! A56). This target is compiled at edition 2021 so that its tests set the process's zone without
//! `unsafe` (SPEC-083 section 3), and every test in it holds the target's one lock for its whole run.

// An integration test is test code: its helpers panic on a fixture that cannot be built, and it
// prints the examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

// The support module is shared with the crate's edition-2024 targets, which format it; formatted
// from this edition-2021 root it would be sorted the other way, so this root leaves it alone.
#[rustfmt::skip]
mod support;

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use deck_streak_ingest::engine::{CollectionWrite, RslibEngine};
use deck_streak_ingest::reader::is_study_event;
use deck_streak_ingest::skip::{skip_spec, SKIP_SPREAD_MAX_DAYS, SKIP_SPREAD_MIN_DAYS};
use support::synthetic::{self, SkipSetup, SKIP_CARDS};
use support::Fixture;

/// An endpoint no test here contacts.
const ENDPOINT: &str = "http://127.0.0.1:9/";
/// The workspace's pinned zone (`.cargo/config.toml`): a POSIX rule that names no zone file, at
/// the zero offset, with no daylight period.
const UTC: &str = "UTC0";
/// How long a test waits after it changes `TZ`: chrono reads the variable again at most once a
/// second on each thread.
const ZONE_SETTLES: Duration = Duration::from_millis(1100);

/// The target's one lock: the zone is the process's, and `cargo test` runs a target's tests on
/// threads of one process.
static LOCK: Mutex<()> = Mutex::new(());

/// Takes the target's one lock and sets the zone the test runs in, waiting for chrono to read it
/// again when it changed.
fn zone(rule: &str) -> MutexGuard<'static, ()> {
    let held = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    if std::env::var("TZ").ok().as_deref() != Some(rule) {
        std::env::set_var("TZ", rule);
        thread::sleep(ZONE_SETTLES);
    }
    held
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[test]
fn a_reschedule_by_the_engine_is_not_a_study_event() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let copy = fixture.copy();
    synthetic::build_skip(&copy, &SKIP_CARDS, SkipSetup::UTC);
    let moved = synthetic::skip_moved(&SKIP_CARDS);
    assert_eq!(
        synthetic::review_log(&copy),
        Vec::new(),
        "the collection holds no review-log row before the reschedule"
    );

    RslibEngine
        .set_due_date(
            &copy,
            &moved,
            &skip_spec(SKIP_SPREAD_MIN_DAYS, SKIP_SPREAD_MAX_DAYS),
        )
        .expect("the engine reschedules the cards");

    // The positive artifact: the engine's Set Due Date wrote one review-log row for each moved
    // card, of type 4 with ease 0 (R18), and no other row.
    let rows = examined(
        "review-log row(s) the reschedule wrote",
        synthetic::review_log(&copy),
    );
    let mut cards: Vec<i64> = rows.iter().map(|row| row.card).collect();
    cards.sort_unstable();
    assert_eq!(cards, moved, "one row for each moved card, and no other");
    for row in &rows {
        assert_eq!(
            (row.kind, row.ease),
            (4, 0),
            "a reschedule's row is of type 4 with ease 0: {row:?}"
        );
        assert!(
            !is_study_event(row.kind, row.ease),
            "the read's rule counts a reschedule as a study event: {row:?}"
        );
    }

    // The read counts none of them: a skip day stays a day with no study.
    let reader = synthetic::reader(&fixture.settings(), "", None, support::clock_at(0));
    let read = support::runtime()
        .block_on(reader.read(0))
        .expect("the copy reads");
    examined("card(s) the read saw", read.cards);
    assert_eq!(
        read.reviews,
        Vec::new(),
        "the read counted a reschedule as a study event"
    );
}
