//! The one-way write in the order the full-sync choice's model checks it: the server copy, the
//! backup, the re-check and the write, each read from its file by the core (SPEC-364 A3-A11).
//!
//! Each test builds its device and its server from the support's synthetic collections with the
//! engine's own API, through a test-only engine handle of its own, and reads what each step left
//! through a fresh engine on the file: the oracle is never the core the test judges.

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
use deck_streak_engine_core::full_sync::{Counted, Direction, IdSets, Offer};
use deck_streak_engine_core::one_way;
use deck_streak_engine_core::table::Transport;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);

/// The test's own engine handle on a closed collection file: the door the tests use to make
/// reviews and to read a file back, which the core does not offer an adapter.
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

/// The ids and the modified stamp a fresh engine reads from `collection`'s file: the oracle each
/// step's file is compared with.
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

/// A native dispatcher started from the default init, with `synthetic`'s collection open.
fn open(synthetic: &support::Synthetic) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &support::open_request(synthetic))
        .expect("the dispatcher opens the collection");
    dispatcher
}

#[test]
fn a_downloads_backup_holds_every_device_id_before_the_write() {
    let device = support::synthetic("one-way-download-backup-device");
    let [first, second] = device.cards;
    answer(&device.collection, first);
    answer(&device.collection, second);
    let server = support::synthetic("one-way-download-backup-server");
    let device_ids = held(&device.collection);
    assert_eq!(
        device_ids.reviews.len(),
        2,
        "the device holds the fixture's two reviews"
    );
    let dispatcher = open(&device);
    let confirmed = Counted::show(
        Offer::of(true, true),
        device_ids.clone(),
        held(&server.collection),
    )
    .confirm(Direction::Download)
    .expect("the offer admits the download");
    let backup = device.dir.join("backup.anki2");

    let backed_up = one_way::back_up(&dispatcher, confirmed, &backup, &server.collection)
        .expect("a backup that holds the device is accepted");

    assert_eq!(
        held(&backup),
        device_ids,
        "the backup, opened by a fresh engine, holds every review, card and note id of the device"
    );
    assert_eq!(
        dispatcher.id_sets(),
        Ok(device_ids),
        "the backup leaves the device's ids and its modified stamp unchanged"
    );
    assert!(
        backed_up.download_ready().is_ok(),
        "a download is ready once its backup holds the device"
    );
}
