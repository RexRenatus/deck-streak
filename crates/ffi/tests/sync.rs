//! The native sync facade against the engine's own sync server on a loopback port (SPEC-503 A2 to
//! A4): a first sync through the full-sync choice, the normal syncs after it, the endpoint rule and
//! the handshake before a native normal sync, and the unsynced count read offline. The choice's
//! steps are each tapped as the app may tap them: after a cancel, after a refusal, with a direction
//! the choice does not name, with no snapshot found, over a server that holds rows, and after the
//! server changed since the counts.
//!
//! Each test that needs the server re-executes its own binary as the server's child
//! ([`sync_server::SERVER`]). The server is filled by a second client through the same facade, an
//! upload, so the test names no engine write of its own; what the device holds after its download
//! is read back through the adapter's own deck list. The fixtures the unsynced read and the counts
//! read are written with the engine's own API before the adapter opens them, as the synthetic
//! collection is.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

#[expect(
    dead_code,
    reason = "the sync tests open the synthetic collection by its path and read its ids only to tell \
              two collections apart and to review its card, and they send and read only the wire \
              fields a sync needs"
)]
mod support;

#[path = "support/sync_server.rs"]
mod sync_server;

use std::io::{Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use anki::collection::CollectionBuilder;
use deck_streak_ffi::engine::{ChoiceLoss, ChoiceStep, Engine, EngineRefusal, UnsyncedCount};
use support::{Synthetic, synthetic, wire};
use sync_server::{PASSWORD, SERVER, SyncServer, USERNAME};

/// The engine's login call, `BackendSyncService.SyncLogin`.
const SYNC_LOGIN: (u32, u32) = (1, 3);
/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `DecksService.GetDeckNames`.
const DECK_NAMES: (u32, u32) = (7, 13);
/// The engine's numbers for what a normal sync requires next: `FULL_SYNC`, `FULL_DOWNLOAD` and
/// `FULL_UPLOAD` (`sync.proto`'s `ChangesRequired`); below them, nothing more than a normal sync.
const FULL_SYNC: u32 = 2;
const FULL_DOWNLOAD: u32 = 3;
const FULL_UPLOAD: u32 = 4;
/// The choice's directions, as the facade numbers them.
const UPLOAD: u32 = 0;
const DOWNLOAD: u32 = 1;
/// A direction that loses nothing.
const NOTHING: ChoiceLoss = ChoiceLoss {
    reviews: 0,
    cards: 0,
    notes: 0,
};

/// The facade's sentence for a tap when no counts are open, as the app shows it.
const NO_COUNTS_OPEN: &str = "Nothing was written: no counts are open. Sync again.";
/// The facade's sentence for a tap on a direction the choice does not offer.
const NOT_OFFERED: &str = "Nothing was written: that direction is not offered.";
/// The facade's sentence for an upload whose snapshot was not found.
const NO_SNAPSHOT: &str =
    "Nothing was written: an upload waits until the server's snapshot is found.";
/// The facade's sentence when the server changed between the counts and the upload.
const SERVER_CHANGED: &str =
    "Nothing was written: the server changed since the counts. Check the new counts.";
/// The login's endpoint rule, as the core states it (SPEC-347 R2).
const RULE: &str =
    "the sync login's endpoint is neither https nor plain http to a loopback address";

/// A direction no tap of the app names: neither the upload (0) nor the download (1).
const NO_DIRECTION: u32 = 2;
/// One loss of a collection's one card and its one note, and of `reviews` review-log rows.
const fn one_card_and_note(reviews: u64) -> ChoiceLoss {
    ChoiceLoss {
        reviews,
        cards: 1,
        notes: 1,
    }
}

/// A collection's last sync, as its `ls` stamp: past any clock this test reads, so the modified and
/// schema stamps sit after it only where a case puts them there.
const LAST_SYNC: i64 = 4_000_000_000_000;
/// The first review-log id a fixture writes: a review's id is the time it was made, in milliseconds.
const FIRST_REVIEW: i64 = 1_700_000_000_000;
/// One review of a card, unsynced (`usn` -1) as the engine records it before its next sync.
const REVIEW_SQL: &str = "insert into revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
                          values (?1, ?2, -1, 3, 1, 0, 2500, 6000, 1)";
/// The collection's three sync stamps: its last sync, its modified stamp and its schema stamp.
const STAMPS_SQL: &str = "update col set ls = ?1, mod = ?2, scm = ?3";
/// The statement front's answer to a read it was not meant to get: an admitting statement.
const ADMITTING_ANSWER: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                                  Content-Length: 26\r\nConnection: close\r\n\r\n\
                                  {\"minimum_client_level\":1}";

fn engine() -> Arc<Engine> {
    Engine::new(Vec::new()).expect("the engine starts from the default init message")
}

/// The text of a string field.
fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("a string field is UTF-8")
}

/// `OpenCollectionRequest` for a collection file in `dir` that need not exist yet: the path (field
/// 1), the media folder (2) and the media database (3).
fn open(engine: &Engine, dir: &Path) {
    std::fs::create_dir_all(dir.join("collection.media")).expect("the media folder is made");
    let path = |name: &str| dir.join(name).to_str().expect("UTF-8").to_owned();
    let mut request = Vec::new();
    wire::put_bytes(&mut request, 1, path("collection.anki2").as_bytes());
    wire::put_bytes(&mut request, 2, path("collection.media").as_bytes());
    wire::put_bytes(&mut request, 3, path("collection.media.db").as_bytes());
    engine
        .run(OPEN_COLLECTION.0, OPEN_COLLECTION.1, request)
        .expect("the adapter opens the collection");
}

/// The host key the server mints for the synthetic account, through the adapter's own login.
fn host_key(engine: &Engine, endpoint: &str) -> String {
    let mut request = Vec::new();
    wire::put_bytes(&mut request, 1, USERNAME.as_bytes());
    wire::put_bytes(&mut request, 2, PASSWORD.as_bytes());
    wire::put_bytes(&mut request, 3, endpoint.as_bytes());
    let auth = engine
        .run(SYNC_LOGIN.0, SYNC_LOGIN.1, request)
        .expect("the adapter logs in");
    text(&wire::bytes(&auth, 1))
}

/// The deck names the open collection lists, through the adapter's own call.
fn deck_names(engine: &Engine) -> Vec<String> {
    let reply = engine
        .run(DECK_NAMES.0, DECK_NAMES.1, Vec::new())
        .expect("the adapter lists the decks");
    wire::repeated(&reply, 1)
        .iter()
        .map(|entry| text(&wire::bytes(entry, 2)))
        .collect()
}

/// A folder of this test's own beside the synthetic collection's, for a fresh install.
fn folder(base: &Path, name: &str) -> String {
    let dir = base.join(name);
    std::fs::create_dir_all(&dir).expect("the folder is made");
    dir.to_str().expect("UTF-8").to_owned()
}

/// The message of an engine-shaped refusal.
fn message(refusal: &EngineRefusal) -> String {
    match refusal {
        EngineRefusal::Engine { error } => text(&wire::bytes(error, 1)),
        other => format!("not an engine-shaped refusal: {other:?}"),
    }
}

/// A second client holding `synthetic` fills the empty server at `endpoint` by its own upload,
/// through the same facade, so the server holds the collection a fresh install downloads.
fn fill_the_server(synthetic: &Synthetic, endpoint: &str) {
    let other = engine();
    other
        .run(
            OPEN_COLLECTION.0,
            OPEN_COLLECTION.1,
            support::open_request(synthetic),
        )
        .expect("the second client opens its collection");
    let other_key = host_key(&other, endpoint);
    let required = other
        .sync(other_key.clone(), endpoint.to_owned())
        .expect("the second client's normal sync is answered");
    assert!(
        [FULL_SYNC, FULL_UPLOAD].contains(&required),
        "an empty server asks the second client for a full sync: {required}"
    );
    let other_folder = folder(&synthetic.dir, "choice");
    let counted = other
        .full_sync_count(
            other_key.clone(),
            endpoint.to_owned(),
            required,
            other_folder.clone(),
        )
        .expect("the second client's counts");
    assert_eq!(
        counted.counts.and_then(|counts| counts.upload),
        Some(NOTHING),
        "the upload is offered, and the empty server loses nothing"
    );
    let uploaded = other
        .full_sync_confirm(UPLOAD, other_key, endpoint.to_owned(), true, other_folder)
        .expect("the second client's upload");
    assert!(
        uploaded.written,
        "the second client's upload is written: {}",
        uploaded.said
    );
}

#[test]
fn a_fresh_installs_first_sync_counts_backs_up_and_downloads_then_syncs_normally() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start(
        "a_fresh_installs_first_sync_counts_backs_up_and_downloads_then_syncs_normally",
    );
    let endpoint = server.endpoint().to_owned();

    // A second client holding the synthetic collection fills the empty server by its own upload.
    let synthetic = synthetic("native-sync-first-sync");
    fill_the_server(&synthetic, &endpoint);

    // The fresh install: an empty collection, its first sync through the choice.
    let device = engine();
    let device_folder = folder(&synthetic.dir, "fresh-install");
    open(&device, Path::new(&device_folder));
    let key = host_key(&device, &endpoint);
    let required = device
        .sync(key.clone(), endpoint.clone())
        .expect("the fresh install's normal sync is answered");
    assert!(
        [FULL_SYNC, FULL_DOWNLOAD].contains(&required),
        "a fresh install's first sync needs a full sync: {required}"
    );
    let unsynced = device.unsynced().expect("the unsynced read");
    assert_eq!(
        unsynced.reviews, 0,
        "a fresh install has no unsynced review"
    );
    let counted = device
        .full_sync_count(
            key.clone(),
            endpoint.clone(),
            required,
            device_folder.clone(),
        )
        .expect("the fresh install's counts");
    let counts = counted.counts.expect("the choice is open with its counts");
    assert_eq!(
        (counts.upload, counts.download),
        (None, Some(NOTHING)),
        "the download alone is offered, and a fresh install loses nothing by it"
    );
    let written = device
        .full_sync_confirm(
            DOWNLOAD,
            key.clone(),
            endpoint.clone(),
            false,
            device_folder.clone(),
        )
        .expect("the download");
    assert!(written.written, "the download is written: {}", written.said);
    let backups = std::fs::read_dir(&device_folder)
        .expect("the folder reads")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("backup-"))
        .count();
    assert_eq!(
        backups, 1,
        "the download's backup is kept beside the collection"
    );
    assert!(
        deck_names(&device).contains(&synthetic::SECOND_DECK.to_owned()),
        "the device holds the server's deck after its download"
    );

    // The write closed the choice: a second tap finds no counts open and writes nothing.
    let closed = device
        .full_sync_confirm(
            DOWNLOAD,
            key.clone(),
            endpoint.clone(),
            false,
            device_folder,
        )
        .expect("a second tap is answered");
    assert_eq!(
        (closed.written, closed.said.as_str(), closed.counts),
        (false, NO_COUNTS_OPEN, None),
        "once the download is written, the choice is closed and a second tap writes nothing"
    );

    // The syncs after it are normal ones.
    let again = device
        .sync(key, endpoint)
        .expect("the next normal sync is answered");
    assert!(
        again < FULL_SYNC,
        "after the first sync, a sync needs no full sync: {again}"
    );
}

