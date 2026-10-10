//! The one-way write in the order the full-sync choice's model checks it: the server copy, the
//! backup, the re-check and the write, each read from its file by the core (SPEC-364 A3-A11), and
//! the upload re-check stamp the re-check compares (A15-A21).
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
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use anki::card::CardId;
use anki::collection::{Collection, CollectionBuilder};
use anki::notes::{Note, NoteId};
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::sync::login::{SyncAuth as EngineAuth, sync_login};
use anki::timestamp::TimestampMillis;
use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::collection::OpenCollectionRequest;
use anki_proto::scheduler::{GetQueuedCardsRequest, QueuedCards, card_answer};
use anki_proto::sync::sync_collection_response::ChangesRequired;
use anki_proto::sync::{SyncAuth, SyncCollectionResponse};
use deck_streak_engine_core::answer::{Grade, OwnerAnswer};
use deck_streak_engine_core::dispatch::Dispatcher;
use deck_streak_engine_core::dispatch::Refusal;
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

/// The ids a fresh engine reads from `collection`'s file, and its upload re-check stamp computed
/// by the support apart from the core: the oracle each step's file is compared with.
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
        modified: support::stamp(&col),
    };
    col.close(None).expect("the engine closes the collection");
    sets
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

/// A native dispatcher with the collection at `collection` open, and no media folder: the open an
/// evicted device makes of the empty collection it was left with.
fn open_at(collection: &Path) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    dispatcher.handshake(Some(br#"{"minimum_client_level":1}"#));
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

/// Answers the card at the head of the queue Good, through the owner's answer door (SPEC-365 R3),
/// after the dispatcher's ordinary queue read, as a client studies while the choice is open, and
/// returns its id.
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
    dispatcher
        .run_answer(
            OwnerAnswer::from_press(card, Grade::Good),
            &answer.encode_to_vec(),
        )
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

/// The counts a changed server returns the upload to when the change added and removed no id.
const NO_LOSS: Losses = Losses {
    reviews: 0,
    cards: 0,
    notes: 0,
};

/// The first note of the collection `col` holds, read by the engine.
fn first_note(col: &Collection) -> Note {
    let id: i64 = col
        .storage
        .db()
        .query_row("select min(id) from notes", [], |row| row.get(0))
        .expect("the test's read of a note id runs");
    col.storage
        .get_note(NoteId(id))
        .expect("the engine reads the note")
        .expect("the note exists")
}

/// The second client's normal sync, which carries its change to the server.
fn normal_sync(auth: &EngineAuth, collection: &Path) {
    assert_eq!(
        synced(auth, collection).required(),
        ChangesRequired::NoChanges,
        "the second client's change reaches the server by a normal sync"
    );
}

/// An upload counted and backed up on a device seeded after `plant`, then a second client's
/// `change`, which `send` carries to the server, then the re-check: its outcome with the counts it
/// returned to as the upload's losses, and whether the stamp the oracle reads differs between the
/// counted copy and the fresh one.
fn rechecked_after(
    server: &SyncServer,
    test: &str,
    plant: impl FnOnce(&Path),
    change: impl FnOnce(&Path),
    send: fn(&EngineAuth, &Path),
) -> (Result<Result<(), Option<Losses>>, Reason>, bool) {
    let (engine_auth, auth) = logged_in(server);
    let device = support::synthetic(test);
    plant(&device.collection);
    seed(&engine_auth, &device.collection);
    let dispatcher = open(&device);
    let copy = device.dir.join("copy.anki2");
    let checked = upload_checked(
        &dispatcher,
        counted(&dispatcher, &auth, &copy),
        &device.dir.join("backup.anki2"),
        &copy,
    );
    let second_client = support::scratch("engine-core-one-way", test).join("second.anki2");
    download_to(&engine_auth, &second_client);
    change(&second_client);
    send(&engine_auth, &second_client);
    let fresh = device.dir.join("fresh.anki2");

    let rechecked = one_way::recheck(&dispatcher, checked, &auth, &fresh)
        .map(|step| step.map(drop).map_err(|counted| counted.counts().upload))
        .map_err(|refused| refused.reason);
    drop(dispatcher);
    (rechecked, held(&copy).modified != held(&fresh).modified)
}

/// Edits the first note's first field through the engine, and returns the note as edited.
fn edit_first_note(col: &mut Collection, text: &str) -> Note {
    let mut note = first_note(col);
    note.set_field(0, text).expect("the note takes the edit");
    col.update_note(&mut note)
        .expect("the engine updates the note");
    note
}

#[test]
fn an_edit_that_adds_no_id_returns_the_upload_to_the_counts() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("an_edit_that_adds_no_id_returns_the_upload_to_the_counts");

    let rechecked = rechecked_after(
        &server,
        "one-way-edit-no-id",
        |_| (),
        |second| {
            let mut col = engine(second);
            edit_first_note(&mut col, "edited after the count");
            col.close(None).expect("the engine closes the collection");
        },
        normal_sync,
    );

    assert_eq!(
        rechecked,
        (Ok(Err(Some(NO_LOSS))), true),
        "a note edit that adds no id moves the stamp and returns the upload to the counts"
    );
}

#[test]
fn a_tag_or_config_change_alone_returns_the_upload_to_the_counts() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("a_tag_or_config_change_alone_returns_the_upload_to_the_counts");

    let tag = rechecked_after(
        &server,
        "one-way-new-tag",
        |_| (),
        |second| {
            let col = engine(second);
            let db = col.storage.db();
            db.execute(
                "insert into tags (tag, usn, collapsed, config) values ('planted', -1, 0, null)",
                [],
            )
            .expect("the second client registers a tag no note carries");
            db.execute("update col set mod = ?1", [now_millis()])
                .expect("the second client's collection reads as changed");
            col.close(None).expect("the engine closes the collection");
        },
        normal_sync,
    );
    let config = rechecked_after(
        &server,
        "one-way-config",
        |_| (),
        |second| {
            let mut col = engine(second);
            col.set_config_json("plantedAfterTheCount", &true, true)
                .expect("the engine sets the config key");
            col.close(None).expect("the engine closes the collection");
        },
        normal_sync,
    );

    assert_eq!(
        [tag, config],
        [
            (Ok(Err(Some(NO_LOSS))), true),
            (Ok(Err(Some(NO_LOSS))), true)
        ],
        "a new tag name alone, and a config change alone, each move the stamp and return the \
         upload to the counts"
    );
}

