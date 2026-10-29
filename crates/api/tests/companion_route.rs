//! The owner's scheduling companion route over the shell's layers (SPEC-511 C1 to C7): the owner's
//! session first, then the directory; three literal names served with their exact bytes, their
//! type and no cache; every other name, traversal forms among them, refused; the cap; and the
//! directory setting.
//!
//! Every expected status, type, reason and header is written out here by hand, never read from the
//! crate. The session cookie is opened on the access's own sessions, as the handshake would.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::FromRef;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use axum::routing::get;
use deck_streak_api::companion_routes::{COMPANION_DIR, COMPANION_FILE_CAP, CompanionDirectory};
use deck_streak_api::{ApiError, ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::session::SESSION_COOKIE;
use deck_streak_identity::{Freshness, Owner, OwnerGate, Sessions, WebAppKey};
use deck_streak_kernel::{
    Environment, ManualClock, SettingsError, StudyDayRule, TelegramUserId, UtcMillis,
};
use serde_json::Value;
use tower::ServiceExt;

/// The synthetic bot token the gate is keyed with; no launch is signed in this file.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// The synthetic owner.
const OWNER: i64 = 4242;
/// 2025-01-15T03:30:10Z in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The most a test reads of a body: the cap and a margin, so a body one byte over would show.
const BODY_READ_LIMIT: usize = 17 * 1024 * 1024;
/// 16 MiB, written out apart from the crate's constant.
const CAP: u64 = 16_777_216;

/// The three names: the path, the file in the directory and the content type, written out here.
const SERVED: [(&str, &str, &str); 3] = [
    (
        "/api/companion/v1/manifest.json",
        "manifest.json",
        "application/json",
    ),
    (
        "/api/companion/v1/module.wasm",
        "module.wasm",
        "application/wasm",
    ),
    (
        "/api/companion/v1/weights.bin",
        "weights.bin",
        "application/octet-stream",
    ),
];

/// Paths that are not one of the three names: other names, other cases, other versions, other
/// shapes of the prefix, and traversal forms, raw and percent-encoded.
const PLANTED: [&str; 24] = [
    "/api/companion/v1/state.bin",
    "/api/companion/v1/bindings.js",
    "/api/companion/v1/index.html",
    "/api/companion/v1/secret.txt",
    "/api/companion/v1/",
    "/api/companion/v1",
    "/api/companion/",
    "/api/companion/v2/module.wasm",
    "/api/companion/v1/Module.wasm",
    "/api/companion/v1/MANIFEST.JSON",
    "/api/companion/v1/module.wasm/",
    "/api/companion/v1/module.wasm.bak",
    "/api/companion/v1/module.wasm%00",
    "/api/companion/v1/module%2ewasm",
    "/api/companion/v1/%6dodule.wasm",
    "/api/companion/v1//module.wasm",
    "/api/companion/v1/./module.wasm",
    "/api/companion/v1/../v1/module.wasm",
    "/api/companion/v1/../secret.txt",
    "/api/companion/v1/..%2fsecret.txt",
    "/api/companion/v1/%2e%2e/secret.txt",
    "/api/companion/v1/%2e%2e%2fsecret.txt",
    "/api/companion/v1/..%5csecret.txt",
    "/api/companion/v1/....//secret.txt",
];

/// The bytes each served file holds in these tests: distinct, and each with a zero byte and a byte
/// above 0x7f, so a text decoding on the way would show.
fn contents(name: &str) -> Vec<u8> {
    match name {
        "manifest.json" => br#"{"module":"00","weights":"ff"}"#.to_vec(),
        "module.wasm" => [b"\0asm\x01\0\0\0".as_slice(), &[0xc3, 0x28, 0x00, 0xff]].concat(),
        "weights.bin" => (0..=255).rev().collect(),
        other => format!("planted {other}").into_bytes(),
    }
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The access the daemon builds for the synthetic owner, on a manual clock.
fn access() -> OwnerAccess {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    OwnerAccess::new(gate, clock, StudyDayRule::default())
}

/// The API as the daemon builds it, serving the companion from `directory` when one is named; and
/// the access's own sessions.
fn app(directory: Option<&Path>) -> (Sessions, Router) {
    let access = access();
    let sessions = Sessions::from_ref(&access);
    let mut state = ApiState::new(Readiness::new()).with_owner(access);
    if let Some(directory) = directory {
        state = state
            .with_companion(CompanionDirectory::new(directory).expect("an absolute directory"));
    }
    (sessions, router(state))
}

/// A `telegram` session's cookie.
fn telegram(sessions: &Sessions) -> String {
    let session = sessions
        .open(Owner::new(TelegramUserId::new(OWNER)))
        .expect("a telegram session opens");
    format!("{SESSION_COOKIE}={}", session.expose())
}

/// A `link` session's cookie.
fn link(sessions: &Sessions) -> String {
    let session = sessions
        .open_link(Owner::new(TelegramUserId::new(OWNER)))
        .expect("a link session opens");
    format!("{SESSION_COOKIE}={}", session.expose())
}

/// A directory holding each of `names` with its test contents.
fn directory_with(names: &[&str]) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for name in names {
        fs::write(directory.path().join(name), contents(name)).expect("a planted file");
    }
    directory
}

/// An answer: its status, its headers and its body.
struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Answer {
    fn header(&self, name: &str) -> Option<&[u8]> {
        self.headers.get(name).map(HeaderValue::as_bytes)
    }

    fn cache(&self) -> Option<&[u8]> {
        self.headers.get(CACHE_CONTROL).map(HeaderValue::as_bytes)
    }

    fn reason(&self) -> Value {
        serde_json::from_slice::<Value>(&self.body)
            .map_or(Value::Null, |body| body["reason"].clone())
    }
}

/// `GET path`, with `cookie` when there is one.
async fn get_path(app: &Router, path: &str, cookie: Option<&str>) -> Answer {
    let mut request = Request::builder().method("GET").uri(path);
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("a well-formed request"))
        .await
        .expect("the router is infallible");
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body")
        .to_vec();
    Answer {
        status,
        headers,
        body,
    }
}