#[test]
fn a_native_normal_sync_keeps_the_endpoint_rule_and_reads_no_statement_for_a_refused_endpoint() {
    let refused = engine()
        .sync(
            String::from("planted-host-key"),
            String::from("http://example.invalid/"),
        )
        .expect_err("plain http to a name is refused");
    assert_eq!(
        message(&refused),
        "the sync login's endpoint is neither https nor plain http to a loopback address",
        "the native normal sync meets the login's endpoint rule before the engine"
    );
}

#[test]
fn a_native_normal_sync_with_no_statement_read_is_refused_before_the_engine() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().expect("its address").port();
    drop(listener);
    let refused = engine()
        .sync(
            String::from("planted-host-key"),
            format!("http://127.0.0.1:{port}/"),
        )
        .expect_err("no statement answered, so the sync is refused");
    assert_eq!(
        message(&refused),
        unread_sentence(),
        "a native normal sync waits for a statement that admits this client"
    );
}

/// The core's sentence when no statement was read, written here from SPEC-374 R6, never read from
/// the core.
fn unread_sentence() -> String {
    String::from(
        "A network error occurred. The sync service's oldest accepted version could not be read, \
         so nothing was synced.",
    )
}

#[test]
fn the_unsynced_count_is_read_offline_before_any_full_sync() {
    let synthetic = synthetic("native-sync-unsynced");
    let device = engine();
    device
        .run(
            OPEN_COLLECTION.0,
            OPEN_COLLECTION.1,
            support::open_request(&synthetic),
        )
        .expect("the device opens its collection");
    let unsynced = device
        .unsynced()
        .expect("the unsynced read needs no network");
    assert!(
        unsynced.changed || unsynced.schema,
        "a collection that never synced reads as changed since its last sync: {unsynced:?}"
    );
}