#[test]
fn a_deletion_returns_the_upload_to_the_counts() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("a_deletion_returns_the_upload_to_the_counts");
    let deck = "Planted before the seed";

    let rechecked = rechecked_after(
        &server,
        "one-way-deletion",
        |device| {
            let mut col = engine(device);
            col.get_or_create_normal_deck(deck)
                .expect("the engine adds an empty deck");
            col.close(None).expect("the engine closes the collection");
        },
        |second| {
            let mut col = engine(second);
            let id = col
                .get_deck_id(deck)
                .expect("the engine reads the deck's id")
                .expect("the deck reached the second client");
            col.remove_decks_and_child_decks(&[id])
                .expect("the engine removes the deck");
            col.close(None).expect("the engine closes the collection");
        },
        normal_sync,
    );

    assert_eq!(
        rechecked,
        (Ok(Err(Some(NO_LOSS))), true),
        "removing an empty deck adds and removes no review, card or note id, moves the stamp and \
         returns the upload to the counts"
    );
}

#[test]
fn an_edit_dated_at_or_below_the_greatest_returns_the_upload_to_the_counts() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start(
        "an_edit_dated_at_or_below_the_greatest_returns_the_upload_to_the_counts",
    );

    let rechecked = rechecked_after(
        &server,
        "one-way-edit-dated-below",
        |_| (),
        |second| {
            let mut col = engine(second);
            let dated = first_note(&col).mtime;
            let note = edit_first_note(&mut col, "edited after the count, dated as before");
            col.storage
                .db()
                .execute(
                    "update notes set mod = ?1 where id = ?2",
                    [dated.0, note.id.0],
                )
                .expect("the edit is dated at its old time, at or below the greatest row's");
            col.close(None).expect("the engine closes the collection");
        },
        normal_sync,
    );

    assert_eq!(
        rechecked,
        (Ok(Err(Some(NO_LOSS))), true),
        "a note edit dated at or below the greatest row's moves the stamp and returns the upload \
         to the counts"
    );
}

