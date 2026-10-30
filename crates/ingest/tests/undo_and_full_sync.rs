//! The undo and full-sync probes (SPEC-342 R10 and R11; ADR-353 D3 and D4).
//!
//! Each probe measures the engine at the workspace's pin, against the engine's own sync server,
//! which the test starts as a child of itself. No code of this workspace runs in a probe: each one
//! calls the engine's own API, so what it measures is the engine's behaviour, which every client
//! that links the engine inherits. A probe that reads the server's collection reads it only after
//! the server has stopped.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anki::card::CardId;
use anki::collection::{Collection, CollectionBuilder};
use anki::error::AnkiError;
use anki::scheduler::answering::{CardAnswer, Rating};
use anki::sync::collection::normal::SyncActionRequired;
use anki::sync::login::{SyncAuth, sync_login};
use anki::timestamp::TimestampMillis;
use support::recording::{Recorded, Recording};
use support::synthetic::{self, Shape, Side};
use support::{PASSWORD, SyncServer, USERNAME};
use tokio::runtime::Runtime;

/// The columns of the engine's `cards` table, in order: a card's whole row, read as one value.
const CARD_COLUMNS: [&str; 18] = [
    "id", "nid", "did", "ord", "mod", "usn", "type", "queue", "due", "ivl", "factor", "reps",
    "lapses", "left", "odue", "odid", "flags", "data",
];

/// A fresh HTTP client of the engine's own type, built by its `Default`, as the port builds one.
fn engine_client<Client: Default>() -> Client {
    Client::default()
}

/// A host key for the synthetic owner at `endpoint`, with the endpoint set as the port sets it.
fn login(runtime: &Runtime, endpoint: &str) -> SyncAuth {
    let mut auth = runtime
        .block_on(sync_login(
            USERNAME,
            PASSWORD,
            Some(endpoint.to_owned()),
            engine_client(),
        ))
        .unwrap_or_else(|error| panic!("the client logs in: {error}"));
    auth.endpoint = endpoint.parse().ok();
    auth
}

/// The collection at `path`, opened by the engine; a path with no collection yet starts as an
/// empty one.
fn open(path: &Path) -> Collection {
    CollectionBuilder::new(path)
        .build()
        .unwrap_or_else(|error| panic!("the engine opens {}: {error}", path.display()))
}

/// Closes a collection the probe opened.
fn close(col: Collection) {
    col.close(None)
        .unwrap_or_else(|error| panic!("the engine closes a collection: {error}"));
}

/// A client that starts in step with the server: a new collection at `path`, replaced by the
/// server's whole by the engine's full download, and closed.
fn download_into(runtime: &Runtime, path: &Path, endpoint: &str) {
    let col = open(path);
    let auth = login(runtime, endpoint);
    runtime
        .block_on(col.full_download(auth, engine_client()))
        .unwrap_or_else(|error| panic!("the client downloads the server's collection: {error}"));
}

/// The first new card by id: a card no review-log row names yet.
fn new_card(col: &Collection) -> CardId {
    let id: i64 = col
        .storage
        .db()
        .query_row(
            "select id from cards where type = 0 and queue = 0 order by id limit 1",
            (),
            |row| row.get(0),
        )
        .unwrap_or_else(|error| panic!("the collection holds a new card: {error}"));
    CardId(id)
}

/// Answers `card` Good, as a study screen does, without the queue's own bookkeeping.
fn answer_good(col: &mut Collection, card: CardId) {
    let states = col
        .get_scheduling_states(card)
        .unwrap_or_else(|error| panic!("the card's scheduling states: {error}"));
    col.answer_card(&mut CardAnswer {
        card_id: card,
        current_state: states.current,
        new_state: states.good,
        rating: Rating::Good,
        answered_at: TimestampMillis::now(),
        milliseconds_taken: 3_000,
        custom_data: None,
        from_queue: false,
    })
    .unwrap_or_else(|error| panic!("the card is answered: {error}"));
}