/// One client of the server through the facade: its engine, its host key, the endpoint it logged
/// in at, and the folder its choice's files are kept in.
struct Client {
    engine: Arc<Engine>,
    key: String,
    endpoint: String,
    folder: String,
}

impl Client {
    /// A client holding `synthetic`, logged in at `endpoint`, its choice's files in a folder of its
    /// own beside its collection.
    fn holding(synthetic: &Synthetic, endpoint: &str) -> Self {
        let engine = engine();
        engine
            .run(
                OPEN_COLLECTION.0,
                OPEN_COLLECTION.1,
                support::open_request(synthetic),
            )
            .expect("the client opens its collection");
        let key = host_key(&engine, endpoint);
        Self {
            engine,
            key,
            endpoint: endpoint.to_owned(),
            folder: folder(&synthetic.dir, "choice"),
        }
    }

    /// A fresh install beside `synthetic`'s folder: an empty collection, logged in at `endpoint`,
    /// its choice's files kept in the collection's own folder.
    fn fresh(synthetic: &Synthetic, endpoint: &str) -> Self {
        let engine = engine();
        let folder = folder(&synthetic.dir, "fresh-install");
        open(&engine, Path::new(&folder));
        let key = host_key(&engine, endpoint);
        Self {
            engine,
            key,
            endpoint: endpoint.to_owned(),
            folder,
        }
    }