#[test]
fn a_full_upload_after_the_count_returns_the_upload_to_the_counts() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server =
        SyncServer::start("a_full_upload_after_the_count_returns_the_upload_to_the_counts");

    let rechecked = rechecked_after(&server, "one-way-full-upload", |_| (), |_| (), seed);

    assert_eq!(
        rechecked,
        (Ok(Err(Some(NO_LOSS))), true),
        "a second client's full upload of the rows it downloaded moves the stamp and returns the \
         upload to the counts"
    );
}

#[test]
fn an_unchanged_server_rechecks_equal() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("an_unchanged_server_rechecks_equal");

    let rechecked = rechecked_after(&server, "one-way-unchanged", |_| (), |_| (), |_, _| ());

    assert_eq!(
        rechecked,
        (Ok(Ok(())), false),
        "an unchanged server, downloaded by a second client and fetched again, carries the counted \
         copy's stamp, and the upload is ready"
    );
}

#[test]
fn the_stamp_reads_every_synced_tables_greatest_usn_and_the_schema() {
    let device = support::synthetic("one-way-stamp");
    answer(&device.collection, device.cards[0]);
    let col = engine(&device.collection);
    let db = col.storage.db();
    db.execute(
        "insert into tags (tag, usn, collapsed, config) values ('planted', 0, 0, null)",
        [],
    )
    .expect("the test plants a tag");
    db.execute("insert into graves (oid, type, usn) values (1, 0, 0)", [])
        .expect("the test plants a grave");
    col.close(None).expect("the engine closes the collection");
    let read = || {
        let core = open(&device)
            .id_sets()
            .expect("the core reads the stamp")
            .modified;
        let col = engine(&device.collection);
        let oracle = support::stamp(&col);
        col.close(None).expect("the engine closes the collection");
        (core, oracle)
    };
    let writes: Vec<String> = support::SYNCED_TABLES
        .iter()
        .zip(100..)
        .map(|(table, usn)| format!("update {table} set usn = {usn}"))
        .chain(std::iter::once("update col set scm = scm + 1".to_owned()))
        .collect();
    let expected: Vec<(String, bool, bool)> = writes
        .iter()
        .map(|write| (write.clone(), true, true))
        .collect();

    let mut before = read();
    let judged: Vec<(String, bool, bool)> = support::examined("stamp writes", writes)
        .into_iter()
        .map(|write| {
            let col = engine(&device.collection);
            col.storage
                .db()
                .execute(&write, [])
                .expect("the test's write runs");
            col.close(None).expect("the engine closes the collection");
            let after = read();
            let judged = (write, after.0 != before.0, after.0 == after.1);
            before = after;
            judged
        })
        .collect();

    assert_eq!(
        judged, expected,
        "(write, the core's stamp moved, it equals the engine's hash read apart from the core): \
         each synced table's greatest usn, and the schema stamp, moves the stamp"
    );
}

#[test]
fn a_server_copy_is_fetched_into_a_file_whose_collection_holds_no_row() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server =
        SyncServer::start("a_server_copy_is_fetched_into_a_file_whose_collection_holds_no_row");
    let (engine_auth, auth) = logged_in(&server);
    let donor = support::synthetic("one-way-no-row-donor");
    answer(&donor.collection, donor.cards[0]);
    let donor_ids = held(&donor.collection);
    seed(&engine_auth, &donor.collection);
    let device = support::synthetic("one-way-no-row-device");
    let copy = device.dir.join("empty.anki2");
    engine(&copy)
        .close(None)
        .expect("the engine closes the collection it created");
    assert_eq!(
        rows(held(&copy)),
        [BTreeSet::new(), BTreeSet::new(), BTreeSet::new()],
        "the planted file's collection holds no review, card or note"
    );
    let dispatcher = open(&device);

    let fetched = one_way::count(&dispatcher, &full_sync_required(), &auth, &copy)
        .map(|counted| counted.counts());

    assert!(
        fetched.is_ok(),
        "a file whose collection holds no row is fetched into: {fetched:?}"
    );
    assert_eq!(
        rows(held(&copy)),
        rows(donor_ids),
        "the copy, opened by a fresh engine, holds the server's reviews, cards and notes"
    );
}

