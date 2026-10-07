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
use anki::sync::login::{SyncAuth as EngineAuth, sync_login};
use anki::timestamp::TimestampMillis;
use anki_proto::collection::OpenCollectionRequest;
use anki_proto::scheduler::{GetQueuedCardsRequest, QueuedCards, card_answer};
use anki_proto::sync::sync_collection_response::ChangesRequired;
use anki_proto::sync::{SyncAuth, SyncCollectionResponse};
use deck_streak_engine_core::dispatch::Dispatcher;
use deck_streak_engine_core::full_sync::{
    Checked, Counted, Counts, Direction, IdSets, Losses, Offer, Ready, SnapshotAnswer,
};
use deck_streak_engine_core::gesture::{GestureRefusal, OwnerGesture, Target};
use deck_streak_engine_core::one_way::{self, Reason, Unwritten};
use deck_streak_engine_core::table::{ExemptWrite, Transport};
use prost::Message;
use support::sync_server::{self, PASSWORD, SERVER, SyncServer, USERNAME};

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// `SchedulerService.AnswerCard`.
const ANSWER_CARD: (u32, u32) = (13, 4);

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

/// A native dispatcher with the collection at `collection` open, and no media folder: the open an
/// evicted device makes of the empty collection it was left with.
fn open_at(collection: &Path) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let request = OpenCollectionRequest {
        collection_path: collection
            .to_str()
            .expect("a scratch path is UTF-8")
            .to_owned(),
        ..OpenCollectionRequest::default()
    };
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &request.encode_to_vec())
        .expect("the dispatcher opens the collection");
    dispatcher
}

/// The review, card and note ids of `sets`, without the modified stamp a sync may move.
fn rows(sets: IdSets) -> [BTreeSet<i64>; 3] {
    [sets.reviews, sets.cards, sets.notes]
}

/// A fresh HTTP client of the engine's own type, built by its `Default`, as the native adapter's
/// login tests build one.
fn engine_client<Client: Default>() -> Client {
    Client::default()
}

/// The synthetic account logged in at `server` by the engine's own login, called directly: the
/// engine's auth for the test's own handles, and the same key and endpoint as the core's auth.
fn logged_in(server: &SyncServer) -> (EngineAuth, SyncAuth) {
    let endpoint = server.endpoint();
    let login = sync_server::runtime()
        .block_on(sync_login(
            USERNAME,
            PASSWORD,
            Some(endpoint.to_owned()),
            engine_client(),
        ))
        .unwrap_or_else(|error| panic!("the engine's own login: {error}"));
    let engine_auth = EngineAuth {
        hkey: login.hkey.clone(),
        endpoint: Some(endpoint.parse().expect("the server's endpoint is a URL")),
        io_timeout_secs: None,
    };
    let auth = SyncAuth {
        hkey: login.hkey,
        endpoint: Some(endpoint.to_owned()),
        io_timeout_secs: None,
    };
    (engine_auth, auth)
}

