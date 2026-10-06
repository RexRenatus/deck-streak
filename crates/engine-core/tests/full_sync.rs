//! The full-sync choice and the core's two reads it compares (SPEC-357 A2-A11).
//!
//! The reads run on collections each test builds with the engine's own API: reviews are answered
//! and a collection is marked synced through a test-only engine handle of the test's own, never
//! through the core's door, and the dispatcher then reads it the way the native adapter does. The
//! rule's tests build the id sets they compare directly, so each names the ids it expects.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeSet;
use std::path::Path;

use anki::card::CardId;
use anki::collection::{Collection, CollectionBuilder};
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::timestamp::TimestampMillis;
use deck_streak_engine_core::dispatch::Dispatcher;
use deck_streak_engine_core::full_sync::{IdSets, Unsynced};
use deck_streak_engine_core::table::Transport;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);

/// The test's own engine handle on a closed collection file: the door the tests use to make
/// reviews and to mark a sync, which the core does not offer an adapter.
fn engine(collection: &Path) -> Collection {
    CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the test's collection")
}

/// Answers `card` Good, as the engine's own reviewer does: one review-log row, unsynced.
fn answer(collection: &Path, card: i64) {
    let mut col = engine(collection);
    let states = col
        .get_scheduling_states(CardId(card))
        .expect("the engine reads the card's next states");
    col.answer_card(&mut CardAnswer {
        card_id: CardId(card),
        current_state: states.current,
        new_state: states.good,
        rating: Rating::Good,
        answered_at: TimestampMillis::now(),
        milliseconds_taken: 1000,
        custom_data: None,
        from_queue: false,
    })
    .expect("the engine answers the card");
    col.close(None).expect("the engine closes the collection");
}

/// Marks every row synced and the collection's last sync after its last change, as a finished
/// normal sync leaves them. The stamps move back a second first, so a later change reads after
/// the sync.
fn mark_synced(collection: &Path) {
    let col = engine(collection);
    let db = col.storage.db();
    for sql in [
        "update revlog set usn = 0",
        "update cards set usn = 0",
        "update notes set usn = 0",
        "update col set mod = mod - 1000, scm = scm - 1000",
        "update col set ls = max(mod, scm)",
    ] {
        db.execute(sql, [])
            .expect("the test marks the collection synced");
    }
    col.close(None).expect("the engine closes the collection");
}

/// Changes the collection's schema after its last sync, as a schema edit does.
fn change_schema(collection: &Path) {
    let col = engine(collection);
    col.storage
        .db()
        .execute("update col set scm = ls + 1000", [])
        .expect("the test changes the schema stamp");
    col.close(None).expect("the engine closes the collection");
}

/// The ids and the modified stamp the test's own handle reads: the oracle the core's read is
/// compared with.
fn held(collection: &Path) -> IdSets {
    let col = engine(collection);
    let db = col.storage.db();
    let ids = |sql: &str| -> BTreeSet<i64> {
        let mut statement = db.prepare(sql).expect("the test's read prepares");
        statement
            .query_map([], |row| row.get(0))
            .expect("the test's read runs")
            .map(|id| id.expect("an id reads"))
            .collect()
    };
    let sets = IdSets {
        reviews: ids("select id from revlog"),
        cards: ids("select id from cards"),
        notes: ids("select id from notes"),
        modified: db
            .query_row("select mod from col", [], |row| row.get(0))
            .expect("the modified stamp reads"),
    };
    col.close(None).expect("the engine closes the collection");
    sets
}

/// A native dispatcher started from the default init, with no sync credential and no endpoint,
/// with `synthetic`'s collection open.
fn open(synthetic: &support::Synthetic) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &support::open_request(synthetic))
        .expect("the native dispatcher opens the collection");
    dispatcher
}

#[test]
fn the_id_reads_take_every_row() {
    let synthetic = support::synthetic("id-reads");
    let [first, second] = synthetic.cards;
    answer(&synthetic.collection, first);
    answer(&synthetic.collection, second);
    let expected = held(&synthetic.collection);
    let read = open(&synthetic).id_sets();
    assert_eq!(
        read,
        Ok(expected.clone()),
        "the core reads every review, card and note id and the modified stamp"
    );
    assert_eq!(
        (expected.reviews.len(), expected.cards, expected.notes.len()),
        (2, BTreeSet::from(synthetic.cards), 2),
        "the collection holds two reviews, its two cards and two notes"
    );
}

#[test]
fn the_unsynced_read_counts_reviews_and_changes_since_the_last_sync() {
    let reviewed = support::synthetic("unsynced-reviewed");
    let [first, second] = reviewed.cards;
    answer(&reviewed.collection, first);
    answer(&reviewed.collection, second);
    mark_synced(&reviewed.collection);
    answer(&reviewed.collection, first);
    let schema = support::synthetic("unsynced-schema");
    mark_synced(&schema.collection);
    change_schema(&schema.collection);
    let quiet = support::synthetic("unsynced-quiet");
    mark_synced(&quiet.collection);
    let read = [&reviewed, &schema, &quiet].map(|synthetic| open(synthetic).unsynced());
    assert_eq!(
        read,
        [
            Ok(Unsynced {
                reviews: 1,
                changed: true,
                schema: false
            }),
            Ok(Unsynced {
                reviews: 0,
                changed: false,
                schema: true
            }),
            Ok(Unsynced::default()),
        ],
        "three reviews, two of them synced, then one review since the sync; a schema changed after \
         its sync; a collection untouched since its sync"
    );
    assert_eq!(
        read.map(|unsynced| unsynced.map(|unsynced| unsynced.warns())),
        [Ok(true), Ok(true), Ok(false)],
        "an unsynced review warns, a schema change alone warns, and nothing unsynced does not"
    );
}
