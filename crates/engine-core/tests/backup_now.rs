//! Back up now (SPEC-516 R4, R5; B4): the open collection written into one new file on the
//! owner's word, holding every review, card and note id of the device; never into the open
//! collection's own file, and never into a file that already holds a row.
//!
//! Each test builds its device from the support's synthetic collection with the engine's own API,
//! through a test-only engine handle of its own, and reads each file back through a fresh engine:
//! the oracle is never the core the test judges.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use anki::card::CardId;
use anki::collection::{Collection, CollectionBuilder};
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::timestamp::TimestampMillis;
use anki_proto::collection::{CloseCollectionRequest, OpenCollectionRequest};
use deck_streak_engine_core::backup_now;
use deck_streak_engine_core::dispatch::Dispatcher;
use deck_streak_engine_core::files::{Files, Standard};
use deck_streak_engine_core::full_sync::IdSets;
use deck_streak_engine_core::one_way::Reason;
use deck_streak_engine_core::table::Transport;
use prost::Message;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `BackendCollectionService.CloseCollection`, which the web column admits and the native does not.
const CLOSE_COLLECTION: (u32, u32) = (3, 1);

/// The test's own engine handle on a closed collection file.
fn engine(collection: &Path) -> Collection {
    CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the test's collection")
}

/// Answers `card` Good, as the engine's own reviewer does: one review-log row.
fn answer(collection: &Path, card: i64, clock: support::clock::Clock) {
    let mut col = engine(collection);
    let states = col
        .get_scheduling_states(CardId(card))
        .expect("the engine reads the card's next states");
    col.answer_card(&mut CardAnswer {
        card_id: CardId(card),
        current_state: states.current,
        new_state: states.good,
        rating: Rating::Good,
        answered_at: TimestampMillis(clock.at_millis()),
        milliseconds_taken: 1000,
        custom_data: None,
        from_queue: false,
    })
    .expect("the engine answers the card");
    col.close(None).expect("the engine closes the collection");
}

/// The review, card and note ids a fresh engine reads from `collection`'s file.
fn held(collection: &Path) -> [BTreeSet<i64>; 3] {
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
    let sets = [
        ids("select id from revlog"),
        ids("select id from cards"),
        ids("select id from notes"),
    ];
    col.close(None).expect("the engine closes the collection");
    sets
}

/// The review, card and note ids of `sets`.
fn rows(sets: IdSets) -> [BTreeSet<i64>; 3] {
    [sets.reviews, sets.cards, sets.notes]
}

/// A native dispatcher started from the default init, with `synthetic`'s collection open.
fn open(synthetic: &support::Synthetic) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    dispatcher.handshake(Some(br#"{"minimum_client_level":1}"#));
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &support::open_request(synthetic))
        .expect("the dispatcher opens the collection");
    dispatcher
}

#[test]
fn back_up_now_writes_every_device_id_into_its_new_file() {
    let device = support::synthetic("backup-now-every-id");
    let [first, second] = device.cards;
    answer(&device.collection, first, device.clock);
    answer(&device.collection, second, device.clock);
    let device_ids = held(&device.collection);
    let [reviews, cards, notes] = &device_ids;
    support::examined("review(s) of the device", reviews.iter().collect());
    support::examined("card(s) of the device", cards.iter().collect());
    support::examined("note(s) of the device", notes.iter().collect());
    let dispatcher = open(&device);
    let backup = device.dir.join("backup-1.anki2");
    assert!(!backup.exists(), "the backup's file is new");

    let read = backup_now::back_up(&dispatcher, &backup).expect("back up now makes the backup");

    assert_eq!(
        held(&backup),
        device_ids,
        "the backup, opened by a fresh engine, holds every review, card and note id of the device"
    );
    assert_eq!(
        rows(read),
        device_ids,
        "back up now answers the ids it read back from the backup's file"
    );
    assert_eq!(
        dispatcher.id_sets().map(rows),
        Ok(device_ids),
        "the backup leaves the device's ids unchanged"
    );
}