/// The review-log rows that name `card`.
fn rows_for(col: &Collection, card: CardId) -> i64 {
    col.storage
        .db()
        .query_row(
            "select count() from revlog where cid = ?",
            (card.0,),
            |row| row.get(0),
        )
        .unwrap_or_else(|error| panic!("the card's review-log rows: {error}"))
}

/// The review-log rows no sync has sent yet: the engine marks each with a usn of -1.
fn unsynced_rows(col: &Collection) -> i64 {
    col.storage
        .db()
        .query_row("select count() from revlog where usn = -1", (), |row| {
            row.get(0)
        })
        .unwrap_or_else(|error| panic!("the unsynced review-log rows: {error}"))
}

/// The id of the one review-log row that names `card`.
fn row_of(col: &Collection, card: CardId) -> i64 {
    col.storage
        .db()
        .query_row("select id from revlog where cid = ?", (card.0,), |row| {
            row.get(0)
        })
        .unwrap_or_else(|error| panic!("the card's review-log row: {error}"))
}

/// Every review-log id in a collection.
fn revlog_ids(col: &Collection) -> BTreeSet<i64> {
    let mut statement = col
        .storage
        .db()
        .prepare("select id from revlog")
        .unwrap_or_else(|error| panic!("the review-log query: {error}"));
    statement
        .query_map((), |row| row.get(0))
        .and_then(Iterator::collect)
        .unwrap_or_else(|error| panic!("the review-log ids: {error}"))
}

/// Every review-log id in the collection at `path`.
fn revlog_ids_at(path: &Path) -> BTreeSet<i64> {
    let col = open(path);
    let ids = revlog_ids(&col);
    close(col);
    ids
}

/// The collection's schema stamp (`col.scm`), which only a schema change or a full sync moves.
fn schema_at(path: &Path) -> i64 {
    let col = open(path);
    let stamp = col
        .storage
        .db()
        .query_row("select scm from col", (), |row| row.get(0))
        .unwrap_or_else(|error| panic!("the schema stamp: {error}"));
    close(col);
    stamp
}

/// A card's whole row, every column of the engine's `cards` table, as one JSON array.
fn card_row(col: &Collection, card: CardId) -> String {
    let columns: i64 = col
        .storage
        .db()
        .query_row(
            "select count() from pragma_table_info('cards')",
            (),
            |row| row.get(0),
        )
        .unwrap_or_else(|error| panic!("the cards table's columns: {error}"));
    assert_eq!(
        usize::try_from(columns).ok(),
        Some(CARD_COLUMNS.len()),
        "the probe reads every column of the cards table"
    );
    col.storage
        .db()
        .query_row(
            &format!(
                "select json_array({}) from cards where id = ?",
                CARD_COLUMNS.join(", ")
            ),
            (card.0,),
            |row| row.get(0),
        )
        .unwrap_or_else(|error| panic!("the card's row: {error}"))
}

/// The choice the engine's normal sync offers the collection at `path`, as
/// `(upload_ok, download_ok)`; None when it offers no full sync.
fn offered(runtime: &Runtime, path: &Path, endpoint: &str) -> Option<(bool, bool)> {
    let mut col = open(path);
    let auth = login(runtime, endpoint);
    let output = runtime
        .block_on(col.normal_sync(auth, engine_client()))
        .unwrap_or_else(|error| panic!("the normal sync answers: {error}"));
    close(col);
    match output.required {
        SyncActionRequired::FullSyncRequired {
            upload_ok,
            download_ok,
        } => Some((upload_ok, download_ok)),
        SyncActionRequired::NoChanges | SyncActionRequired::NormalSyncRequired => None,
    }
}

/// The methods the relay recorded, in order.
fn methods(recording: &Recording) -> Vec<String> {
    recording
        .requests()
        .into_iter()
        .map(|request| request.method)
        .collect()
}

/// Whether the relay recorded a chunk carrying the review-log row `id`.
fn pushed(recording: &Recording, id: i64) -> bool {
    recording.requests().iter().any(|request| {
        request.method == "applyChunk" && request.revlog().iter().any(|row| row.id == id)
    })
}

