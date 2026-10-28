//! The native sync facade against the engine's own sync server on a loopback port (SPEC-503 A2 to
//! A4): a first sync through the full-sync choice, the normal syncs after it, the endpoint rule and
//! the handshake before a native normal sync, and the unsynced count read offline.
//!
//! Each test that needs the server re-executes its own binary as the server's child
//! ([`sync_server::SERVER`]). The server is filled by a second client through the same facade, an
//! upload, so the test names no engine write of its own; what the device holds after its download
//! is read back through the adapter's own deck list.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

mod support;

#[path = "support/sync_server.rs"]
mod sync_server;

use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;

use deck_streak_ffi::engine::{ChoiceLoss, Engine, EngineRefusal};
use support::{synthetic, wire};
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
    let other = engine();
    other
        .run(
            OPEN_COLLECTION.0,
            OPEN_COLLECTION.1,
            support::open_request(&synthetic),
        )
        .expect("the second client opens its collection");
    let other_key = host_key(&other, &endpoint);
    let required = other
        .sync(other_key.clone(), endpoint.clone())
        .expect("the second client's normal sync is answered");
    assert!(
        [FULL_SYNC, FULL_UPLOAD].contains(&required),
        "an empty server asks the second client for a full sync: {required}"
    );
    let other_folder = folder(&synthetic.dir, "choice");
    let counted = other
        .full_sync_count(other_key.clone(), endpoint.clone(), required, other_folder.clone())
        .expect("the second client's counts");
    assert_eq!(
        counted.counts.and_then(|counts| counts.upload),
        Some(NOTHING),
        "the upload is offered, and the empty server loses nothing"
    );
    let uploaded = other
        .full_sync_confirm(UPLOAD, other_key, endpoint.clone(), true, other_folder)
        .expect("the second client's upload");
    assert!(uploaded.written, "the second client's upload is written: {}", uploaded.said);

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
    assert_eq!(unsynced.reviews, 0, "a fresh install has no unsynced review");
    let counted = device
        .full_sync_count(key.clone(), endpoint.clone(), required, device_folder.clone())
        .expect("the fresh install's counts");
    let counts = counted.counts.expect("the choice is open with its counts");
    assert_eq!(
        (counts.upload, counts.download),
        (None, Some(NOTHING)),
        "the download alone is offered, and a fresh install loses nothing by it"
    );
    let written = device
        .full_sync_confirm(DOWNLOAD, key.clone(), endpoint.clone(), false, device_folder.clone())
        .expect("the download");
    assert!(written.written, "the download is written: {}", written.said);
    let backups = std::fs::read_dir(&device_folder)
        .expect("the folder reads")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("backup-"))
        .count();
    assert_eq!(backups, 1, "the download's backup is kept beside the collection");
    assert!(
        deck_names(&device).contains(&synthetic::SECOND_DECK.to_owned()),
        "the device holds the server's deck after its download"
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
fn a_native_normal_sync_keeps_the_endpoint_rule_and_reads_no_statement_for_a_refused_endpoint()
{
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
    let unsynced = device.unsynced().expect("the unsynced read needs no network");
    assert!(
        unsynced.changed || unsynced.schema,
        "a collection that never synced reads as changed since its last sync: {unsynced:?}"
    );
}