/// The paths of `candidates` that answer 200 to the owner, in order.
async fn served(app: &Router, cookie: &str, candidates: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for path in candidates {
        if get_path(app, path, Some(cookie)).await.status == StatusCode::OK {
            found.push((*path).to_owned());
        }
    }
    found
}

/// The census's judgement: the route serves exactly the three names, each path that answered beyond
/// them named.
fn judge(served: &[String]) -> Result<(), String> {
    let wanted: Vec<String> = SERVED
        .iter()
        .map(|(path, _, _)| (*path).to_owned())
        .collect();
    let extra: Vec<&String> = served
        .iter()
        .filter(|path| !wanted.contains(path))
        .collect();
    let missing: Vec<&String> = wanted
        .iter()
        .filter(|path| !served.contains(path))
        .collect();
    if extra.is_empty() && missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "served beyond the three: {extra:?}; not served: {missing:?}"
        ))
    }
}

/// Every candidate the census asks: the three names, then the planted ones.
fn candidates() -> Vec<&'static str> {
    SERVED
        .iter()
        .map(|(path, _, _)| *path)
        .chain(PLANTED)
        .collect()
}

/// A directory holding a file of every name the census asks for, the three and each planted
/// name's last segment, and a secret beside it in its parent.
fn crowded() -> (tempfile::TempDir, std::path::PathBuf) {
    let parent = tempfile::tempdir().expect("a temporary directory");
    fs::write(parent.path().join("secret.txt"), b"the parent's secret").expect("a secret");
    let inner = parent.path().join("companion");
    fs::create_dir(&inner).expect("the companion directory");
    let mut names: Vec<&str> = SERVED.iter().map(|(_, name, _)| *name).collect();
    names.extend([
        "state.bin",
        "bindings.js",
        "index.html",
        "secret.txt",
        "module.wasm.bak",
    ]);
    for name in names {
        fs::write(inner.join(name), contents(name)).expect("a planted file");
    }
    (parent, inner)
}

