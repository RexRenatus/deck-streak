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
use deck_streak_ingest::settings::SkipSearch;
use deck_streak_ingest::skip::{skip_spec, SKIP_SPREAD_MAX_DAYS, SKIP_SPREAD_MIN_DAYS};
use deck_streak_ingest::skip_write::{list_digest, preview, Preview, PreviewCard};
use deck_streak_ingest::write_class_stop::{ClassStop, StopSetter, WriteClassStop};
use deck_streak_kernel::{Environment, UtcMillis};
use sha2::{Digest, Sha256};
use support::synthetic::{self, SkipSetup, SKIP_CARDS, SKIP_FLOOR};
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

/// The configured skip search when none is set: the default (R3).
fn default_search() -> SkipSearch {
    SkipSearch::from_env(&Environment::from_vars(Vec::<(&str, &str)>::new()))
        .expect("the default search is one expression")
}

/// The D20 digest worked by hand: SHA-256 over the ascending ids, each in decimal ended by a line
/// feed, its first 16 bytes as lowercase hexadecimal.
fn digest_by_hand(ids: &[i64]) -> String {
    let mut ascending = ids.to_vec();
    ascending.sort_unstable();
    let text: String = ascending.iter().map(|id| format!("{id}\n")).collect();
    let sum = Sha256::digest(text.as_bytes());
    sum.iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn the_preview_names_a_set_stop_and_who_set_it() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let runtime = support::runtime();
    let stop = WriteClassStop::new(runtime.block_on(fixture.db()));
    let since = UtcMillis::from_epoch_millis(1_700_000_000_999);
    runtime
        .block_on(stop.set_by_counts("review_log_rows", since))
        .expect("the stop is set");

    // No collection is built: a preview that read one while the stop is set would fail here.
    let shown = runtime
        .block_on(preview(
            &RslibEngine,
            &fixture.settings(),
            &default_search(),
            &stop,
        ))
        .expect("the preview answers");
    assert_eq!(
        shown,
        Preview::Stopped(ClassStop::Stopped {
            set_by: StopSetter::Counts,
            reason: "review_log_rows".to_owned(),
            since,
        }),
        "the preview names who set the stop, why and since when, and lists nothing"
    );

    // A stop already set keeps who set it, why and since when.
    runtime
        .block_on(stop.set_by_counts("cards", UtcMillis::from_epoch_millis(1)))
        .expect("a second set is a no-op");
    assert_eq!(
        runtime.block_on(stop.read_stop()),
        ClassStop::Stopped {
            set_by: StopSetter::Counts,
            reason: "review_log_rows".to_owned(),
            since,
        }
    );
}

#[test]
fn a_missing_stop_row_reads_as_stopped() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    let stop = WriteClassStop::new(db.clone());
    assert_eq!(
        runtime.block_on(stop.read_stop()),
        ClassStop::Running,
        "the migration seeds the stop not set"
    );
    runtime.block_on(async {
        sqlx::query("DELETE FROM write_class_stop")
            .execute(db.reader())
            .await
            .expect("the row is removed");
    });
    let read = runtime.block_on(stop.read_stop());
    assert_eq!(read, ClassStop::Unread, "a missing row reads as unread");
    assert!(read.is_stopped(), "an unread stop stops every write");
    let shown = runtime
        .block_on(preview(
            &RslibEngine,
            &fixture.settings(),
            &default_search(),
            &stop,
        ))
        .expect("the preview answers");
    assert_eq!(shown, Preview::Stopped(ClassStop::Unread));
}

#[test]
fn the_class_stop_is_one_row_and_a_second_is_refused() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let runtime = support::runtime();
    let db = runtime.block_on(fixture.db());
    // A row every other check admits: only the singleton key can refuse it.
    let second = runtime.block_on(async {
        sqlx::query("INSERT INTO write_class_stop (id, stopped, created_at) VALUES (2, 0, 0)")
            .execute(db.reader())
            .await
    });
    assert!(second.is_err(), "a second stop row was admitted");
    let rows: Vec<i64> = runtime.block_on(async {
        sqlx::query_scalar("SELECT id FROM write_class_stop ORDER BY id")
            .fetch_all(db.reader())
            .await
            .expect("the rows are read")
    });
    assert_eq!(rows, [1], "the class's stop is one row, id 1");
}

#[test]
fn the_preview_lists_the_cards_and_a_changed_list_writes_nothing() {
    let _zone = zone(UTC);
    let fixture = Fixture::new(ENDPOINT);
    let copy = fixture.copy();
    let planned = synthetic::build_skip(&copy, &SKIP_CARDS, SkipSetup::UTC);
    let runtime = support::runtime();
    let stop = WriteClassStop::new(runtime.block_on(fixture.db()));
    let search = default_search();
    let skip_deck = planned.decks[synthetic::SKIP_DECK];

    // The list: exactly the cards the default search moves, each card of every other kind left out,
    // ascending by id, each under its top-level deck, with its digest. The private copy is unchanged.
    let before = std::fs::read(&copy).expect("the copy is read");
    let shown = runtime
        .block_on(preview(&RslibEngine, &fixture.settings(), &search, &stop))
        .expect("the preview answers");
    assert_eq!(std::fs::read(&copy).expect("the copy is read"), before);
    let Preview::Listed { cards, digest } = shown else {
        panic!("the preview listed nothing: {shown:?}");
    };
    let cards = examined("previewed card(s)", cards);
    let moved = synthetic::skip_moved(&SKIP_CARDS);
    let ids: Vec<i64> = cards.iter().map(|card| card.id).collect();
    assert_eq!(
        ids, moved,
        "the preview lists exactly the cards the search moves"
    );
    for card in &cards {
        assert_eq!(card.top_level_deck, skip_deck, "{card:?}");
    }
    assert_eq!(digest.len(), 32, "{digest}");
    assert!(
        digest
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "{digest}"
    );
    assert_eq!(digest, digest_by_hand(&moved));
    assert_eq!(digest, list_digest(&moved));

    // A changed collection changes the list and its digest, and the preview still writes nothing.
    let joined = SKIP_FLOOR + 12;
    synthetic::run_sql(
        &copy,
        &format!("update cards set due = due - 10 where id = {joined}"),
    );
    let before = std::fs::read(&copy).expect("the copy is read");
    let shown = runtime
        .block_on(preview(&RslibEngine, &fixture.settings(), &search, &stop))
        .expect("the preview answers");
    assert_eq!(std::fs::read(&copy).expect("the copy is read"), before);
    let Preview::Listed {
        cards,
        digest: changed,
    } = shown
    else {
        panic!("the preview listed nothing: {shown:?}");
    };
    let mut grown = moved.clone();
    grown.push(joined);
    let ids: Vec<i64> = cards.iter().map(|card: &PreviewCard| card.id).collect();
    assert_eq!(ids, grown, "the card now due joins the list");
    assert_ne!(changed, digest, "a changed list has a changed digest");
    assert_eq!(changed, digest_by_hand(&grown));
}