/// A server whose collection holds cards, behind the relay, and a client in step with it.
struct InStep {
    base: PathBuf,
    local: PathBuf,
    other: PathBuf,
    recording: Recording,
    server: SyncServer,
}

/// Starts [`InStep`] in `dir`: the server's collection is the small synthetic one, the local
/// client has downloaded it whole, and `other`, another client, is a copy of the local one.
fn in_step(runtime: &Runtime, test: &str, dir: &Path) -> InStep {
    let base = dir.join("server");
    synthetic::build_shaped(
        &support::server_collection(&base),
        Side::Server,
        Shape::SMALL,
    );
    let server = SyncServer::start(test, &base);
    let recording = Recording::start(server.endpoint());
    let local = dir.join("local.anki2");
    let other = dir.join("other.anki2");
    download_into(runtime, &local, recording.endpoint());
    fs::copy(&local, &other).unwrap_or_else(|error| {
        panic!("the other client starts from the same collection: {error}")
    });
    InStep {
        base,
        local,
        other,
        recording,
        server,
    }
}

/// [`in_step`], then another client uploads with a schema change: both sides hold cards and their
/// schemas differ, so the local client is offered both choices.
fn both_sides_hold_cards(runtime: &Runtime, test: &str, dir: &Path) -> InStep {
    let sides = in_step(runtime, test, dir);
    support::upload_from_another_client(runtime, &sides.other, sides.recording.endpoint());
    assert_eq!(
        offered(runtime, &sides.local, sides.recording.endpoint()),
        Some((true, true)),
        "both sides hold cards and their schemas differ"
    );
    sides
}

/// Stops the relay and the server, then returns the server's collection.
fn stopped(sides: InStep) -> PathBuf {
    let InStep {
        base,
        recording,
        server,
        ..
    } = sides;
    drop(recording);
    drop(server);
    support::server_collection(&base)
}

#[test]
fn u1_undo_deletes_the_answers_review_log_row_and_restores_the_card() {
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let path = scratch.path().join("local.anki2");
    synthetic::build_shaped(&path, Side::Client, Shape::SMALL);
    let mut col = open(&path);
    let card = new_card(&col);
    assert_eq!(rows_for(&col, card), 0, "the card has never been answered");
    let before = card_row(&col, card);

    answer_good(&mut col, card);
    assert_eq!(
        rows_for(&col, card),
        1,
        "the answer wrote one review-log row"
    );
    assert_ne!(card_row(&col, card), before, "the answer changed the card");

    col.undo().expect("the answer is undone");
    assert_eq!(rows_for(&col, card), 0, "the undo deleted the answer's row");
    assert_eq!(
        card_row(&col, card),
        before,
        "the undo restored the card's row byte for byte"
    );
    close(col);
}

#[test]
fn u2_a_normal_sync_empties_undo_and_the_synced_row_stays() {
    const TEST: &str = "u2_a_normal_sync_empties_undo_and_the_synced_row_stays";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let sides = in_step(&runtime, TEST, scratch.path());
    let mut col = open(&sides.local);
    let card = new_card(&col);
    answer_good(&mut col, card);
    let row = row_of(&col, card);
    assert!(
        col.can_undo().is_some(),
        "the answer can be undone before a sync"
    );

    let auth = login(&runtime, sides.recording.endpoint());
    runtime
        .block_on(col.normal_sync(auth, engine_client()))
        .expect("the normal sync completes");
    assert!(
        pushed(&sides.recording, row),
        "the sync pushed the answer's review-log row"
    );
    assert!(
        col.can_undo().is_none(),
        "after a normal sync, nothing can be undone"
    );
    assert!(
        matches!(col.undo(), Err(AnkiError::UndoEmpty)),
        "after a normal sync, an undo is refused as UndoEmpty"
    );
    assert_eq!(rows_for(&col, card), 1, "the local row stays");
    close(col);

    let server = open(&stopped(sides));
    assert_eq!(rows_for(&server, card), 1, "the server's row stays");
    close(server);
}