    /// The client's normal sync, then the choice's counts from what it required.
    fn count(&self) -> ChoiceStep {
        let required = self
            .engine
            .sync(self.key.clone(), self.endpoint.clone())
            .expect("the client's normal sync is answered");
        self.engine
            .full_sync_count(
                self.key.clone(),
                self.endpoint.clone(),
                required,
                self.folder.clone(),
            )
            .expect("the client's counts")
    }

    /// One tap on `direction`, with the snapshot answer `found`.
    fn tap(&self, direction: u32, found: bool) -> ChoiceStep {
        self.engine
            .full_sync_confirm(
                direction,
                self.key.clone(),
                self.endpoint.clone(),
                found,
                self.folder.clone(),
            )
            .expect("the tap is answered")
    }

    /// The files of `kind` the choice made in the client's folder.
    fn files(&self, kind: &str) -> usize {
        std::fs::read_dir(&self.folder)
            .expect("the folder reads")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(kind))
            .count()
    }
}

/// A step's counts as (upload, download), each `None` when not offered; `None` with no counts.
type Offered = Option<(Option<ChoiceLoss>, Option<ChoiceLoss>)>;

/// A step's three parts, for one comparison.
fn shown(step: &ChoiceStep) -> (bool, &str, Offered) {
    (
        step.written,
        step.said.as_str(),
        step.counts.map(|counts| (counts.upload, counts.download)),
    )
}