#[tokio::test]
async fn no_session_gives_401_and_an_owner_with_no_directory_or_no_file_gives_404() {
    let full = directory_with(&["manifest.json", "module.wasm", "weights.bin"]);
    let (sessions, app_full) = app(Some(full.path()));
    let linked = link(&sessions);
    let unknown = format!("{SESSION_COOKIE}=unknown");
    let mut refusals = Vec::new();
    for (path, _, _) in SERVED {
        for (what, cookie) in [
            ("no cookie", None),
            ("an unknown session", Some(unknown.as_str())),
            ("a link session", Some(linked.as_str())),
        ] {
            refusals.push((path, what, cookie));
        }
    }
    for (path, what, cookie) in examined("requests with no owner session", refusals) {
        let answer = get_path(&app_full, path, cookie).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{what} on {path}");
        assert_eq!(
            answer.reason(),
            "no_session",
            "{what} on {path} is told why"
        );
        assert_ne!(
            answer.body,
            contents(path.rsplit('/').next().unwrap_or_default())
        );
    }

    // An owner, with no directory named, with an empty directory, and with a directory whose files
    // are directories: 404 `companion_absent` on each name.
    let (sessions_off, app_off) = app(None);
    let empty = directory_with(&[]);
    let (sessions_empty, app_empty) = app(Some(empty.path()));
    let shaped = tempfile::tempdir().expect("a temporary directory");
    for (_, name, _) in SERVED {
        fs::create_dir(shaped.path().join(name)).expect("a directory where a file should be");
    }
    let (sessions_shaped, app_shaped) = app(Some(shaped.path()));
    let cases = examined(
        "owner requests with no file to serve",
        vec![
            ("no directory", &app_off, telegram(&sessions_off)),
            ("an empty directory", &app_empty, telegram(&sessions_empty)),
            (
                "directories, not files",
                &app_shaped,
                telegram(&sessions_shaped),
            ),
        ],
    );
    for (what, app, cookie) in cases {
        for (path, _, _) in SERVED {
            let answer = get_path(app, path, Some(&cookie)).await;
            assert_eq!(answer.status, StatusCode::NOT_FOUND, "{what} on {path}");
            assert_eq!(answer.reason(), "companion_absent", "{what} on {path}");
        }
    }
}

#[tokio::test]
async fn each_of_the_three_names_answers_its_exact_bytes_its_type_and_private_no_store() {
    let full = directory_with(&["manifest.json", "module.wasm", "weights.bin"]);
    let (sessions, app) = app(Some(full.path()));
    let owner = telegram(&sessions);
    for (path, name, content_type) in examined("served names", SERVED.to_vec()) {
        let answer = get_path(&app, path, Some(&owner)).await;
        assert_eq!(answer.status, StatusCode::OK, "{path}");
        assert_eq!(
            answer.body,
            contents(name),
            "{path} answers the file's exact bytes"
        );
        assert_eq!(
            answer.header(CONTENT_TYPE.as_str()),
            Some(content_type.as_bytes()),
            "{path}"
        );
        assert_eq!(
            answer.header("x-content-type-options"),
            Some(b"nosniff".as_slice()),
            "{path}"
        );
        assert_eq!(
            answer.cache(),
            Some(b"private, no-store".as_slice()),
            "{path}"
        );
    }
}

#[tokio::test]
async fn every_other_name_and_every_traversal_form_gives_404_and_three_names_are_served() {
    let (parent, inner) = crowded();
    let (sessions, app) = app(Some(&inner));
    let owner = telegram(&sessions);
    let asked = examined("asked paths", candidates());
    for path in PLANTED {
        let answer = get_path(&app, path, Some(&owner)).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND, "{path}");
        assert_ne!(answer.body, b"the parent's secret".to_vec(), "{path}");
    }
    let found = served(&app, &owner, &asked).await;
    assert_eq!(
        judge(&found),
        Ok(()),
        "the route serves exactly the three names"
    );
    drop(parent);
}

#[tokio::test]
async fn a_route_that_serves_a_fourth_name_is_refused_by_the_census() {
    let (parent, inner) = crowded();
    let (sessions, app) = app(Some(&inner));
    let owner = telegram(&sessions);
    // The planted route: the same API, and beside it a fourth name served from the same directory.
    let fourth = fs::read(inner.join("state.bin")).expect("the planted fourth file");
    let planted = app.merge(Router::new().route(
        "/api/companion/v1/state.bin",
        get(move || {
            let body = fourth.clone();
            async move { body }
        }),
    ));
    let asked = examined("asked paths", candidates());
    let found = served(&planted, &owner, &asked).await;
    let verdict = judge(&found);
    assert!(
        verdict
            .as_ref()
            .is_err_and(|why| why.contains("/api/companion/v1/state.bin")),
        "the census refuses a route that serves a fourth name, by its path: {verdict:?}"
    );
    drop(parent);
}

