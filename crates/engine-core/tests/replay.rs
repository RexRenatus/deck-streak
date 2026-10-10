//! The replay of a deck set's review history into stock-field values (SPEC-386 A11 to A21).
//!
//! Each test builds its collection with the engine's own API through a test-only engine handle:
//! the reviews by the engine's own answer, Forget and set-due-date, and the undo by the owner's
//! gesture on a web dispatcher. The expected values are typed from the pinned scheduler's own test
//! (`src/inference.rs:346-349` at its pinned revision: `FSRS::default()`, no previous state,
//! retention 0.9, 0 days elapsed) or read from the engine, never computed by the core.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anki::card::CardId;
use anki::collection::{Collection, CollectionBuilder};
use anki::decks::DeckId;
use anki::notes::NoteId;
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::sync::login::{SyncAuth, sync_login};
use anki::timestamp::TimestampMillis;
use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::cards::{self, Card, UpdateCardsRequest};
use anki_proto::collection::{CloseCollectionRequest, OpenCollectionRequest, UndoStatus};
use anki_proto::scheduler::{GetQueuedCardsRequest, QueuedCards, card_answer};
use anki_proto::sync::SyncCollectionResponse;
use anki_proto::sync::sync_collection_response::ChangesRequired;
use deck_streak_engine_core::answer::{Grade, OwnerAnswer};
use deck_streak_engine_core::dispatch::{Dispatcher, Read, Refusal};
use deck_streak_engine_core::gesture::{OwnerGesture, Target};
use deck_streak_engine_core::replay::{CardReplay, Schedule};
use deck_streak_engine_core::table::{ExemptWrite, Transport};
use deck_streak_engine_core::undo_answer::Recorded;
use prost::Message;
use serde_json::{Map, Value};
use support::sync_server::{self, PASSWORD, SERVER, SyncServer, USERNAME};

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `BackendCollectionService.CloseCollection`.
const CLOSE_COLLECTION: (u32, u32) = (3, 1);
/// `CollectionService.GetUndoStatus`.
const GET_UNDO_STATUS: (u32, u32) = (3, 7);
/// `CardsService.GetCard`.
const GET_CARD: (u32, u32) = (5, 0);
/// `CardsService.UpdateCards`.
const UPDATE_CARDS: (u32, u32) = (5, 1);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);

/// The default deck, where every built card starts.
const HOME: i64 = 1;
/// Milliseconds in a day.
const DAY_MS: i64 = 86_400_000;
/// How far each built collection's creation is moved back, so its day count is not 0.
const DAYS_BACK: i64 = 10;
/// The engine's card types, as its `CardType` numbers them.
const LEARN_TYPE: i64 = 1;
/// A review card.
const REVIEW_TYPE: i64 = 2;
/// The desired retention every replay here is asked for.
const RETENTION: f32 = 0.9;
/// A maximum interval no replay here reaches.
const MAX_IVL: u32 = 36_500;
/// A first Good's stock stability and difficulty: its interval at 0.9 and its difficulty
/// (`src/inference.rs:348` at the pinned revision).
const FIRST_GOOD: (f32, f32) = (4.777_724_7, 3.530_724_3);

/// A collection built for one test, closed.
struct Built {
    /// The test's own directory.
    dir: PathBuf,
    /// The collection file.
    collection: PathBuf,
    /// The cards, one per Basic note, in the order they were added.
    cards: Vec<i64>,
}

/// The test's own engine handle on a closed collection file.
fn engine(collection: &Path) -> Collection {
    CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the test's collection")
}

/// `count` Basic notes in the default deck, so `count` cards, with the engine's own API, and the
/// collection's creation moved back `DAYS_BACK` days as the core's day test moves it.
fn built(test: &str, count: usize) -> Built {
    let dir = support::scratch("engine-core-replay", test);
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let collection = dir.join("collection.anki2");
    let mut col = engine(&collection);
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let cards = (0..count)
        .map(|index| {
            let mut note = basic.new_note();
            note.set_field(0, format!("replay front {index}"))
                .expect("the front is set");
            note.set_field(1, format!("replay back {index}"))
                .expect("the back is set");
            col.add_note(&mut note, DeckId(HOME))
                .expect("the engine adds the note to the default deck");
            col.storage
                .db()
                .query_row("select id from cards where nid = ?", [note.id.0], |row| {
                    row.get(0)
                })
                .expect("the note has one card")
        })
        .collect();
    col.storage
        .db()
        .execute("update col set crt = crt - ?", [DAYS_BACK * 86_400])
        .expect("the collection's creation moves back");
    col.close(None).expect("the engine closes the collection");
    Built {
        dir,
        collection,
        cards,
    }
}