#[test]
fn u3_an_answer_undone_before_a_sync_never_reaches_the_server() {
    const TEST: &str = "u3_an_answer_undone_before_a_sync_never_reaches_the_server";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let sides = in_step(&runtime, TEST, scratch.path());
    let mut col = open(&sides.local);
    let card = new_card(&col);
    answer_good(&mut col, card);
    col.undo().expect("the answer is undone");
    assert_eq!(rows_for(&col, card), 0, "the undo deleted the answer's row");

    sides.recording.clear();
    let auth = login(&runtime, sides.recording.endpoint());
    runtime
        .block_on(col.normal_sync(auth, engine_client()))
        .expect("the normal sync completes");
    close(col);
    let requests = sides.recording.requests();
    assert!(
        requests.iter().any(|request| request.method == "meta"),
        "the sync asked the server: {:?}",
        methods(&sides.recording)
    );
    let carried: Vec<_> = requests
        .iter()
        .filter(|request| request.method == "applyChunk")
        .flat_map(Recorded::revlog)
        .filter(|row| row.cid == card.0)
        .collect();
    assert!(
        carried.is_empty(),
        "no pushed chunk carries a review-log row for the card: {carried:?}"
    );

    let server = open(&stopped(sides));
    let held: i64 = server
        .storage
        .db()
        .query_row("select count() from cards where id = ?", (card.0,), |row| {
            row.get(0)
        })
        .expect("the server's card");
    assert_eq!(held, 1, "the server holds the card");
    assert_eq!(rows_for(&server, card), 0, "the server holds no row for it");
    close(server);
}

#[test]
fn f1_the_choice_offered_follows_which_side_is_empty() {
    const TEST: &str = "f1_the_choice_offered_follows_which_side_is_empty";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let scratch = tempfile::tempdir().expect("a scratch directory");

    // Both sides hold cards: both choices (asserted inside the setup).
    let both = scratch.path().join("both");
    fs::create_dir_all(&both).expect("a directory for the first case");
    drop(both_sides_hold_cards(&runtime, TEST, &both));

    // The server is empty and the local collection is not: the upload alone.
    let base = scratch.path().join("empty-server");
    let held = support::server_collection(&base);
    assert!(!held.exists(), "the server starts with no collection");
    let server = SyncServer::start(TEST, &base);
    let local = scratch.path().join("full-local.anki2");
    synthetic::build_shaped(&local, Side::Client, Shape::SMALL);
    assert_eq!(
        offered(&runtime, &local, server.endpoint()),
        Some((true, false)),
        "an empty server offers the upload alone"
    );
    drop(server);

    // The local collection is empty and the server's is not: the download alone.
    let base = scratch.path().join("full-server");
    synthetic::build_shaped(
        &support::server_collection(&base),
        Side::Server,
        Shape::SMALL,
    );
    let server = SyncServer::start(TEST, &base);
    let local = scratch.path().join("empty-local.anki2");
    close(open(&local));
    assert_eq!(
        offered(&runtime, &local, server.endpoint()),
        Some((false, true)),
        "an empty local collection offers the download alone"
    );
}

#[test]
fn f2_the_upload_choice_sends_one_upload_and_no_second_meta() {
    const TEST: &str = "f2_the_upload_choice_sends_one_upload_and_no_second_meta";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let sides = both_sides_hold_cards(&runtime, TEST, scratch.path());

    let col = open(&sides.local);
    let auth = login(&runtime, sides.recording.endpoint());
    sides.recording.clear();
    runtime
        .block_on(col.full_upload(auth, engine_client()))
        .expect("the upload completes");
    assert_eq!(
        methods(&sides.recording),
        ["upload"],
        "the upload choice sends one request, with no meta before it"
    );
}