/// A fresh install whose first sync's counts are open against a server a second client filled
/// with `synthetic`: the download alone is offered, and it loses nothing.
fn counted_fresh_install(synthetic: &Synthetic, endpoint: &str) -> Client {
    fill_the_server(synthetic, endpoint);
    let device = Client::fresh(synthetic, endpoint);
    let counted = device.count();
    assert_eq!(
        shown(&counted).2,
        Some((None, Some(NOTHING))),
        "a fresh install is offered the download alone, which loses nothing"
    );
    device
}

/// Whether `client`'s collection lists the synthetic collection's second deck.
fn holds_the_servers_deck(client: &Client) -> bool {
    deck_names(&client.engine).contains(&synthetic::SECOND_DECK.to_owned())
}

#[test]
fn a_cancelled_choice_holds_no_counts_so_a_tap_after_it_writes_nothing() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server =
        SyncServer::start("a_cancelled_choice_holds_no_counts_so_a_tap_after_it_writes_nothing");
    let synthetic = synthetic("native-sync-cancel");
    let device = counted_fresh_install(&synthetic, server.endpoint());

    device.engine.full_sync_cancel();
    let tapped = device.tap(DOWNLOAD, false);
    assert_eq!(
        (shown(&tapped), holds_the_servers_deck(&device)),
        ((false, NO_COUNTS_OPEN, None), false),
        "after the cancel no counts are open, so the download's tap writes nothing"
    );
}

#[test]
fn a_refused_tap_keeps_the_open_counts_for_the_next_tap() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("a_refused_tap_keeps_the_open_counts_for_the_next_tap");
    let synthetic = synthetic("native-sync-refused-tap");
    let device = counted_fresh_install(&synthetic, server.endpoint());

    let refused = device.tap(UPLOAD, false);
    assert_eq!(
        shown(&refused),
        (false, NOT_OFFERED, None),
        "the upload is not offered to a fresh install, so its tap writes nothing"
    );
    let written = device.tap(DOWNLOAD, false);
    assert!(
        written.written && holds_the_servers_deck(&device),
        "the refused tap kept the counts, so the download's tap writes: {}",
        written.said
    );
}

#[test]
fn a_direction_the_choice_does_not_name_is_refused_and_writes_nothing() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server =
        SyncServer::start("a_direction_the_choice_does_not_name_is_refused_and_writes_nothing");
    let synthetic = synthetic("native-sync-no-direction");
    let device = counted_fresh_install(&synthetic, server.endpoint());

    let tapped = device.tap(NO_DIRECTION, false);
    assert_eq!(
        (shown(&tapped), holds_the_servers_deck(&device)),
        ((false, NOT_OFFERED, None), false),
        "a direction that is neither the upload nor the download is refused, never run as one"
    );
}

#[test]
fn an_upload_waits_for_its_snapshot_and_writes_nothing_without_it() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server =
        SyncServer::start("an_upload_waits_for_its_snapshot_and_writes_nothing_without_it");
    let synthetic = synthetic("native-sync-no-snapshot");
    let device = Client::holding(&synthetic, server.endpoint());
    assert_eq!(
        shown(&device.count()).2.and_then(|(upload, _)| upload),
        Some(NOTHING),
        "the empty server is offered the device's upload, and loses nothing by it"
    );

    let refused = device.tap(UPLOAD, false);
    assert_eq!(
        shown(&refused),
        (false, NO_SNAPSHOT, None),
        "with no snapshot found, the upload's tap writes nothing"
    );
    let written = device.tap(UPLOAD, true);
    assert!(
        written.written,
        "once the snapshot is found, the same counts' upload writes: {}",
        written.said
    );
}