fn now_millis() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    i64::try_from(millis).expect("the clock's milliseconds fit an i64")
}

/// Answers `card` with `rating` at `at`, as the engine's own reviewer does: the review-log row's
/// id is `at`, and the card is scheduled from the engine's today.
fn answer(col: &mut Collection, card: i64, rating: Rating, at: i64) {
    let states = col
        .get_scheduling_states(CardId(card))
        .expect("the engine reads the card's next states");
    let new_state = match rating {
        Rating::Again => states.again,
        Rating::Hard => states.hard,
        Rating::Good => states.good,
        Rating::Easy => states.easy,
    };
    col.answer_card(&mut CardAnswer {
        card_id: CardId(card),
        current_state: states.current,
        new_state,
        rating,
        answered_at: TimestampMillis(at),
        milliseconds_taken: 1000,
        custom_data: None,
        from_queue: false,
    })
    .expect("the engine answers the card");
}

/// The id of `card`'s newest review-log row.
fn newest_row(col: &Collection, card: i64) -> i64 {
    col.storage
        .db()
        .query_row("select max(id) from revlog where cid = ?", [card], |row| {
            row.get(0)
        })
        .expect("the card has a review-log row")
}

/// `card`'s review-log rows as the engine wrote them: each row's kind and ease factor, by id.
fn logged(col: &Collection, card: i64) -> Vec<(i64, i64)> {
    let db = col.storage.db();
    let mut statement = db
        .prepare("select type, factor from revlog where cid = ? order by id")
        .expect("the test's read prepares");
    statement
        .query_map([card], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("the test's read runs")
        .map(|row| row.expect("a row reads"))
        .collect()
}

/// `card`'s type, due and interval, read by a fresh engine on the file.
fn scheduled(collection: &Path, card: i64) -> (i64, i32, i32) {
    let col = engine(collection);
    let read = col
        .storage
        .db()
        .query_row(
            "select type, due, ivl from cards where id = ?",
            [card],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("the card reads");
    col.close(None).expect("the engine closes the collection");
    read
}

fn text(path: &Path) -> String {
    path.to_str().expect("a scratch path is UTF-8").to_owned()
}

/// The encoded `OpenCollectionRequest` for a built collection.
fn open_request(built: &Built) -> Vec<u8> {
    OpenCollectionRequest {
        collection_path: text(&built.collection),
        media_folder_path: text(&built.dir.join("collection.media")),
        media_db_path: text(&built.dir.join("collection.media.db")),
    }
    .encode_to_vec()
}

/// A web dispatcher with `built`'s collection open.
fn opened(built: &Built) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Web, &[]).expect("the engine starts from the default init");
    dispatcher
        .run(OPEN_COLLECTION.0, OPEN_COLLECTION.1, &open_request(built))
        .expect("the web dispatcher opens the collection");
    dispatcher
}

/// Closes the dispatcher's collection.
fn close(dispatcher: &Dispatcher) {
    dispatcher
        .run(
            CLOSE_COLLECTION.0,
            CLOSE_COLLECTION.1,
            &CloseCollectionRequest::default().encode_to_vec(),
        )
        .expect("the web dispatcher closes the collection");
}

/// The replay of `decks` under the pinned defaults, by card.
fn replayed(dispatcher: &Dispatcher, decks: &[i64], max_ivl: u32) -> BTreeMap<i64, CardReplay> {
    dispatcher
        .replay(decks, &[], RETENTION, max_ivl)
        .expect("the replay runs")
        .into_iter()
        .map(|entry| (entry.card, entry))
        .collect()
}

/// The replay of `decks` by a dispatcher opened for it and closed after it.
fn replay_of(built: &Built, decks: &[i64], max_ivl: u32) -> BTreeMap<i64, CardReplay> {
    let dispatcher = opened(built);
    let replay = replayed(&dispatcher, decks, max_ivl);
    close(&dispatcher);
    replay
}

/// The cards a replay has an entry for.
fn cards_of(replay: &BTreeMap<i64, CardReplay>) -> BTreeSet<i64> {
    replay.keys().copied().collect()
}

/// A replay's stock stability and difficulty, as bits, so equal values compare exactly.
fn values(entry: &CardReplay) -> (u32, u32) {
    (entry.stability.to_bits(), entry.difficulty.to_bits())
}