#[test]
fn f3_the_download_choice_sends_one_download_and_replaces_the_collection() {
    const TEST: &str = "f3_the_download_choice_sends_one_download_and_replaces_the_collection";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let sides = both_sides_hold_cards(&runtime, TEST, scratch.path());
    let schema_before = schema_at(&sides.local);

    let col = open(&sides.local);
    let auth = login(&runtime, sides.recording.endpoint());
    sides.recording.clear();
    runtime
        .block_on(col.full_download(auth, engine_client()))
        .expect("the download completes");
    assert_eq!(
        methods(&sides.recording),
        ["download"],
        "the download choice sends one request"
    );
    let local = sides.local.clone();
    let server = stopped(sides);
    let schema = schema_at(&server);
    assert_ne!(
        schema_before, schema,
        "the two sides' schemas differed before the download"
    );
    assert_eq!(
        (schema_at(&local), revlog_ids_at(&local)),
        (schema, revlog_ids_at(&server)),
        "the local collection is the server's: its schema stamp and its review-log ids"
    );
}

/// F4's two sides before the choice: the local client holds 2 reviews the server lacks, and the
/// server holds 3 the local client lacks, planted by another client's normal sync before its
/// upload with a schema change.
struct Diverged {
    sides: InStep,
    local_ids: BTreeSet<i64>,
    server_ids: BTreeSet<i64>,
    local_only: BTreeSet<i64>,
    server_only: BTreeSet<i64>,
}

fn diverged(runtime: &Runtime, test: &str, dir: &Path) -> Diverged {
    let sides = in_step(runtime, test, dir);
    let common = revlog_ids_at(&sides.local);
    support::review_on_another_client(runtime, &sides.other, sides.recording.endpoint(), 3);
    support::upload_from_another_client(runtime, &sides.other, sides.recording.endpoint());
    let server_ids = revlog_ids_at(&sides.other);
    let mut col = open(&sides.local);
    for _ in 0..2 {
        let card = new_card(&col);
        answer_good(&mut col, card);
    }
    close(col);
    let local_ids = revlog_ids_at(&sides.local);
    let local_only: BTreeSet<i64> = local_ids.difference(&common).copied().collect();
    let server_only: BTreeSet<i64> = server_ids.difference(&common).copied().collect();
    assert_eq!(
        (local_only.len(), server_only.len()),
        (2, 3),
        "the local client planted 2 reviews and the other client 3"
    );
    assert_eq!(
        offered(runtime, &sides.local, sides.recording.endpoint()),
        Some((true, true)),
        "both sides hold cards and their schemas differ"
    );
    Diverged {
        sides,
        local_ids,
        server_ids,
        local_only,
        server_only,
    }
}

#[test]
fn f4_each_choice_loses_exactly_the_other_sides_review_log_rows() {
    const TEST: &str = "f4_each_choice_loses_exactly_the_other_sides_review_log_rows";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let scratch = tempfile::tempdir().expect("a scratch directory");

    let up = scratch.path().join("upload");
    fs::create_dir_all(&up).expect("a directory for the upload");
    let up = diverged(&runtime, TEST, &up);
    let col = open(&up.sides.local);
    let auth = login(&runtime, up.sides.recording.endpoint());
    runtime
        .block_on(col.full_upload(auth, engine_client()))
        .expect("the upload completes");
    let after = revlog_ids_at(&stopped(up.sides));
    assert_eq!(
        after, up.local_ids,
        "after the upload the server holds the local rows"
    );
    assert_eq!(
        up.server_ids
            .difference(&after)
            .copied()
            .collect::<BTreeSet<_>>(),
        up.server_only,
        "the upload lost exactly the server's 3 rows"
    );

    let down = scratch.path().join("download");
    fs::create_dir_all(&down).expect("a directory for the download");
    let down = diverged(&runtime, TEST, &down);
    let col = open(&down.sides.local);
    let auth = login(&runtime, down.sides.recording.endpoint());
    runtime
        .block_on(col.full_download(auth, engine_client()))
        .expect("the download completes");
    let after = revlog_ids_at(&down.sides.local);
    assert_eq!(
        after, down.server_ids,
        "after the download the local rows are the server's"
    );
    assert_eq!(
        down.local_ids
            .difference(&after)
            .copied()
            .collect::<BTreeSet<_>>(),
        down.local_only,
        "the download lost exactly the local 2 rows"
    );
}