#[test]
fn an_upload_over_a_server_that_holds_rows_rechecks_it_into_a_new_file_and_writes() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start(
        "an_upload_over_a_server_that_holds_rows_rechecks_it_into_a_new_file_and_writes",
    );
    let first = synthetic("native-sync-held-server");
    fill_the_server(&first, server.endpoint());
    let device = Client::holding(&synthetic("native-sync-held-device"), server.endpoint());
    assert_eq!(
        shown(&device.count()).2.and_then(|(upload, _)| upload),
        Some(one_card_and_note(0)),
        "the upload is offered, and loses the server's card and note"
    );

    let written = device.tap(UPLOAD, true);
    assert_eq!(
        (written.written, device.files("server-copy-")),
        (true, 2),
        "the re-check reads the server into a second file, never the counted copy, and the upload \
         writes: {}",
        written.said
    );
}

#[test]
fn the_counts_name_each_directions_lost_reviews_cards_and_notes() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start("the_counts_name_each_directions_lost_reviews_cards_and_notes");
    let first = synthetic("native-sync-counted-server");
    fill_the_server(&first, server.endpoint());
    let held = synthetic("native-sync-counted-device");
    assert_ne!(
        (first.note_id, first.card_id),
        (held.note_id, held.card_id),
        "the two collections share no id"
    );
    stamped(&held, 2, None);
    let device = Client::holding(&held, server.endpoint());

    assert_eq!(
        shown(&device.count()).2,
        Some((Some(one_card_and_note(0)), Some(one_card_and_note(2)))),
        "the upload loses the server's card and note; the download loses the device's card, note \
         and two reviews"
    );
}

#[test]
fn a_server_change_after_the_counts_shows_new_counts_and_the_next_tap_writes() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server = SyncServer::start(
        "a_server_change_after_the_counts_shows_new_counts_and_the_next_tap_writes",
    );
    fill_the_server(&synthetic("native-sync-changed-first"), server.endpoint());
    let device = Client::holding(&synthetic("native-sync-changed-device"), server.endpoint());
    device.count();

    // A third client replaces the server's collection after the device's counts.
    let third = Client::holding(&synthetic("native-sync-changed-third"), server.endpoint());
    third.count();
    let replaced = third.tap(UPLOAD, true);
    assert!(
        replaced.written,
        "the third client's upload: {}",
        replaced.said
    );

    let changed = device.tap(UPLOAD, true);
    assert_eq!(
        shown(&changed),
        (
            false,
            SERVER_CHANGED,
            Some((Some(one_card_and_note(0)), Some(one_card_and_note(0))))
        ),
        "the re-check finds the third client's collection, so nothing is written and new counts show"
    );
    let written = device.tap(UPLOAD, true);
    assert!(
        written.written,
        "the new counts are held, so the next tap's re-check finds the server unchanged and writes: {}",
        written.said
    );
}

#[test]
fn a_count_at_an_endpoint_the_rule_refuses_is_the_engines_refusal_not_a_sentence() {
    let synthetic = synthetic("native-sync-refused-count");
    let refused = engine()
        .full_sync_count(
            String::from("planted-host-key"),
            String::from("http://example.invalid/"),
            FULL_SYNC,
            folder(&synthetic.dir, "choice"),
        )
        .expect_err("the server copy's fetch meets the endpoint rule");
    assert_eq!(
        message(&refused),
        RULE,
        "the core's refusal reaches the app as the engine's error, with the rule's own sentence"
    );
}

#[test]
fn a_fresh_engine_syncs_on_its_own_statement_read_with_no_login_before_it() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let server =
        SyncServer::start("a_fresh_engine_syncs_on_its_own_statement_read_with_no_login_before_it");
    let endpoint = server.endpoint().to_owned();
    // The host key is another engine's login; this engine has read no statement before its sync.
    let key = host_key(&engine(), &endpoint);
    let device = engine();
    let synthetic = synthetic("native-sync-no-login");
    open(&device, Path::new(&folder(&synthetic.dir, "fresh-engine")));

    let synced = device.sync(key, endpoint);
    assert!(
        matches!(synced, Ok(required) if required <= FULL_UPLOAD),
        "only the sync's own statement read can admit this engine, before the engine syncs: \
         {synced:?}"
    );
}