/// The server's collection replaced by `collection`'s, by the engine's own full upload through the
/// test's handle.
fn seed(auth: &EngineAuth, collection: &Path) {
    sync_server::runtime()
        .block_on(engine(collection).full_upload(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own upload: {error}"));
}

/// The engine's own normal sync of `collection` through the test's handle, and its answer.
fn synced(auth: &EngineAuth, collection: &Path) -> SyncCollectionResponse {
    let mut col = engine(collection);
    let output = sync_server::runtime()
        .block_on(col.normal_sync(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own normal sync: {error}"));
    col.close(None).expect("the engine closes the collection");
    output.into()
}

/// A new collection at `collection` holding the server's, by the engine's own full download.
fn download_to(auth: &EngineAuth, collection: &Path) {
    sync_server::runtime()
        .block_on(engine(collection).full_download(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own download: {error}"));
}

/// The engine's answer to a normal sync that needs a full sync: both directions offered.
fn full_sync_required() -> SyncCollectionResponse {
    SyncCollectionResponse {
        required: ChangesRequired::FullSync as i32,
        ..SyncCollectionResponse::default()
    }
}

/// The owner's tap on the one-way sync of the open collection.
fn one_way_tap() -> OwnerGesture {
    OwnerGesture::from_tap(ExemptWrite::OneWaySync, Target::Collection)
        .expect("the one-way sync takes the open collection as its target")
}

/// The choice counted over the server's collection fetched into `copy`.
fn counted(dispatcher: &Dispatcher, auth: &SyncAuth, copy: &Path) -> Counted {
    one_way::count(dispatcher, &full_sync_required(), auth, copy)
        .expect("the server's collection is fetched into the empty copy and counted")
}

/// The download confirmed over `counted`, backed up into `backup`, and ready for its write.
fn download_ready(dispatcher: &Dispatcher, counted: Counted, backup: &Path, copy: &Path) -> Ready {
    let confirmed = counted
        .confirm(Direction::Download)
        .expect("the offer admits the download");
    one_way::back_up(dispatcher, confirmed, backup, copy)
        .expect("a backup that holds the device is accepted")
        .download_ready()
        .expect("a download is ready once its backup holds the device")
}

/// The upload confirmed over `counted`, backed up by the counted copy, its snapshot found.
fn upload_checked(
    dispatcher: &Dispatcher,
    counted: Counted,
    backup: &Path,
    copy: &Path,
) -> Checked {
    let confirmed = counted
        .confirm(Direction::Upload)
        .expect("the offer admits the upload");
    one_way::back_up(dispatcher, confirmed, backup, copy)
        .expect("the counted copy, read again from its file, backs the upload")
        .snapshot_found(&SnapshotAnswer { found: true })
        .expect("a found snapshot checks the upload")
}

fn now_millis() -> i64 {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    i64::try_from(millis).expect("the clock's milliseconds fit an i64")
}

/// Answers the card at the head of the queue Good, through the dispatcher's ordinary calls, as a
/// client studies while the choice is open, and returns its id.
fn answer_next(dispatcher: &Dispatcher) -> i64 {
    let request = GetQueuedCardsRequest {
        fetch_limit: 1,
        intraday_learning_only: false,
    };
    let (service, method) = GET_QUEUED_CARDS;
    let queued = dispatcher
        .run(service, method, &request.encode_to_vec())
        .expect("the queue is admitted");
    let first = QueuedCards::decode(queued.as_slice())
        .expect("the queue decodes")
        .cards
        .into_iter()
        .next()
        .expect("a card is queued");
    let card = first.card.expect("a queued card carries its card").id;
    let states = first.states.expect("a queued card carries its states");
    let answer = anki_proto::scheduler::CardAnswer {
        card_id: card,
        current_state: states.current,
        new_state: states.good,
        rating: card_answer::Rating::Good as i32,
        answered_at_millis: now_millis(),
        milliseconds_taken: 1000,
    };
    let (service, method) = ANSWER_CARD;
    dispatcher
        .run(service, method, &answer.encode_to_vec())
        .expect("the answer reaches the engine");
    card
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

#[test]
fn a_write_takes_only_a_one_way_gesture() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("a_write_takes_only_a_one_way_gesture");
    let (engine_auth, auth) = logged_in(&server);
    let donor = support::synthetic("one-way-gesture-donor");
    seed(&engine_auth, &donor.collection);
    let device = support::synthetic("one-way-gesture-device");
    let before = held(&device.collection);
    let dispatcher = open(&device);
    let copy = device.dir.join("copy.anki2");
    let counted = counted(&dispatcher, &auth, &copy);
    let ready = download_ready(
        &dispatcher,
        counted,
        &device.dir.join("backup.anki2"),
        &copy,
    );
    let [card, _] = device.cards;
    let forget = OwnerGesture::from_tap(ExemptWrite::Forget, Target::Card(card))
        .expect("a Forget gesture takes a card");

    let written = one_way::write(&dispatcher, ready, forget, &auth);

    assert_eq!(
        written,
        Err(Unwritten::Refused(Reason::Gesture(
            GestureRefusal::WrongKind {
                write: ExemptWrite::Forget,
                target: Target::Card(card),
            }
        ))),
        "the write refuses a gesture of any other write, naming the write and the target it named"
    );
    drop(dispatcher);
    assert_eq!(
        held(&device.collection),
        before,
        "the refused write left the device's ids and its modified stamp as they were"
    );
}

#[test]
fn the_write_sends_the_direction_the_owner_confirmed_and_no_media() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    // The core's own request for each direction, built through the public chain of the choice.
    let ids = IdSets {
        reviews: BTreeSet::from([11]),
        cards: BTreeSet::from([12]),
        notes: BTreeSet::from([13]),
        modified: 14,
    };
    let planted = SyncAuth {
        hkey: String::from("planted-host-key"),
        endpoint: Some(String::from("http://127.0.0.1:1/")),
        io_timeout_secs: None,
    };
    let counted_ids = Counted::show(Offer::of(true, true), ids.clone(), ids.clone());
    let download = counted_ids
        .clone()
        .confirm(Direction::Download)
        .expect("the offer admits the download")
        .backed_up(&ids)
        .expect("the backup holds the device")
        .download_ready()
        .expect("a backed-up download is ready")
        .at_write(&ids)
        .expect("an unchanged device is written");
    let upload = counted_ids
        .confirm(Direction::Upload)
        .expect("the offer admits the upload")
        .backed_up(&ids)
        .expect("the backup holds the server copy")
        .snapshot_found(&SnapshotAnswer { found: true })
        .expect("a found snapshot checks the upload")
        .rechecked(ids.clone())
        .expect("an unchanged server is ready")
        .at_write(&ids)
        .expect("an upload is written");
    let sent: Vec<_> = [download, upload]
        .iter()
        .map(|write| {
            let request = one_way::request(write, &planted);
            (
                write.direction(),
                request.upload,
                request.server_usn,
                request.auth == Some(planted.clone()),
            )
        })
        .collect();
    assert_eq!(
        sent,
        vec![
            (Direction::Download, false, None, true),
            (Direction::Upload, true, None, true),
        ],
        "(direction, upload, server_usn, auth carried): each write sends its own direction, the \
         checked auth, and no media"
    );

    // End to end: a download leaves the device holding the server's reviews.
    let server =
        SyncServer::start("the_write_sends_the_direction_the_owner_confirmed_and_no_media");
    let (engine_auth, auth) = logged_in(&server);
    let donor = support::synthetic("one-way-direction-donor");
    let [first, second] = donor.cards;
    answer(&donor.collection, first);
    answer(&donor.collection, second);
    let donor_ids = held(&donor.collection);
    seed(&engine_auth, &donor.collection);
    let device = support::synthetic("one-way-direction-device");
    let dispatcher = open(&device);
    let copy = device.dir.join("copy.anki2");
    let counted = counted(&dispatcher, &auth, &copy);
    let ready = download_ready(
        &dispatcher,
        counted,
        &device.dir.join("backup.anki2"),
        &copy,
    );
    one_way::write(&dispatcher, ready, one_way_tap(), &auth).expect("the download is written");
    drop(dispatcher);
    assert_eq!(
        rows(held(&device.collection)),
        rows(donor_ids),
        "after the download the device holds the server's reviews, cards and notes"
    );
    support::examined("write(s)", sent);
}

#[test]
fn a_server_copy_is_fetched_only_into_an_empty_file() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("a_server_copy_is_fetched_only_into_an_empty_file");
    let (engine_auth, auth) = logged_in(&server);
    let donor = support::synthetic("one-way-copy-donor");
    answer(&donor.collection, donor.cards[0]);
    let donor_ids = held(&donor.collection);
    seed(&engine_auth, &donor.collection);
    let device = support::synthetic("one-way-copy-device");
    let full = support::synthetic("one-way-copy-full");
    let full_before = held(&full.collection);
    let dispatcher = open(&device);
    let offered = full_sync_required();
    let cases = [
        ("the open collection's path", device.collection.clone()),
        (
            "the open collection's path, spelled through another folder",
            device
                .dir
                .join("collection.media")
                .join("..")
                .join("collection.anki2"),
        ),
        (
            "a file whose collection holds rows",
            full.collection.clone(),
        ),
    ];
    let refused: Vec<(&str, Result<Counts, Reason>)> = cases
        .iter()
        .map(|(case, copy)| {
            (
                *case,
                one_way::count(&dispatcher, &offered, &auth, copy).map(|counted| counted.counts()),
            )
        })
        .collect();
    assert_eq!(
        refused,
        vec![
            ("the open collection's path", Err(Reason::OpenCollection)),
            (
                "the open collection's path, spelled through another folder",
                Err(Reason::OpenCollection)
            ),
            ("a file whose collection holds rows", Err(Reason::HoldsRows)),
        ],
        "a server copy is never fetched into the open collection nor into a file that holds rows"
    );
    assert_eq!(
        held(&full.collection),
        full_before,
        "the file that holds rows is left as it was"
    );
    let copy = device.dir.join("copy.anki2");
    let fetched =
        one_way::count(&dispatcher, &offered, &auth, &copy).map(|counted| counted.counts());
    assert!(
        fetched.is_ok(),
        "an empty path is fetched into: {fetched:?}"
    );
    assert_eq!(
        rows(held(&copy)),
        rows(donor_ids),
        "the copy, opened by a fresh engine, holds the server's reviews, cards and notes"
    );
    support::examined("refused copy path(s)", refused);
}

#[test]
fn an_uploads_backup_is_the_counted_server_copy() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("an_uploads_backup_is_the_counted_server_copy");
    let (engine_auth, auth) = logged_in(&server);
    let device = support::synthetic("one-way-upload-backup-device");
    let [first, second] = device.cards;
    answer(&device.collection, first);
    seed(&engine_auth, &device.collection);
    answer(&device.collection, second);
    let dispatcher = open(&device);
    let copy = device.dir.join("copy.anki2");
    let confirmed = counted(&dispatcher, &auth, &copy)
        .confirm(Direction::Upload)
        .expect("the offer admits the upload");
    let backup = device.dir.join("backup.anki2");

    let intact = one_way::back_up(&dispatcher, confirmed.clone(), &backup, &copy).map(drop);
    let emptied = engine(&copy);
    emptied
        .storage
        .db()
        .execute_batch("delete from revlog; delete from cards; delete from notes")
        .expect("the test empties the counted copy");
    emptied.close(None).expect("the engine closes the copy");
    let after = one_way::back_up(&dispatcher, confirmed, &backup, &copy)
        .map(drop)
        .map_err(|refused| refused.reason);

    assert_eq!(
        (intact.map_err(|refused| refused.reason), after),
        (Ok(()), Err(Reason::Unheld)),
        "the counted copy, read again from its file, backs the upload; emptied after the count, it \
         backs nothing"
    );
}

#[test]
fn a_server_changed_after_the_count_returns_to_the_counts() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("a_server_changed_after_the_count_returns_to_the_counts");
    let (engine_auth, auth) = logged_in(&server);
    let device = support::synthetic("one-way-recheck-device");
    let [first, second] = device.cards;
    answer(&device.collection, first);
    seed(&engine_auth, &device.collection);
    let dispatcher = open(&device);
    let copy = device.dir.join("copy.anki2");
    let checked = upload_checked(
        &dispatcher,
        counted(&dispatcher, &auth, &copy),
        &device.dir.join("backup.anki2"),
        &copy,
    );
    let second_client =
        support::scratch("engine-core-one-way", "a-second-client").join("second.anki2");
    download_to(&engine_auth, &second_client);
    answer(&second_client, second);
    assert_eq!(
        synced(&engine_auth, &second_client).required(),
        ChangesRequired::NoChanges,
        "the second client's review reaches the server by a normal sync"
    );

    let rechecked = one_way::recheck(&dispatcher, checked, &auth, &device.dir.join("fresh.anki2"))
        .map(|step| step.map(drop).map_err(|counted| counted.counts().upload))
        .map_err(|refused| refused.reason);

    assert_eq!(
        rechecked,
        Ok(Err(Some(Losses {
            reviews: 1,
            cards: 0,
            notes: 0,
        }))),
        "a server changed since the count returns the upload to new counts, which lose the second \
         client's review"
    );
}

#[test]
fn a_device_review_after_the_backup_refuses_the_download() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("a_device_review_after_the_backup_refuses_the_download");
    let (engine_auth, auth) = logged_in(&server);
    let device = support::synthetic("one-way-late-review-device");
    seed(&engine_auth, &device.collection);
    let dispatcher = open(&device);
    let copy = device.dir.join("copy.anki2");
    let ready = download_ready(
        &dispatcher,
        counted(&dispatcher, &auth, &copy),
        &device.dir.join("backup.anki2"),
        &copy,
    );
    let reviewed = answer_next(&dispatcher);

    let written =
        one_way::write(&dispatcher, ready, one_way_tap(), &auth).map_err(
            |unwritten| match unwritten {
                Unwritten::Recount(counted) => Ok(counted.counts().download),
                Unwritten::Refused(reason) => Err(reason),
            },
        );

    assert_eq!(
        written,
        Err(Ok(Some(Losses {
            reviews: 1,
            cards: 0,
            notes: 0,
        }))),
        "a review made on the device after its backup refuses the download and returns to new \
         counts, which lose that review"
    );
    drop(dispatcher);
    let kept = held(&device.collection);
    assert_eq!(
        (kept.reviews.len(), kept.cards.contains(&reviewed)),
        (1, true),
        "the device keeps the review it made after the backup"
    );
}

#[test]
fn the_backup_and_the_server_copies_outlive_the_write() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("the_backup_and_the_server_copies_outlive_the_write");
    let (engine_auth, auth) = logged_in(&server);
    let donor = support::synthetic("one-way-outlive-donor");
    answer(&donor.collection, donor.cards[0]);
    seed(&engine_auth, &donor.collection);
    let device = support::synthetic("one-way-outlive-device");
    answer(&device.collection, device.cards[1]);
    let dispatcher = open(&device);
    let path = |name: &str| device.dir.join(name);

    let ready = download_ready(
        &dispatcher,
        counted(&dispatcher, &auth, &path("download-copy.anki2")),
        &path("download-backup.anki2"),
        &path("download-copy.anki2"),
    );
    let mut made: Vec<(&str, IdSets)> = ["download-copy.anki2", "download-backup.anki2"]
        .into_iter()
        .map(|name| (name, held(&path(name))))
        .collect();
    one_way::write(&dispatcher, ready, one_way_tap(), &auth).expect("the download is written");
    let checked = upload_checked(
        &dispatcher,
        counted(&dispatcher, &auth, &path("upload-copy.anki2")),
        &path("upload-backup.anki2"),
        &path("upload-copy.anki2"),
    );
    let ready = one_way::recheck(&dispatcher, checked, &auth, &path("upload-fresh.anki2"))
        .expect("the fresh copy is fetched")
        .expect("an unchanged server is ready for the upload");
    made.extend(
        ["upload-copy.anki2", "upload-fresh.anki2"]
            .into_iter()
            .map(|name| (name, held(&path(name)))),
    );
    one_way::write(&dispatcher, ready, one_way_tap(), &auth).expect("the upload is written");

    let kept: Vec<(&str, IdSets)> = made
        .iter()
        .map(|(name, _)| (*name, held(&path(name))))
        .collect();
    assert_eq!(
        kept, made,
        "the backup and every server copy the choice made still open, after both writes, with the \
         ids they held before them"
    );
    assert!(
        kept.iter().all(|(_, ids)| ids.cards.len() == 2),
        "each kept file holds a collection's two cards: {kept:?}"
    );
    support::examined("kept file(s)", kept);
}

