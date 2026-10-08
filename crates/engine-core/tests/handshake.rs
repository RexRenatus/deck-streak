//! The minimum-client handshake in the core (SPEC-374 A1 to A3; ADR-385).
//!
//! Each test hands a dispatcher a statement of the service's minimum client level, spelled here as
//! a literal and never derived from the level the core declares, then sends it sync calls whose
//! endpoint is a closed loopback port: a call the core lets through reaches the engine, which
//! answers with its own network refusal, and a call the core refuses answers with the sentence the
//! SPEC names, written out here from R6 and never read from the core.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture fails its test, and an enumerating test prints what it examined"
)]

mod support;

use std::net::{Ipv4Addr, TcpListener};
use std::path::Path;

use anki::collection::{Collection, CollectionBuilder};
use anki::sync::login::{SyncAuth as EngineAuth, sync_login};
use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::sync::sync_collection_response::ChangesRequired;
use anki_proto::sync::{SyncAuth, SyncCollectionRequest, SyncCollectionResponse, SyncLoginRequest};
use deck_streak_engine_core::dispatch::{Dispatcher, Refusal};
use deck_streak_engine_core::full_sync::{Counted, Direction, Ready};
use deck_streak_engine_core::gesture::{GestureRefusal, OwnerGesture, Target};
use deck_streak_engine_core::one_way::{self, Reason, Unwritten};
use deck_streak_engine_core::table::{ExemptWrite, Transport};
use prost::Message;
use support::sync_server::{self, PASSWORD, SERVER, SyncServer, USERNAME};

/// `BackendSyncService.SyncLogin`.
const SYNC_LOGIN: (u32, u32) = (1, 3);
/// `BackendSyncService.SyncCollection`.
const SYNC_COLLECTION: (u32, u32) = (1, 5);

/// A statement whose minimum is above the level this tree builds.
const BELOW_STATEMENT: &[u8] = br#"{"minimum_client_level":2}"#;
/// A statement whose minimum equals the level this tree builds: the edge that still admits.
const ADMITTING_STATEMENT: &[u8] = br#"{"minimum_client_level":1}"#;

/// R6's sentence for a client below the minimum.
const BELOW: &str = "This version of DeckStreak is older than the oldest the sync service accepts. Update DeckStreak to sync.";
/// R6's sentence when no statement was read.
const UNREAD: &str = "A network error occurred. The sync service's oldest accepted version could not be read, so nothing was synced.";
/// R6's sentence when a statement was read and does not decode.
const UNDECODABLE: &str = "The sync service's statement of the oldest version it accepts could not be read, so nothing was synced.";