#[test]
fn f5_a_normal_sync_between_the_meta_and_the_upload_is_overwritten() {
    const TEST: &str = "f5_a_normal_sync_between_the_meta_and_the_upload_is_overwritten";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let scratch = tempfile::tempdir().expect("a scratch directory");
    // Client A is the local one, client B the other; both start in step.
    let sides = in_step(&runtime, TEST, scratch.path());

    // A changes its schema and answers a card, so both its stamps differ from the server's.
    let mut a = open(&sides.local);
    a.set_schema_modified()
        .expect("client A changes its schema");
    let card = new_card(&a);
    answer_good(&mut a, card);
    let auth = login(&runtime, sides.recording.endpoint());
    let output = runtime
        .block_on(a.normal_sync(auth, engine_client()))
        .expect("client A's meta answers");
    assert!(
        matches!(
            output.required,
            SyncActionRequired::FullSyncRequired {
                upload_ok: true,
                download_ok: true
            }
        ),
        "client A is offered both choices"
    );

    // In the window, B answers one card and syncs normally: its row reaches the server.
    let b_before = revlog_ids_at(&sides.other);
    sides.recording.clear();
    support::review_on_another_client(&runtime, &sides.other, sides.recording.endpoint(), 1);
    let b_rows: Vec<i64> = revlog_ids_at(&sides.other)
        .difference(&b_before)
        .copied()
        .collect();
    let [b_row] = b_rows.as_slice() else {
        panic!("client B answered one card: {b_rows:?}");
    };
    assert!(
        pushed(&sides.recording, *b_row),
        "client B's review reached the server"
    );

    // A uploads as it was offered, with nothing re-checking the server.
    let auth = login(&runtime, sides.recording.endpoint());
    sides.recording.clear();
    runtime
        .block_on(a.full_upload(auth, engine_client()))
        .expect("client A's upload completes");
    assert_eq!(
        methods(&sides.recording),
        ["upload"],
        "no meta before the upload"
    );
    let a_ids = revlog_ids_at(&sides.local);
    let after = revlog_ids_at(&stopped(sides));
    assert!(
        !after.contains(b_row),
        "client B's review is absent from the server after A's upload"
    );
    assert_eq!(after, a_ids, "the server holds client A's rows");
}

#[test]
fn f6_a_download_loses_a_synced_review_the_server_no_longer_holds() {
    const TEST: &str = "f6_a_download_loses_a_synced_review_the_server_no_longer_holds";
    if support::role().as_deref() == Some(support::SERVER) {
        return support::serve();
    }
    let runtime = support::runtime();
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let sides = in_step(&runtime, TEST, scratch.path());

    // The local client answers card X and syncs it: X's row reaches the server.
    let mut col = open(&sides.local);
    let card = new_card(&col);
    answer_good(&mut col, card);
    let x_row = row_of(&col, card);
    let auth = login(&runtime, sides.recording.endpoint());
    runtime
        .block_on(col.normal_sync(auth, engine_client()))
        .expect("the local sync completes");
    assert!(
        pushed(&sides.recording, x_row),
        "X's review reached the server"
    );
    assert_eq!(
        unsynced_rows(&col),
        0,
        "the local client holds no unsynced row"
    );
    close(col);

    // Another client, without X, uploads with a schema change.
    support::upload_from_another_client(&runtime, &sides.other, sides.recording.endpoint());
    let replaced = revlog_ids_at(&sides.other);
    assert!(!replaced.contains(&x_row), "the other client never saw X");
    assert_eq!(
        offered(&runtime, &sides.local, sides.recording.endpoint()),
        Some((true, true)),
        "the local client's next sync offers a full sync"
    );

    let col = open(&sides.local);
    let auth = login(&runtime, sides.recording.endpoint());
    runtime
        .block_on(col.full_download(auth, engine_client()))
        .expect("the download completes");
    let after = revlog_ids_at(&sides.local);
    assert!(
        !after.contains(&x_row),
        "the download lost X's review, which the local client had already synced"
    );
    assert_eq!(after, replaced, "the local rows are the replaced server's");
}