#[test]
fn a_deck_sets_replay_reads_only_its_home_decks_cards() {
    let built = built("replay-home-decks", 3);
    let (home, moved, filtered) = (built.cards[0], built.cards[1], built.cards[2]);
    let first = now_millis() - DAY_MS;
    let mut col = engine(&built.collection);
    for (offset, card) in (0..).zip(&built.cards) {
        answer(&mut col, *card, Rating::Good, first + offset);
    }
    let other = col
        .get_or_create_normal_deck("Replay other")
        .expect("the engine adds a deck")
        .id
        .0;
    let db = col.storage.db();
    db.execute("update cards set did = ?1 where id = ?2", [other, moved])
        .expect("the card moves to the other deck");
    db.execute(
        "update cards set did = ?1, odid = ?2 where id = ?3",
        [other, HOME, filtered],
    )
    .expect("the card sits in the other deck with the default deck as its home");
    col.close(None).expect("the engine closes the collection");

    assert_eq!(
        cards_of(&replay_of(&built, &[HOME], MAX_IVL)),
        BTreeSet::from([home, filtered]),
        "the default deck's replay holds its own card and the card whose home it is"
    );
    assert_eq!(
        cards_of(&replay_of(&built, &[other], MAX_IVL)),
        BTreeSet::from([moved]),
        "the other deck's replay holds only the card whose home it is"
    );
    assert_eq!(
        cards_of(&replay_of(&built, &[HOME, other], MAX_IVL)),
        BTreeSet::from([home, moved, filtered]),
        "a set of both decks holds every card"
    );
    assert_eq!(
        replay_of(&built, &[], MAX_IVL),
        BTreeMap::new(),
        "an empty deck set gives an empty result"
    );
}

#[test]
fn a_deleted_cards_reviews_are_not_replayed() {
    let built = built("replay-deleted-card", 2);
    let (kept, deleted) = (built.cards[0], built.cards[1]);
    let first = now_millis() - DAY_MS;
    let mut col = engine(&built.collection);
    answer(&mut col, kept, Rating::Good, first);
    answer(&mut col, deleted, Rating::Good, first + 1);
    let note: i64 = col
        .storage
        .db()
        .query_row("select nid from cards where id = ?", [deleted], |row| {
            row.get(0)
        })
        .expect("the card's note reads");
    col.remove_notes(&[NoteId(note)])
        .expect("the engine deletes the note");
    let rows = logged(&col, deleted);
    col.close(None).expect("the engine closes the collection");

    assert_eq!(
        cards_of(&replay_of(&built, &[HOME], MAX_IVL)),
        BTreeSet::from([kept]),
        "only the card that still exists is replayed"
    );
    assert_eq!(
        rows.len(),
        1,
        "the engine keeps the deleted card's review row, so the read itself must leave it"
    );
}

#[test]
fn a_card_with_no_kept_review_has_no_entry() {
    let built = built("replay-no-kept-review", 3);
    let (forgotten, unseen, answered) = (built.cards[0], built.cards[1], built.cards[2]);
    let first = now_millis() - DAY_MS;
    let mut col = engine(&built.collection);
    answer(&mut col, forgotten, Rating::Good, first);
    col.reschedule_cards_as_new(&[CardId(forgotten)], true, false, false, None)
        .expect("the engine forgets the card");
    answer(&mut col, answered, Rating::Good, first + 1);
    let rows = [logged(&col, forgotten).len(), logged(&col, unseen).len()];
    col.close(None).expect("the engine closes the collection");

    assert_eq!(
        cards_of(&replay_of(&built, &[HOME], MAX_IVL)),
        BTreeSet::from([answered]),
        "only the card with a kept review has an entry"
    );
    assert_eq!(
        rows,
        [2, 0],
        "the forgotten card has its review and its reset row, and the unseen card has none"
    );
}