/// One case of A2: what it is, the statement handed (`None` for none at all), and the sentence
/// every sync pair is refused with.
type Case = (&'static str, Option<Option<&'static [u8]>>, &'static str);

/// A synthetic account: short values no refusal may repeat.
const USER: &str = "learner-one";
/// The synthetic account's password.
const SECRET: &str = "hunter-two";

/// A loopback endpoint on a port nothing listens on: a call the engine makes there fails at once
/// with the engine's own network refusal.
fn closed_endpoint() -> String {
    let listener =
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port to close again");
    let port = listener.local_addr().expect("the bound port reads").port();
    drop(listener);
    format!("http://{}:{port}/", Ipv4Addr::LOCALHOST)
}

/// The encoded `SyncLoginRequest` for the synthetic account and `endpoint`.
fn login(endpoint: &str) -> Vec<u8> {
    SyncLoginRequest {
        username: USER.to_owned(),
        password: SECRET.to_owned(),
        endpoint: Some(endpoint.to_owned()),
    }
    .encode_to_vec()
}

/// A sync auth naming `endpoint`, with a host key no server issued.
fn sync_auth(endpoint: &str) -> SyncAuth {
    SyncAuth {
        hkey: String::from("planted-host-key"),
        endpoint: Some(endpoint.to_owned()),
        io_timeout_secs: None,
    }
}

/// The encoded normal sync naming `endpoint`, without media.
fn normal_sync(endpoint: &str) -> Vec<u8> {
    SyncCollectionRequest {
        auth: Some(sync_auth(endpoint)),
        sync_media: false,
    }
    .encode_to_vec()
}

/// A refusal's kind and message, decoded with the engine's own schema.
fn decoded(error: &[u8]) -> (Kind, String) {
    let refusal = BackendError::decode(error).expect("a refusal decodes as the engine's error");
    (refusal.kind(), refusal.message)
}

/// The kind and message of the engine-shaped refusal a call answered, or what else it answered.
fn refused(reply: Result<Vec<u8>, Refusal>) -> Result<(Kind, String), String> {
    match reply {
        Err(Refusal::Engine { error }) => Ok(decoded(&error)),
        other => Err(format!("not an engine-shaped refusal: {other:?}")),
    }
}

/// The core's refusal of `sentence`, as a test expects it.
fn core_refusal(sentence: &str) -> (Kind, String) {
    (Kind::InvalidInput, sentence.to_owned())
}

/// A dispatcher on `transport` started from the default init.
fn started(transport: Transport) -> Dispatcher {
    Dispatcher::start(transport, &[]).expect("the engine starts from the default init")
}

/// A native dispatcher with `synthetic`'s collection open.
fn open(synthetic: &support::Synthetic) -> Dispatcher {
    let dispatcher = started(Transport::Native);
    dispatcher
        .run(3, 0, &support::open_request(synthetic))
        .expect("the dispatcher opens the collection");
    dispatcher
}

/// The engine's answer to a normal sync that needs a full sync: both directions offered.
fn full_sync_required() -> SyncCollectionResponse {
    SyncCollectionResponse {
        required: ChangesRequired::FullSync as i32,
        ..SyncCollectionResponse::default()
    }
}

/// The one-way fetch of the server's copy into `copy` through a private engine: the refusal it
/// answered in the engine's shape, or what else it answered.
fn fetched(dispatcher: &Dispatcher, endpoint: &str, copy: &Path) -> Result<(Kind, String), String> {
    match one_way::count(
        dispatcher,
        &full_sync_required(),
        &sync_auth(endpoint),
        copy,
    ) {
        Err(Reason::Engine(Refusal::Engine { error })) => Ok(decoded(&error)),
        other => Err(format!("not an engine-shaped refusal: {other:?}")),
    }
}

/// Whether a refusal is the engine's own answer to a call that reached it: a network refusal, not
/// one of the core's sentences.
fn reached_the_engine(read: &Result<(Kind, String), String>) -> bool {
    matches!(read, Ok((Kind::NetworkError, message))
        if ![BELOW, UNREAD, UNDECODABLE].contains(&message.as_str()))
}

/// The test's own engine handle on a closed collection file, for the server's seed.
fn engine(collection: &Path) -> Collection {
    CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the test's collection")
}

/// The synthetic account logged in at `server` by the engine's own login, called directly.
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

/// A fresh HTTP client of the engine's own type, built by its `Default`.
fn engine_client<Client: Default>() -> Client {
    Client::default()
}

/// The server's collection replaced by `collection`'s, by the engine's own full upload.
fn seed(auth: &EngineAuth, collection: &Path) {
    sync_server::runtime()
        .block_on(engine(collection).full_upload(auth.clone(), engine_client()))
        .unwrap_or_else(|error| panic!("the engine's own upload: {error}"));
}

/// The download counted over the server's copy, backed up and ready for its write, with the
/// dispatcher admitted by an admitting statement for the reads that precede the write.
fn download_ready(dispatcher: &Dispatcher, auth: &SyncAuth, dir: &Path) -> Ready {
    dispatcher.handshake(Some(ADMITTING_STATEMENT));
    let copy = dir.join("copy.anki2");
    let counted: Counted = one_way::count(dispatcher, &full_sync_required(), auth, &copy)
        .expect("the server's collection is fetched into the empty copy and counted");
    let confirmed = counted
        .confirm(Direction::Download)
        .expect("the offer admits the download");
    one_way::back_up(dispatcher, confirmed, &dir.join("backup.anki2"), &copy)
        .expect("a backup that holds the device is accepted")
        .download_ready()
        .expect("a download is ready once its backup holds the device")
}

/// The owner's tap on the one-way sync of the open collection.
fn one_way_tap() -> OwnerGesture {
    OwnerGesture::from_tap(ExemptWrite::OneWaySync, Target::Collection)
        .expect("the one-way sync takes the open collection as its target")
}

#[test]
fn a_client_below_the_minimum_is_refused_every_sync_before_the_engine() {
    if sync_server::role().as_deref() == Some(SERVER) {
        return sync_server::serve();
    }
    let endpoint = closed_endpoint();

    let native = started(Transport::Native);
    native.handshake(Some(BELOW_STATEMENT));
    let web = started(Transport::Web);
    web.handshake(Some(BELOW_STATEMENT));
    let calls = [
        (
            "the native sync login",
            refused(native.run(SYNC_LOGIN.0, SYNC_LOGIN.1, &login(&endpoint))),
        ),
        (
            "the web sync login",
            refused(web.run(SYNC_LOGIN.0, SYNC_LOGIN.1, &login(&endpoint))),
        ),
        (
            "the web normal sync",
            refused(web.run(
                SYNC_COLLECTION.0,
                SYNC_COLLECTION.1,
                &normal_sync(&endpoint),
            )),
        ),
    ];
    let expected: Vec<_> = calls
        .iter()
        .map(|(call, _)| (*call, Ok(core_refusal(BELOW))))
        .collect();
    assert_eq!(
        calls.to_vec(),
        expected,
        "below the minimum, each sync call is refused before the engine with the below sentence"
    );

    let server =
        SyncServer::start("a_client_below_the_minimum_is_refused_every_sync_before_the_engine");
    let (engine_auth, auth) = logged_in(&server);
    let donor = support::synthetic("handshake-below-donor");
    seed(&engine_auth, &donor.collection);
    let device = support::synthetic("handshake-below-device");
    let dispatcher = open(&device);
    let ready = download_ready(&dispatcher, &auth, &device.dir);
    let ids_before = dispatcher.id_sets().expect("the device's ids read");
    let unsynced_before = dispatcher
        .unsynced()
        .expect("the device's unsynced state reads");

    dispatcher.handshake(Some(BELOW_STATEMENT));
    let written = one_way::write(&dispatcher, ready, one_way_tap(), &auth);

    let read = match written {
        Err(Unwritten::Refused(Reason::Gesture(GestureRefusal::Engine { error }))) => {
            Ok(decoded(&error))
        }
        other => Err(format!("not an engine-shaped refusal: {other:?}")),
    };
    assert_eq!(
        read,
        Ok(core_refusal(BELOW)),
        "below the minimum, the one-way write is refused before the engine with the below sentence"
    );
    assert_eq!(
        dispatcher.id_sets(),
        Ok(ids_before),
        "the refused write left the device's reviews, cards and notes as they were"
    );
    assert_eq!(
        dispatcher.unsynced(),
        Ok(unsynced_before),
        "the refused write left what the device has not synced as it was"
    );
    support::examined("sync call(s) below the minimum", calls.to_vec());
}

#[test]
fn an_unread_or_unreadable_statement_refuses_every_sync() {
    let endpoint = closed_endpoint();
    let statements: [Case; 4] = [
        ("a dispatcher never handed a statement", None, UNREAD),
        ("no answer read", Some(None), UNREAD),
        (
            "a body that is not JSON",
            Some(Some(b"not json")),
            UNDECODABLE,
        ),
        ("an empty body", Some(Some(b"")), UNDECODABLE),
    ];
    let mut judged = Vec::new();
    let mut expected = Vec::new();
    for (at, (case, statement, sentence)) in statements.into_iter().enumerate() {
        let synthetic = support::synthetic(&format!("handshake-unread-{at}"));
        let native = open(&synthetic);
        let web = started(Transport::Web);
        if let Some(statement) = statement {
            native.handshake(statement);
            web.handshake(statement);
        }
        let calls = [
            (
                "the native sync login",
                refused(native.run(SYNC_LOGIN.0, SYNC_LOGIN.1, &login(&endpoint))),
            ),
            (
                "the web sync login",
                refused(web.run(SYNC_LOGIN.0, SYNC_LOGIN.1, &login(&endpoint))),
            ),
            (
                "the web normal sync",
                refused(web.run(
                    SYNC_COLLECTION.0,
                    SYNC_COLLECTION.1,
                    &normal_sync(&endpoint),
                )),
            ),
            (
                "the one-way fetch",
                fetched(&native, &endpoint, &synthetic.dir.join("copy.anki2")),
            ),
        ];
        for (call, read) in calls {
            judged.push((case, call, read));
            expected.push((case, call, Ok(core_refusal(sentence))));
        }
    }
    assert_eq!(
        judged, expected,
        "an unread, an undecodable and a never-read statement each refuse every sync pair with \
         its sentence"
    );
    support::examined("sync call(s) without an admitting statement", judged);
}

#[test]
fn the_latest_statement_decides_and_a_private_engine_obeys_it() {
    let endpoint = closed_endpoint();

    let native = started(Transport::Native);
    native.handshake(Some(ADMITTING_STATEMENT));
    native.handshake(Some(BELOW_STATEMENT));
    assert_eq!(
        refused(native.run(SYNC_LOGIN.0, SYNC_LOGIN.1, &login(&endpoint))),
        Ok(core_refusal(BELOW)),
        "a below statement after an admitting one refuses the login again"
    );

    native.handshake(Some(ADMITTING_STATEMENT));
    let admitted = refused(native.run(SYNC_LOGIN.0, SYNC_LOGIN.1, &login(&endpoint)));
    assert!(
        reached_the_engine(&admitted),
        "an admitting statement, its minimum equal to the level, lets the login reach the engine, \
         which answers its own network refusal: {admitted:?}"
    );

    let synthetic = support::synthetic("handshake-private");
    let parent = open(&synthetic);
    parent.handshake(Some(BELOW_STATEMENT));
    let below = fetched(&parent, &endpoint, &synthetic.dir.join("copy-below.anki2"));
    parent.handshake(Some(ADMITTING_STATEMENT));
    let admitting = fetched(
        &parent,
        &endpoint,
        &synthetic.dir.join("copy-admitting.anki2"),
    );
    parent.handshake(Some(BELOW_STATEMENT));
    let below_again = fetched(
        &parent,
        &endpoint,
        &synthetic.dir.join("copy-below-again.anki2"),
    );
    assert_eq!(
        below,
        Ok(core_refusal(BELOW)),
        "the private engine of a parent below the minimum refuses the fetch"
    );
    assert!(
        reached_the_engine(&admitting),
        "the private engine of a parent admitted since reaches the engine: {admitting:?}"
    );
    assert_eq!(
        below_again,
        Ok(core_refusal(BELOW)),
        "the private engine obeys the parent's latest statement, below again"
    );
    support::examined("private fetch(es)", vec![below, admitting, below_again]);
}