/// A step's refusal in the engine's error shape, decoded with the engine's own schema: its kind
/// and message; `None` for a reason the engine's error shape does not carry.
fn engine_refusal(reason: Reason) -> Option<(Kind, String)> {
    match reason {
        Reason::Engine(Refusal::Engine { error }) => {
            let error = BackendError::decode(error.as_slice())
                .expect("a refusal decodes as the engine's error");
            Some((error.kind(), error.message))
        }
        Reason::Engine(
            Refusal::NotAllowed { .. } | Refusal::NeedsGesture { .. } | Refusal::NeedsAnswer { .. },
        )
        | Reason::Gesture(_)
        | Reason::OpenCollection
        | Reason::HoldsRows
        | Reason::Unheld => None,
    }
}

#[test]
fn a_copy_path_that_is_not_utf8_is_the_cores_own_refusal() {
    let device = support::synthetic("one-way-not-utf8");
    let copy = device.dir.join(OsStr::from_bytes(b"copy-\xff.anki2"));
    let dispatcher = open(&device);

    let refused = one_way::count(
        &dispatcher,
        &full_sync_required(),
        &SyncAuth::default(),
        &copy,
    )
    .map(|counted| counted.counts())
    .map_err(engine_refusal);

    assert_eq!(
        refused,
        Err(Some((
            Kind::InvalidInput,
            "the one-way sync's path is not UTF-8".to_owned()
        ))),
        "a copy path that is not UTF-8 is the core's own refusal, its kind and message byte for \
         byte, and is never rewritten into another path"
    );
    assert!(
        !copy.exists(),
        "nothing is written at a path the core refused"
    );
}

// SPEC-377 R4, A4 (ADR-388 D7): the core judges a choice path through the dispatcher's `Files`
// port, the one an adapter installed, and never through the standard library directly.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use deck_streak_engine_core::files::{FailClosed, Files, Standard};

/// A port that answers as `port` does and records each ask, in order, by its question and path.
struct Recording<P> {
    port: P,
    asks: Mutex<Vec<String>>,
}

impl<P: Files> Recording<P> {
    fn over(port: P) -> Arc<Self> {
        Arc::new(Self {
            port,
            asks: Mutex::new(Vec::new()),
        })
    }

    fn asks(&self) -> Vec<String> {
        self.asks
            .lock()
            .expect("the record is not poisoned")
            .clone()
    }

    fn record(&self, question: &str, path: &Path) {
        self.asks
            .lock()
            .expect("the record is not poisoned")
            .push(format!("{question} {}", path.display()));
    }
}

impl<P: Files> Files for Recording<P> {
    fn holds(&self, path: &Path) -> bool {
        self.record("holds", path);
        self.port.holds(path)
    }

    fn same(&self, open: &Path, path: &Path) -> bool {
        self.record("same", path);
        self.port.same(open, path)
    }
}

/// A port that names one path the open collection and answers one word for every holds.
struct Told {
    open_as: PathBuf,
    holds: bool,
}

impl Files for Told {
    fn holds(&self, _path: &Path) -> bool {
        self.holds
    }

    fn same(&self, _open: &Path, path: &Path) -> bool {
        path == self.open_as
    }
}

/// A download the owner confirmed over two empty sides, which needs no server to back up.
fn confirmed_download() -> deck_streak_engine_core::full_sync::Confirmed {
    Counted::show(Offer::of(false, true), IdSets::default(), IdSets::default())
        .confirm(Direction::Download)
        .expect("a download is offered")
}

/// An upload past its backup and snapshot check, which needs no server until its re-check.
fn checked_upload(dispatcher: &Dispatcher, copy: &Path) -> Checked {
    let confirmed = Counted::show(Offer::of(true, false), IdSets::default(), IdSets::default())
        .confirm(Direction::Upload)
        .expect("an upload is offered");
    one_way::back_up(dispatcher, confirmed, copy, copy)
        .expect("an upload's backup reads its copy")
        .snapshot_found(&SnapshotAnswer { found: true })
        .expect("the snapshot is found")
}

/// A key for an endpoint no server answers: a step that reached the network would fail there.
fn unanswered() -> SyncAuth {
    SyncAuth {
        hkey: "unanswered".to_owned(),
        endpoint: Some("http://127.0.0.1:9/".to_owned()),
        io_timeout_secs: Some(1),
    }
}