#[test]
fn a_review_cards_due_is_its_last_review_day_plus_its_interval() {
    let built = built("replay-review-schedule", 2);
    let (easy, again) = (built.cards[0], built.cards[1]);
    // Each answer is made at the clock's now under the collection's default rollover, as the
    // engine's own answering test makes it (rslib/src/scheduler/answering/mod.rs:651 and the
    // rollover of rslib/src/scheduler/mod.rs:58-70 at the engine's pin), so the engine's due of the
    // Easy card, less its interval, is the engine day of the review.
    let mut col = engine(&built.collection);
    answer(&mut col, easy, Rating::Easy, now_millis());
    answer(&mut col, again, Rating::Again, now_millis());
    col.set_due_date(&[CardId(again)], "0", None)
        .expect("the engine makes the card a review card due today");
    col.close(None).expect("the engine closes the collection");
    let (easy_type, due, ivl) = scheduled(&built.collection, easy);
    let day = due - ivl;

    let replay = replay_of(&built, &[HOME], MAX_IVL);
    assert_eq!(
        cards_of(&replay),
        BTreeSet::from([easy, again]),
        "both cards are replayed"
    );
    // A first Easy's interval at 0.9 is 53.869392 days, so 54; a first Again's is 3.8275626e-5
    // days, which rounds to 0 and is held at 1 (src/inference.rs:346 and :349 at the pinned
    // revision).
    assert_eq!(
        (replay[&easy].schedule, replay[&again].schedule),
        (
            Some(Schedule {
                ivl: 54,
                due: day + 54
            }),
            Some(Schedule {
                ivl: 1,
                due: day + 1
            })
        ),
        "a review card is due its last review's engine day plus its rounded interval, at least 1"
    );
    assert_eq!(
        replay_of(&built, &[HOME], 3)[&easy].schedule,
        Some(Schedule {
            ivl: 3,
            due: day + 3
        }),
        "the interval is held to the maximum"
    );
    assert_eq!(
        (easy_type, scheduled(&built.collection, again).0),
        (REVIEW_TYPE, REVIEW_TYPE),
        "the engine graduated the Easy card and set the other's due date, so both are review cards"
    );
    assert!(
        day >= i32::try_from(DAYS_BACK).expect("a small day count"),
        "the engine day {day} is the moved-back collection's"
    );
}

#[test]
fn a_learning_card_keeps_its_due_and_interval() {
    let built = built("replay-learning-card", 1);
    let card = built.cards[0];
    let mut col = engine(&built.collection);
    answer(&mut col, card, Rating::Good, now_millis() - DAY_MS);
    col.close(None).expect("the engine closes the collection");

    let replay = replay_of(&built, &[HOME], MAX_IVL);
    assert_eq!(
        cards_of(&replay),
        BTreeSet::from([card]),
        "the card is replayed"
    );
    assert_eq!(
        (values(&replay[&card]), replay[&card].schedule),
        ((FIRST_GOOD.0.to_bits(), FIRST_GOOD.1.to_bits()), None),
        "a learning card carries its stock stability and difficulty, and no schedule"
    );
    assert_eq!(
        scheduled(&built.collection, card).0,
        LEARN_TYPE,
        "the engine left the card in learning"
    );
}