#[test]
fn back_up_now_refuses_the_open_collection() {
    let device = support::synthetic("backup-now-open-collection");
    let [first, _] = device.cards;
    answer(&device.collection, first, device.clock);
    let before = held(&device.collection);
    let dispatcher = open(&device);

    let refused = backup_now::back_up(&dispatcher, &device.collection);

    assert_eq!(
        refused.map(rows),
        Err(Reason::OpenCollection),
        "the open collection's own file is never a backup's"
    );
    drop(dispatcher);
    assert_eq!(
        held(&device.collection),
        before,
        "the collection is unchanged"
    );
}

#[test]
fn back_up_now_refuses_a_file_that_holds_rows() {
    let device = support::synthetic("backup-now-holds-rows-device");
    let other = support::synthetic("backup-now-holds-rows-other");
    let before = held(&other.collection);
    let [_, cards, notes] = &before;
    support::examined("card(s) of the other file", cards.iter().collect());
    support::examined("note(s) of the other file", notes.iter().collect());
    let dispatcher = open(&device);

    let refused = backup_now::back_up(&dispatcher, &other.collection);

    assert_eq!(
        refused.map(rows),
        Err(Reason::HoldsRows),
        "a file that holds a card or a note is never filled"
    );
    assert_eq!(
        held(&other.collection),
        before,
        "the other file is unchanged"
    );
}

/// A files port that answers as the standard library does, and that, the first time it is asked
/// whether a path names the open collection, closes that collection and opens `twin` in its place:
/// the engine then writes a collection other than the one whose ids back up now read first.
struct Swapped {
    dispatcher: Mutex<Option<Dispatcher>>,
    twin: Vec<u8>,
}

impl Files for Swapped {
    fn holds(&self, path: &Path) -> bool {
        Standard.holds(path)
    }

    fn same(&self, open: &Path, path: &Path) -> bool {
        let taken = self
            .dispatcher
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(dispatcher) = taken {
            let (service, method) = CLOSE_COLLECTION;
            dispatcher
                .run(
                    service,
                    method,
                    &CloseCollectionRequest::default().encode_to_vec(),
                )
                .expect("the web dispatcher closes the device's collection");
            let (service, method) = OPEN_COLLECTION;
            dispatcher
                .run(service, method, &self.twin)
                .expect("the web dispatcher opens the twin");
        }
        Standard.same(open, path)
    }
}

#[test]
fn back_up_now_refuses_a_backup_that_lacks_a_review_the_device_held() {
    let device = support::synthetic("backup-now-unheld");
    let twin = device.dir.join("twin.anki2");
    std::fs::copy(&device.collection, &twin).expect("the twin is the device before its answer");
    let [first, _] = device.cards;
    answer(&device.collection, first, device.clock);
    let [reviews, cards, notes] = held(&device.collection);
    support::examined("review(s) of the device", reviews.iter().collect());
    assert_eq!(
        held(&twin),
        [BTreeSet::new(), cards, notes],
        "the twin holds every card and note of the device, and none of its reviews"
    );
    let dispatcher =
        Dispatcher::start(Transport::Web, &[]).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &support::open_request(&device))
        .expect("the web dispatcher opens the collection");
    let media = device.dir.join("collection.media");
    let media_db = device.dir.join("collection.media.db");
    let text = |path: &Path| path.to_str().expect("a scratch path is UTF-8").to_owned();
    dispatcher.install_files(Arc::new(Swapped {
        dispatcher: Mutex::new(Some(dispatcher.clone())),
        twin: OpenCollectionRequest {
            collection_path: text(&twin),
            media_folder_path: text(&media),
            media_db_path: text(&media_db),
        }
        .encode_to_vec(),
    }));
    let backup = device.dir.join("backup-unheld.anki2");

    let refused = backup_now::back_up(&dispatcher, &backup);

    assert_eq!(
        refused.map(rows),
        Err(Reason::Unheld),
        "a backup whose file, read back, lacks a review the device held is refused"
    );
    assert_eq!(
        held(&backup)[0],
        BTreeSet::new(),
        "the engine wrote the twin, which lacks the device's review"
    );
}