#[test]
fn the_choice_paths_are_judged_by_the_installed_files_port() {
    let device = support::synthetic("files-port-judged");
    let dispatcher = open(&device);
    let elsewhere = device.dir.join("elsewhere.anki2");
    let port = Recording::over(Told {
        open_as: elsewhere.clone(),
        holds: false,
    });
    dispatcher.install_files(port.clone());

    let backed_up = one_way::back_up(&dispatcher, confirmed_download(), &elsewhere, &elsewhere)
        .map(drop)
        .map_err(|refused| refused.reason);
    let counted = one_way::count(
        &dispatcher,
        &full_sync_required(),
        &unanswered(),
        &elsewhere,
    )
    .map(drop);
    let asked = format!("same {}", elsewhere.display());
    assert_eq!(
        (backed_up, counted, port.asks(), elsewhere.exists()),
        (
            Err(Reason::OpenCollection),
            Err(Reason::OpenCollection),
            vec![asked.clone(), asked],
            false
        ),
        "a path the port names as the open collection is refused, and nothing is written there"
    );
}

#[test]
fn a_path_the_files_port_holds_is_read_for_rows() {
    let full = support::synthetic("files-port-holds-full");
    let full_before = held(&full.collection);
    let device = support::synthetic("files-port-holds-device");
    let dispatcher = open(&device);
    let mut judged = Vec::new();
    for holds in [true, false] {
        let port = Recording::over(Told {
            open_as: device.dir.join("never.anki2"),
            holds,
        });
        dispatcher.install_files(port.clone());
        let reason = one_way::back_up(
            &dispatcher,
            confirmed_download(),
            &full.collection,
            &full.collection,
        )
        .map(drop)
        .map_err(|refused| match refused.reason {
            Reason::Engine(_) => "the engine refused the write".to_owned(),
            reason => format!("{reason:?}"),
        });
        judged.push((holds, reason, port.asks()));
    }
    let asks = vec![
        format!("same {}", full.collection.display()),
        format!("holds {}", full.collection.display()),
    ];
    assert_eq!(
        judged,
        [
            (true, Err("HoldsRows".to_owned()), asks.clone()),
            (false, Err("the engine refused the write".to_owned()), asks)
        ],
        "the port's holds decides whether the file is read for rows"
    );
    assert_eq!(rows(held(&full.collection)), rows(full_before));
}

#[test]
fn the_open_collection_by_another_spelling_is_refused() {
    let device = support::synthetic("files-port-spelling");
    let dispatcher = open(&device);
    let spelled = device
        .dir
        .join("collection.media")
        .join("..")
        .join("collection.anki2");
    assert!(
        Standard.same(&device.collection, &spelled) && Standard.holds(&spelled),
        "the standard port finds the open collection by another spelling"
    );
    let port = Recording::over(Standard);
    dispatcher.install_files(port.clone());
    let refused = one_way::back_up(&dispatcher, confirmed_download(), &spelled, &spelled)
        .map(drop)
        .map_err(|refused| refused.reason);
    assert_eq!(
        (refused, port.asks()),
        (
            Err(Reason::OpenCollection),
            vec![format!("same {}", spelled.display())]
        )
    );
}

#[test]
fn an_uninstalled_browser_port_refuses_every_choice_path() {
    assert!(
        FailClosed.holds(Path::new("/any.anki2"))
            && FailClosed.same(Path::new("/open.anki2"), Path::new("/any.anki2")),
        "the fail-closed port holds every path and names each the open collection"
    );
    let device = support::synthetic("files-port-fail-closed");
    let dispatcher = open(&device);
    let copy = device.dir.join("copy.anki2");
    let checked = checked_upload(&dispatcher, &copy);
    dispatcher.install_files(Arc::new(FailClosed));
    let fresh = device.dir.join("fresh.anki2");

    let counted =
        one_way::count(&dispatcher, &full_sync_required(), &unanswered(), &fresh).map(drop);
    let backed_up = one_way::back_up(&dispatcher, confirmed_download(), &fresh, &fresh)
        .map(drop)
        .map_err(|refused| refused.reason);
    let rechecked = one_way::recheck(&dispatcher, checked, &unanswered(), &fresh)
        .map(drop)
        .map_err(|refused| refused.reason);
    let refusals = support::examined("choice path(s) judged", vec![counted, backed_up, rechecked]);
    assert_eq!(
        (refusals, fresh.exists()),
        (vec![Err(Reason::OpenCollection); 3], false),
        "every choice path is refused before anything is written"
    );
}