#[test]
fn an_evicted_device_is_offered_the_download_alone_and_restores_every_review() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start(
        "an_evicted_device_is_offered_the_download_alone_and_restores_every_review",
    );
    let (engine_auth, auth) = logged_in(&server);
    let donor = support::synthetic("one-way-evicted-donor");
    let [first, second] = donor.cards;
    answer(&donor.collection, first);
    answer(&donor.collection, second);
    let donor_ids = held(&donor.collection);
    seed(&engine_auth, &donor.collection);
    let dir = support::scratch("engine-core-one-way", "an-evicted-device");
    let device = dir.join("collection.anki2");
    engine(&device)
        .close(None)
        .expect("the engine closes the new, empty collection");
    let offered = synced(&engine_auth, &device);
    let dispatcher = open_at(&device);
    let copy = dir.join("copy.anki2");
    let counted = one_way::count(&dispatcher, &offered, &auth, &copy)
        .expect("the server's collection is fetched into the empty copy and counted");

    assert_eq!(
        counted.counts(),
        Counts {
            upload: None,
            download: Some(Losses {
                reviews: 0,
                cards: 0,
                notes: 0,
            }),
        },
        "an empty device is offered the download alone, and the download loses nothing of it"
    );
    let ready = download_ready(&dispatcher, counted, &dir.join("backup.anki2"), &copy);
    one_way::write(&dispatcher, ready, one_way_tap(), &auth)
        .expect("the owner's download is written");
    drop(dispatcher);
    assert_eq!(
        (donor_ids.reviews.len(), rows(held(&device))),
        (2, rows(donor_ids)),
        "after the owner's download the evicted device holds every review the server holds"
    );
}