#[test]
fn a_native_normal_sync_reads_no_statement_at_a_name_the_rule_refuses() {
    let counter = Counter::start();
    let refused = engine()
        .sync(
            String::from("planted-host-key"),
            format!("http://localhost:{}/", counter.address.port()),
        )
        .expect_err("plain http to a name is refused");
    assert_eq!(
        (counter.seen(), message(&refused)),
        (0, RULE.to_owned()),
        "an endpoint the rule refuses is never read, and the sync keeps the rule's sentence"
    );
}

#[test]
fn the_unsynced_count_reads_the_unsynced_reviews_and_each_stamp_alone() {
    let read = |case: &str, reviews: i64, stamps: (bool, bool)| {
        let synthetic = synthetic(case);
        stamped(&synthetic, reviews, Some(stamps));
        let device = engine();
        device
            .run(
                OPEN_COLLECTION.0,
                OPEN_COLLECTION.1,
                support::open_request(&synthetic),
            )
            .expect("the device opens its collection");
        device.unsynced().expect("the unsynced read")
    };
    let count = |reviews, changed, schema| UnsyncedCount {
        reviews,
        changed,
        schema,
    };
    assert_eq!(
        [
            read("native-sync-unsynced-reviews", 2, (false, false)),
            read("native-sync-unsynced-changed", 0, (true, false)),
            read("native-sync-unsynced-schema", 0, (false, true)),
        ],
        [
            count(2, false, false),
            count(0, true, false),
            count(0, false, true),
        ],
        "the unsynced reviews are counted, and each stamp after the last sync is read alone"
    );
}

/// Writes into `synthetic`, with the engine's own API before the adapter opens it, `reviews`
/// unsynced reviews of its card and, when `stamps` names them, its last sync at [`LAST_SYNC`] with
/// its modified and its schema stamp after it or before it.
fn stamped(synthetic: &Synthetic, reviews: i64, stamps: Option<(bool, bool)>) {
    let col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine reopens the synthetic collection");
    let db = col.storage.db();
    for review in 0..reviews {
        db.execute(REVIEW_SQL, [FIRST_REVIEW + review, synthetic.card_id])
            .expect("a review row is written");
    }
    if let Some((changed, schema)) = stamps {
        let after = |set: bool| if set { LAST_SYNC + 1 } else { LAST_SYNC - 1 };
        db.execute(STAMPS_SQL, [LAST_SYNC, after(changed), after(schema)])
            .expect("the stamps are written");
    }
    col.close(None).expect("the collection closes");
}

/// A loopback listener that counts the connections it is sent and answers each with an admitting
/// statement, so a read made of it is seen and returns at once.
struct Counter {
    address: SocketAddr,
    seen: Arc<AtomicUsize>,
    stopped: Arc<AtomicBool>,
}

impl Counter {
    fn start() -> Self {
        let listener =
            TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("the counter binds a loopback port");
        let address = listener.local_addr().expect("the counter's address reads");
        let seen = Arc::new(AtomicUsize::new(0));
        let stopped = Arc::new(AtomicBool::new(false));
        let (count, stop) = (Arc::clone(&seen), Arc::clone(&stopped));
        thread::spawn(move || {
            for connection in listener.incoming() {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                count.fetch_add(1, Ordering::SeqCst);
                if let Ok(connection) = connection {
                    admit(connection);
                }
            }
        });
        Self {
            address,
            seen,
            stopped,
        }
    }

    /// The connections counted so far.
    fn seen(&self) -> usize {
        self.seen.load(Ordering::SeqCst)
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
    }
}

/// Reads one request's head and answers it with an admitting statement.
fn admit(mut connection: TcpStream) {
    let _ = connection.set_read_timeout(Some(Duration::from_secs(5)));
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16_384 {
        match connection.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => break,
        }
    }
    let _ = connection.write_all(ADMITTING_ANSWER);
    let _ = connection.shutdown(Shutdown::Write);
    let _ = std::io::copy(&mut connection, &mut std::io::sink());
}