/// Every row of every table of `collection`, each cell as the database holds it, read by a fresh
/// engine on the file.
fn every_row(collection: &Path) -> Vec<(String, Vec<Vec<String>>)> {
    let col = engine(collection);
    let rows = {
        let db = col.storage.db();
        let mut statement = db
            .prepare("select name from sqlite_master where type = 'table' order by name")
            .expect("the table list prepares");
        let tables: Vec<String> = statement
            .query_map([], |row| row.get(0))
            .expect("the table list runs")
            .map(|name| name.expect("a table name reads"))
            .collect();
        support::examined("tables", tables)
            .into_iter()
            .map(|table| {
                let mut statement = db
                    .prepare(&format!("select * from {table} order by 1"))
                    .expect("a table's read prepares");
                let columns = statement.column_count();
                let rows = statement
                    .query_map([], |row| {
                        (0..columns)
                            .map(|index| row.get_ref(index).map(|cell| format!("{cell:?}")))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .expect("a table's read runs")
                    .map(|row| row.expect("a row reads"))
                    .collect();
                (table, rows)
            })
            .collect()
    };
    col.close(None).expect("the engine closes the collection");
    rows
}

/// The engine's undo status, through the web dispatcher.
fn undo_status(dispatcher: &Dispatcher) -> UndoStatus {
    dispatcher
        .run(GET_UNDO_STATUS.0, GET_UNDO_STATUS.1, &[])
        .map(|bytes| UndoStatus::decode(bytes.as_slice()).expect("the undo status decodes"))
        .expect("the undo status is admitted on the web")
}

#[test]
fn the_replay_moves_no_stamp_no_undo_and_no_row() {
    let built = built("replay-writes-nothing", 2);
    let reviewed = built.cards[0];
    let mut col = engine(&built.collection);
    answer(&mut col, reviewed, Rating::Good, now_millis() - DAY_MS);
    col.close(None).expect("the engine closes the collection");
    close(&opened(&built));
    let before = every_row(&built.collection);

    let replay = replay_of(&built, &[HOME], MAX_IVL);
    assert_eq!(
        cards_of(&replay),
        BTreeSet::from([reviewed]),
        "the reviewed card is replayed"
    );
    assert_eq!(
        every_row(&built.collection),
        before,
        "an open, a replay and a close leave every row of every table as an open and a close do"
    );

    let dispatcher = opened(&built);
    let (studied, _) = answer_head(&dispatcher);
    let status = undo_status(&dispatcher);
    let answered = replayed(&dispatcher, &[HOME], MAX_IVL);
    let after = undo_status(&dispatcher);
    close(&dispatcher);
    assert_eq!(
        cards_of(&answered),
        BTreeSet::from([reviewed, studied]),
        "the replay holds the answer just made"
    );
    assert!(
        !status.undo.is_empty(),
        "the answer left an undo step: {status:?}"
    );
    assert_eq!(after, status, "the replay leaves the engine's undo status");
}

/// The engine's own login, normal sync and full transfers, through the test's handle.
fn engine_client<Client: Default>() -> Client {
    Client::default()
}

/// The synthetic account logged in at `server` by the engine's own login.
fn logged_in(server: &SyncServer) -> SyncAuth {
    let endpoint = server.endpoint();
    let login = sync_server::runtime()
        .block_on(sync_login(
            USERNAME,
            PASSWORD,
            Some(endpoint.to_owned()),
            engine_client(),
        ))
        .unwrap_or_else(|error| panic!("the engine's own login: {error}"));
    SyncAuth {
        hkey: login.hkey,
        endpoint: Some(endpoint.parse().expect("the server's endpoint is a URL")),
        io_timeout_secs: None,
    }
}

/// The server's collection replaced by `collection`'s, by the engine's own full upload.
fn seed(auth: &SyncAuth, collection: &Path) {
    sync_server::runtime()
        .block_on(engine(collection).full_upload(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own upload: {error}"));
}

/// The engine's own normal sync of `collection`, which needs no full sync.
fn normal_sync(auth: &SyncAuth, collection: &Path) {
    let mut col = engine(collection);
    let output = sync_server::runtime()
        .block_on(col.normal_sync(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own normal sync: {error}"));
    col.close(None).expect("the engine closes the collection");
    assert_eq!(
        SyncCollectionResponse::from(output).required(),
        ChangesRequired::NoChanges,
        "the device's change reaches the server by a normal sync"
    );
}

/// A new collection at `collection` holding the server's, by the engine's own full download.
fn download_to(auth: &SyncAuth, collection: &Path) {
    sync_server::runtime()
        .block_on(engine(collection).full_download(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own download: {error}"));
}

/// Every column of `card` by name, each cell as the database holds it, and its data's keys.
fn card_row(collection: &Path, card: i64) -> (BTreeMap<String, String>, Map<String, Value>) {
    let col = engine(collection);
    let read = {
        let db = col.storage.db();
        let mut statement = db
            .prepare("select * from cards where id = ?")
            .expect("the card's read prepares");
        let names: Vec<String> = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let (cells, data): (Vec<String>, String) = statement
            .query_row([card], |row| {
                let cells = (0..names.len())
                    .map(|index| row.get_ref(index).map(|cell| format!("{cell:?}")))
                    .collect::<Result<_, _>>()?;
                Ok((cells, row.get("data")?))
            })
            .expect("the card reads");
        let data = serde_json::from_str::<Map<String, Value>>(&data)
            .expect("the card's data is a JSON object");
        (names.into_iter().zip(cells).collect(), data)
    };
    col.close(None).expect("the engine closes the collection");
    read
}

/// `entry`'s stock fields written into its card by the engine's own card update, through a
/// test-only engine: the write the scheduler switch will make, which no core path makes.
fn written(built: &Built, entry: &CardReplay) {
    let backend = anki::backend::init_backend(&[]).expect("the test's engine starts");
    backend
        .run_service_method(OPEN_COLLECTION.0, OPEN_COLLECTION.1, &open_request(built))
        .expect("the test's engine opens the collection");
    let reply = backend
        .run_service_method(
            GET_CARD.0,
            GET_CARD.1,
            &cards::CardId { cid: entry.card }.encode_to_vec(),
        )
        .expect("the engine reads the card");
    let mut card = Card::decode(reply.as_slice()).expect("the card decodes");
    entry.stock_fields(&mut card);
    backend
        .run_service_method(
            UPDATE_CARDS.0,
            UPDATE_CARDS.1,
            &UpdateCardsRequest {
                cards: vec![card],
                skip_undo_entry: true,
            }
            .encode_to_vec(),
        )
        .expect("the engine updates the card");
    backend
        .run_service_method(
            CLOSE_COLLECTION.0,
            CLOSE_COLLECTION.1,
            &CloseCollectionRequest::default().encode_to_vec(),
        )
        .expect("the test's engine closes the collection");
}

/// `value` rounded to `places` decimals as the engine rounds a card's data before it stores it
/// (rslib/src/storage/card/data.rs:110-113 at the engine's pin).
fn stored(value: f32, places: i32) -> u32 {
    let factor = 10_f32.powi(places);
    ((value * factor).round() / factor).to_bits()
}

/// A data key's number, as the f32 the engine wrote, in bits.
fn data_bits(data: &Map<String, Value>, key: &str) -> Option<u32> {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the engine wrote an f32, and its shortest decimal reads back to that f32"
    )]
    data.get(key)
        .and_then(Value::as_f64)
        .map(|value| (value as f32).to_bits())
}

#[test]
fn the_stock_fields_survive_a_sync_round_trip_alone() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("the_stock_fields_survive_a_sync_round_trip_alone");
    let auth = logged_in(&server);
    let built = built("replay-sync-round-trip", 1);
    let card = built.cards[0];
    let mut col = engine(&built.collection);
    answer(&mut col, card, Rating::Easy, now_millis() - DAY_MS);
    col.close(None).expect("the engine closes the collection");
    seed(&auth, &built.collection);

    let replay = replay_of(&built, &[HOME], MAX_IVL);
    assert_eq!(
        cards_of(&replay),
        BTreeSet::from([card]),
        "the card is replayed"
    );
    let entry = replay[&card];
    let (columns, data) = card_row(&built.collection, card);
    written(&built, &entry);
    let (written_columns, _) = card_row(&built.collection, card);
    normal_sync(&auth, &built.collection);
    let downloaded = built.dir.join("downloaded.anki2");
    download_to(&auth, &downloaded);
    let (read_columns, read_data) = card_row(&downloaded, card);

    let schedule = entry.schedule.map(|schedule| {
        (
            format!("Integer({})", schedule.ivl),
            format!("Integer({})", schedule.due),
        )
    });
    assert_eq!(
        (
            data_bits(&read_data, "s"),
            data_bits(&read_data, "d"),
            Some((read_columns["ivl"].clone(), read_columns["due"].clone())),
        ),
        (
            Some(stored(entry.stability, 4)),
            Some(stored(entry.difficulty, 3)),
            schedule,
        ),
        "the card read back after the round trip holds the projection, as the engine stores it"
    );
    let others = |row: &BTreeMap<String, String>| -> BTreeMap<String, String> {
        row.iter()
            .filter(|(name, _)| !["ivl", "due", "data", "mod", "usn"].contains(&name.as_str()))
            .map(|(name, cell)| (name.clone(), cell.clone()))
            .collect()
    };
    assert_eq!(
        (others(&read_columns).len(), others(&read_columns)),
        (13, others(&columns)),
        "every other column of the card is unchanged"
    );
    let rest = |data: &Map<String, Value>| -> Map<String, Value> {
        data.iter()
            .filter(|(key, _)| !["s", "d"].contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    };
    assert_eq!(
        (rest(&read_data).contains_key("lrt"), rest(&read_data)),
        (true, rest(&data)),
        "every other data key of the card is unchanged"
    );
    assert_eq!(
        read_columns["mod"], written_columns["mod"],
        "the card's modified stamp is the update's"
    );
}

#[test]
fn the_engines_forget_reads_as_a_reset_and_set_due_does_not() {
    let built = built("replay-forget-and-set-due", 3);
    let (moved, plain, forgotten) = (built.cards[0], built.cards[1], built.cards[2]);
    let first = now_millis() - 2 * DAY_MS;
    let mut col = engine(&built.collection);
    // The moved card: Good, the engine's set-due-date, Good. The plain card: two Goods at the same
    // gap. The forgotten card: Good, the engine's Forget, Good.
    answer(&mut col, moved, Rating::Good, first);
    col.set_due_date(&[CardId(moved)], "0", None)
        .expect("the engine sets the card's due date");
    let set_due = newest_row(&col, moved);
    answer(&mut col, moved, Rating::Good, set_due + 1);
    answer(&mut col, plain, Rating::Good, first + 1);
    answer(&mut col, plain, Rating::Good, set_due + 2);
    answer(&mut col, forgotten, Rating::Good, first + 2);
    col.reschedule_cards_as_new(&[CardId(forgotten)], true, false, false, None)
        .expect("the engine forgets the card");
    let reset = newest_row(&col, forgotten);
    answer(&mut col, forgotten, Rating::Good, reset + 1);
    let rows = [logged(&col, moved)[1], logged(&col, forgotten)[1]];
    col.close(None).expect("the engine closes the collection");

    let replay = replay_of(&built, &[HOME], MAX_IVL);
    assert_eq!(
        cards_of(&replay),
        BTreeSet::from([moved, plain, forgotten]),
        "every card is replayed"
    );
    assert_eq!(
        (values(&replay[&moved]), replay[&moved].last_review),
        (values(&replay[&plain]), set_due + 1),
        "the set-due-date row is not a reset, and is not a review: the moved card replays as the \
         plain card's two Goods"
    );
    assert_eq!(
        values(&replay[&forgotten]),
        (FIRST_GOOD.0.to_bits(), FIRST_GOOD.1.to_bits()),
        "the Forget row is a reset: the forgotten card replays as its one Good after it"
    );
    assert_ne!(
        values(&replay[&moved]),
        values(&replay[&forgotten]),
        "two Goods and one Good replay apart"
    );
    assert_eq!(
        (rows[0].0, rows[0].1 != 0, rows[1]),
        (4, true, (4, 0)),
        "the engine logs its set-due-date as kind 4 with a factor, and its Forget as kind 4 with 0"
    );
}

#[test]
fn the_last_kept_review_is_the_engines_last_review_time() {
    let built = built("replay-last-review", 1);
    let card = built.cards[0];
    let mut col = engine(&built.collection);
    answer(&mut col, card, Rating::Good, now_millis() - DAY_MS);
    col.set_due_date(&[CardId(card)], "0", None)
        .expect("the engine sets the card's due date");
    let later = newest_row(&col, card);
    col.close(None).expect("the engine closes the collection");
    let (_, data) = card_row(&built.collection, card);

    let replay = replay_of(&built, &[HOME], MAX_IVL);
    assert_eq!(
        cards_of(&replay),
        BTreeSet::from([card]),
        "the card is replayed"
    );
    let last = replay[&card].last_review;
    assert_eq!(
        Some(last / 1000),
        data.get("lrt").and_then(Value::as_i64),
        "the last kept review, in seconds, is the engine's own last-review time"
    );
    assert!(
        later > last,
        "the set-due-date row {later} is later than the last kept review {last}, and is not it"
    );
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
        .expect("a new card is queued");
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
        .expect("the owner's press records the answer");
    let newest = dispatcher
        .read(Read::NewestReview)
        .expect("the fixed read runs");
    let review = serde_json::from_slice::<Value>(&newest)
        .expect("the engine replies with JSON rows")
        .pointer("/0/0")
        .and_then(Value::as_i64)
        .expect("the newest review row has an id");
    (
        card,
        Recorded {
            status: Some(undo_status(dispatcher)),
            review,
        },
    )
}

#[test]
fn an_undone_answer_leaves_the_replay_as_before_it() {
    let built = built("replay-undone-answer", 2);
    let (reviewed, studied) = (built.cards[0], built.cards[1]);
    let mut col = engine(&built.collection);
    answer(&mut col, reviewed, Rating::Easy, now_millis() - DAY_MS);
    col.close(None).expect("the engine closes the collection");

    let dispatcher = opened(&built);
    let before = replayed(&dispatcher, &[HOME], MAX_IVL);
    let (card, recorded) = answer_head(&dispatcher);
    let answered = replayed(&dispatcher, &[HOME], MAX_IVL);
    let gesture = OwnerGesture::from_tap(ExemptWrite::Undo, Target::Card(card))
        .expect("an undo takes a card");
    let undone = dispatcher.run_exempt(gesture, &recorded.encode_to_vec());
    let after = replayed(&dispatcher, &[HOME], MAX_IVL);
    close(&dispatcher);

    assert_eq!(
        cards_of(&before),
        BTreeSet::from([reviewed]),
        "the reviewed card is replayed before the answer"
    );
    assert_eq!(
        (card, cards_of(&answered)),
        (studied, BTreeSet::from([reviewed, studied])),
        "the answer of the new card at the queue's head enters the replay"
    );
    assert!(undone.is_ok(), "the owner's undo is admitted: {undone:?}");
    assert_eq!(
        after, before,
        "after the undo the replay is as before the answer"
    );
}

#[test]
fn two_replays_of_one_collection_are_equal() {
    let built = built("replay-twice", 2);
    let (first, second) = (built.cards[0], built.cards[1]);
    let start = now_millis() - 2 * DAY_MS;
    let mut col = engine(&built.collection);
    answer(&mut col, first, Rating::Good, start);
    answer(&mut col, second, Rating::Easy, start + DAY_MS);
    answer(&mut col, first, Rating::Hard, start + DAY_MS + 1);
    col.close(None).expect("the engine closes the collection");

    let dispatcher = opened(&built);
    let once = replayed(&dispatcher, &[HOME], MAX_IVL);
    let twice = replayed(&dispatcher, &[HOME], MAX_IVL);
    close(&dispatcher);
    assert_eq!(
        cards_of(&once),
        BTreeSet::from([first, second]),
        "both cards are replayed"
    );
    assert_eq!(
        twice, once,
        "a second replay of the same collection is equal"
    );
}

/// The pinned revision's FSRS-7 default parameters, typed by hand from `src/inference_v7.rs:1-5`
/// at its pin: a vector of the one length a preset's fitted vector has.
const DEFAULT_PARAMETERS: [f32; 34] = [
    0.1104, 2.2395, 3.9221, 11.7841, 6.1686, 0.6457, 3.6807, 1.9795, 0.0, 1.3826, 0.7024, 0.5999,
    0.8146, 0.6398, 1.0, 1.3207, 0.6707, 3.8668, 0.4416, 0.0934, 1.8631, 0.6162, 1.0869, 0.1567,
    0.0801, 0.2421, 0.9464, 0.1433, 0.7145, 0.0, 0.5667, 0.3734, 0.5333, 0.3048,
];

/// The error kind of a refusal the engine's error shape carries, or `None` for any other refusal.
fn kind(refusal: &Refusal) -> Option<i32> {
    match refusal {
        Refusal::Engine { error } => BackendError::decode(error.as_slice())
            .ok()
            .map(|error| error.kind),
        _ => None,
    }
}

#[test]
fn a_parameter_vector_of_another_length_is_refused_whole() {
    let built = built("replay-parameter-length", 1);
    let card = built.cards[0];
    let mut col = engine(&built.collection);
    answer(&mut col, card, Rating::Good, now_millis() - DAY_MS);
    col.close(None).expect("the engine closes the collection");

    // The lengths the scheduler itself reads as an older model's (17, 19 and 21) and lengths it
    // refuses; each is the default vector's values, cycled.
    let lengths = support::examined("parameter length(s)", vec![1, 17, 19, 21, 33, 35]);
    let dispatcher = opened(&built);
    let defaults = replayed(&dispatcher, &[HOME], MAX_IVL);
    let typed = dispatcher
        .replay(&[HOME], &DEFAULT_PARAMETERS, RETENTION, MAX_IVL)
        .map(|replay| {
            replay
                .into_iter()
                .map(|entry| (entry.card, entry))
                .collect::<BTreeMap<_, _>>()
        });
    let refused: Vec<(usize, Option<i32>)> = lengths
        .iter()
        .map(|&length| {
            let parameters: Vec<f32> = DEFAULT_PARAMETERS
                .into_iter()
                .cycle()
                .take(length)
                .collect();
            let reply = dispatcher.replay(&[HOME], &parameters, RETENTION, MAX_IVL);
            (length, reply.err().as_ref().and_then(kind))
        })
        .collect();
    close(&dispatcher);

    assert_eq!(
        cards_of(&defaults),
        BTreeSet::from([card]),
        "the card is replayed under the empty vector"
    );
    assert_eq!(
        typed,
        Ok(defaults),
        "the 34 default values replay as the empty vector does"
    );
    assert_eq!(
        refused,
        lengths
            .iter()
            .map(|&length| (length, Some(Kind::InvalidInput as i32)))
            .collect::<Vec<_>>(),
        "a vector neither empty nor 34 long is refused whole, in the engine's error shape"
    );
}