#[tokio::test]
async fn a_file_over_the_cap_is_refused_and_one_at_the_cap_is_served() {
    assert_eq!(COMPANION_FILE_CAP, CAP, "the cap is 16 MiB");
    let at = tempfile::tempdir().expect("a temporary directory");
    let at_cap: Vec<u8> = (0..CAP)
        .map(|at| u8::try_from(at % 251).expect("below 251"))
        .collect();
    fs::write(at.path().join("weights.bin"), &at_cap).expect("a file at the cap");
    let (sessions, app_at) = app(Some(at.path()));
    let answer = get_path(
        &app_at,
        "/api/companion/v1/weights.bin",
        Some(&telegram(&sessions)),
    )
    .await;
    assert_eq!(answer.status, StatusCode::OK, "a file of the cap is served");
    assert_eq!(answer.body.len(), at_cap.len());
    assert!(answer.body == at_cap, "with its exact bytes");

    let over = tempfile::tempdir().expect("a temporary directory");
    let mut over_cap = at_cap;
    over_cap.push(7);
    fs::write(over.path().join("weights.bin"), &over_cap).expect("a file one byte over");
    let (sessions, app_over) = app(Some(over.path()));
    let answer = get_path(
        &app_over,
        "/api/companion/v1/weights.bin",
        Some(&telegram(&sessions)),
    )
    .await;
    assert_eq!(
        answer.status,
        StatusCode::NOT_FOUND,
        "a file over the cap is refused"
    );
    assert_eq!(answer.reason(), "companion_over_cap");
    assert!(answer.body.len() < 1024, "and none of its bytes are sent");
}

#[tokio::test]
async fn every_refusal_of_the_route_carries_private_no_store() {
    let (sessions_off, app_off) = app(None);
    let over = tempfile::tempdir().expect("a temporary directory");
    fs::write(
        over.path().join("module.wasm"),
        vec![0_u8; usize::try_from(CAP + 1).expect("a size")],
    )
    .expect("a file one byte over");
    let (sessions_over, app_over) = app(Some(over.path()));
    let cases = examined(
        "refusals",
        vec![
            ("no session", &app_off, None, StatusCode::UNAUTHORIZED),
            (
                "no directory",
                &app_off,
                Some(telegram(&sessions_off)),
                StatusCode::NOT_FOUND,
            ),
            (
                "over the cap",
                &app_over,
                Some(telegram(&sessions_over)),
                StatusCode::NOT_FOUND,
            ),
        ],
    );
    for (what, app, cookie, status) in cases {
        let answer = get_path(app, "/api/companion/v1/module.wasm", cookie.as_deref()).await;
        assert_eq!(answer.status, status, "{what}");
        assert_eq!(
            answer.cache(),
            Some(b"private, no-store".as_slice()),
            "{what} is not stored"
        );
    }
}

#[test]
fn the_directory_setting_is_off_unset_and_refuses_start_when_relative() {
    assert_eq!(COMPANION_DIR, "DECKSTREAK_COMPANION_DIR");
    let read = |value: Option<&str>| {
        let env = match value {
            Some(value) => Environment::from_vars([("DECKSTREAK_COMPANION_DIR", value)]),
            None => Environment::from_vars(Vec::<(String, String)>::new()),
        };
        CompanionDirectory::from_env(&env)
    };
    let off = examined("settings read as off", vec![None, Some(""), Some("   ")]);
    for value in off {
        assert!(matches!(read(value), Ok(None)), "{value:?} is off");
    }
    for value in examined(
        "relative values",
        vec!["companion", "./companion", "srv/owner-companion"],
    ) {
        let refused = read(Some(value));
        assert!(
            matches!(
                refused,
                Err(ApiError::Settings(SettingsError::Malformed {
                    setting: "DECKSTREAK_COMPANION_DIR",
                    ..
                }))
            ),
            "{value} refuses start: {refused:?}"
        );
        let said = refused.map(|_| ()).expect_err("a refusal").to_string();
        assert!(said.contains("DECKSTREAK_COMPANION_DIR"), "{said}");
        assert!(
            !said.contains(value),
            "the refusal never names its value: {said}"
        );
    }
    let named = read(Some("/srv/owner-companion")).expect("an absolute directory");
    assert_eq!(
        named.as_ref().map(CompanionDirectory::path),
        Some(Path::new("/srv/owner-companion"))
    );
}
