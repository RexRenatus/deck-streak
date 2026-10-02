//! The owner's quick capture route over the shell's layers (SPEC-118 A15 to A18; R10): one text
//! lands in the vault inbox as the Mini App's stub, a journal capture lands there too, a retry with
//! the same capture id answers the first name and writes nothing, and the route is served to the
//! owner's live session alone with its text bounded at 1 to 4000 characters.
//!
//! The launch payloads are SPEC-024's, as in the drill routes' test. The vault is a temporary
//! folder; a retry on a later UTC day is the vault's own A24, whose unique is this route's
//! `capture_id` (ruling (h)).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_coordination::inbox_capture::{InboxCaptures, LayoutInForce, RealFs};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Db, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

/// The synthetic bot token Python signed the payloads for.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// The synthetic owner.
const OWNER: i64 = 4242;
/// The owner's launch data, signed for [`BOT_TOKEN`].
const OWNER_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=79502d49032542c5030e80b666d03f5af53e053f97d387806adb0aa843ddeb2d",
);
/// 2025-01-15T03:30:10Z, ten seconds after the payloads were signed, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 256 * 1024;
/// The inbox folder of the layout the tests write into.
const INBOX: &str = "90-Inbox";
/// The route's path.
const CAPTURES: &str = "/api/inbox/captures";
/// A capture id as the Mini App mints one: 32 hexadecimal digits.
const CAPTURE_ID: &str = "6f1d2c9a0b1c4d5e8f9a0b1c2d3e4f50";

/// An answer: its status, its headers and its body.
struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl Answer {
    /// The body as JSON.
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).expect("a JSON body")
    }
}

/// `method` on `path` with `headers` and `body`, through the whole app.
async fn send(
    app: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: String,
) -> Answer {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(body))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router is infallible");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    Answer {
        status,
        headers,
        body: String::from_utf8(bytes.to_vec()).expect("a UTF-8 body"),
    }
}

/// The owner's session cookie, from a same-origin JSON handshake: its `name=value`.
async fn owner_cookie(app: &Router) -> String {
    let answer = send(
        app,
        "POST",
        "/api/session",
        &[
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
        ],
        json!({ "init_data": OWNER_PAYLOAD }).to_string(),
    )
    .await;
    assert_eq!(answer.status, StatusCode::OK, "the owner's handshake");
    answer
        .headers
        .get(SET_COOKIE)
        .expect("a Set-Cookie")
        .to_str()
        .expect("a text header")
        .split(';')
        .next()
        .expect("a name and value")
        .trim()
        .to_owned()
}

/// `POST /api/inbox/captures` with a JSON `body` carrying `cookie`, same-origin.
async fn post(app: &Router, body: &Value, cookie: Option<&str>) -> Answer {
    let mut headers = vec![
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
    ];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(app, "POST", CAPTURES, &headers, body.to_string()).await
}

/// A capture's body.
fn capture(capture_id: &str, kind: &str, text: &str) -> Value {
    json!({ "capture_id": capture_id, "kind": kind, "text": text })
}

/// The API over a temporary database and a temporary vault whose inbox exists; the vault root and
/// the clock.
async fn app(scratch: &TempDir) -> (Router, PathBuf, Arc<ManualClock>) {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let vault = scratch.path().join("vault");
    fs::create_dir_all(vault.join(INBOX)).expect("the inbox folder");
    let layout = LayoutInForce {
        inbox: INBOX.to_owned(),
        journal: Vec::new(),
    };
    let captures = InboxCaptures::new(RealFs, vault.clone(), layout);
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock.clone(), StudyDayRule::default());
    let readiness = Readiness::new();
    readiness.database_opened(db);
    let state = ApiState::new(readiness)
        .with_owner(access)
        .with_inbox(Arc::new(captures));
    (router(state), vault, clock)
}

/// The names in `folder`, sorted.
fn files(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .expect("the folder lists")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .into_string()
                .expect("a UTF-8 name")
        })
        .collect();
    names.sort();
    examined_may_be_empty("file name(s) in the folder", names)
}

/// Prints how many items a listing examined (the tdd pack's examined contract) and accepts zero:
/// a test here asserts that a folder stayed empty, so zero is an answer it expects.
fn examined_may_be_empty<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    items
}

/// The Mini App's stub for `kind` and `text`, captured at [`STARTED_AT`]: R10's keys, spelled out
/// rather than computed, so the route is judged against the stub the owner reads.
fn stub(kind: &str, text: &str) -> String {
    format!(
        "---\nstatus: captured\nsource: miniapp\nkind: {kind}\ncaptured: \
         2025-01-15T03:30:10+00:00\ntags: [inbox, miniapp-capture]\n---\n\nCaptured via the Mini \
         App.\n\n{text}\n"
    )
}

#[tokio::test]
async fn a_quick_capture_writes_the_miniapp_stub() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let (app, vault, _clock) = app(&scratch).await;
    let cookie = owner_cookie(&app).await;
    let name = format!("2025-01-15-text-{CAPTURE_ID}.md");

    let answer = post(
        &app,
        &capture(CAPTURE_ID, "text", "  call the clinic \n"),
        Some(&cookie),
    )
    .await;
    assert_eq!(answer.status, StatusCode::CREATED, "{}", answer.body);
    assert_eq!(answer.json(), json!({ "name": name }));
    assert_eq!(files(&vault.join(INBOX)), vec![name.clone()]);
    assert_eq!(
        fs::read_to_string(vault.join(INBOX).join(&name)).expect("the stub"),
        stub("text", "call the clinic"),
        "the stub holds the trimmed text after R10's keys"
    );
}

#[tokio::test]
async fn a_journal_quick_capture_lands_in_the_inbox() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let (app, vault, _clock) = app(&scratch).await;
    let cookie = owner_cookie(&app).await;
    let name = format!("2025-01-15-journal-{CAPTURE_ID}.md");

    let answer = post(
        &app,
        &capture(CAPTURE_ID, "journal", "a calm morning"),
        Some(&cookie),
    )
    .await;
    assert_eq!(answer.status, StatusCode::CREATED, "{}", answer.body);
    assert_eq!(answer.json(), json!({ "name": name }));
    assert_eq!(
        fs::read_to_string(vault.join(INBOX).join(&name)).expect("the stub"),
        stub("journal", "a calm morning"),
        "the journal capture is an inbox stub of kind journal"
    );
    assert_eq!(
        files(&vault),
        vec![INBOX.to_owned()],
        "it lands in the inbox, and no other folder is made"
    );
}

#[tokio::test]
async fn a_retried_quick_capture_answers_the_same_name() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let (app, vault, clock) = app(&scratch).await;
    let cookie = owner_cookie(&app).await;
    let name = format!("2025-01-15-text-{CAPTURE_ID}.md");

    let first = post(
        &app,
        &capture(CAPTURE_ID, "text", "call the clinic"),
        Some(&cookie),
    )
    .await;
    assert_eq!(first.status, StatusCode::CREATED, "{}", first.body);

    clock.advance(Duration::from_secs(5));
    let retry = post(
        &app,
        &capture(CAPTURE_ID, "text", "call the clinic, sent again"),
        Some(&cookie),
    )
    .await;
    assert_eq!(retry.status, StatusCode::OK, "{}", retry.body);
    assert_eq!(
        retry.json(),
        json!({ "name": name, "already_captured": true }),
        "the retry answers the first name"
    );
    assert_eq!(
        files(&vault.join(INBOX)),
        vec![name.clone()],
        "the retry wrote nothing"
    );
    assert_eq!(
        fs::read_to_string(vault.join(INBOX).join(&name)).expect("the stub"),
        stub("text", "call the clinic"),
        "the first capture's stub is unchanged"
    );
}

#[tokio::test]
async fn the_quick_capture_is_owner_only_and_bounds_its_text() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let (app, vault, _clock) = app(&scratch).await;
    let cookie = owner_cookie(&app).await;
    let longest = "é".repeat(4000);

    // 201 at 4000 characters, with the session: the control of the 401 below.
    let saved = post(&app, &capture(CAPTURE_ID, "text", &longest), Some(&cookie)).await;
    assert_eq!(
        saved.status,
        StatusCode::CREATED,
        "4000 characters with the session: {}",
        saved.body
    );
    let written = files(&vault.join(INBOX));
    assert_eq!(written.len(), 1, "the 4000-character capture was written");

    // 401: the same capture without the session.
    let other = "0a1b2c3d4e5f60718293a4b5c6d7e8f9";
    let refused = post(&app, &capture(other, "text", &longest), None).await;
    assert_eq!(
        refused.status,
        StatusCode::UNAUTHORIZED,
        "without the session: {}",
        refused.body
    );
    assert_eq!(
        files(&vault.join(INBOX)),
        written,
        "the refusal wrote nothing"
    );

    // 422 at 0 characters, after the trim too.
    for empty in ["", " \n\t "] {
        let refused = post(&app, &capture(other, "text", empty), Some(&cookie)).await;
        assert_eq!(
            (refused.status, refused.json()),
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "reason": "text_out_of_bounds" })
            ),
            "{} characters",
            empty.chars().count()
        );
    }

    // 422 at 4001 characters.
    let too_long = format!("{longest}x");
    let refused = post(&app, &capture(other, "text", &too_long), Some(&cookie)).await;
    assert_eq!(
        (refused.status, refused.json()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "reason": "text_out_of_bounds" })
        ),
        "4001 characters"
    );
    assert_eq!(
        files(&vault.join(INBOX)),
        written,
        "no refusal wrote a file"
    );
}

#[tokio::test]
async fn the_quick_capture_refuses_a_bad_request_and_a_missing_vault() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let (app, vault, _clock) = app(&scratch).await;
    let cookie = owner_cookie(&app).await;

    // A capture id that is not its own safe form would share a retry key with another capture.
    let longest_id = "a".repeat(32);
    let too_long_id = "a".repeat(33);
    for (capture_id, kind, reason) in [
        ("", "text", "invalid_capture_id"),
        ("6f1d 2c9a", "text", "invalid_capture_id"),
        ("6f1d2c9a.md", "text", "invalid_capture_id"),
        (too_long_id.as_str(), "text", "invalid_capture_id"),
        (CAPTURE_ID, "photo", "unknown_kind"),
        (CAPTURE_ID, "Text", "unknown_kind"),
    ] {
        let refused = post(&app, &capture(capture_id, kind, "a note"), Some(&cookie)).await;
        assert_eq!(
            (refused.status, refused.json()),
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "reason": reason })
            ),
            "{capture_id:?} of kind {kind:?}"
        );
    }
    assert!(
        files(&vault.join(INBOX)).is_empty(),
        "no refusal wrote a file"
    );

    // 503 when the inbox folder is gone, and it is never created.
    fs::remove_dir(vault.join(INBOX)).expect("the empty inbox folder is removed");
    let missing = post(&app, &capture(CAPTURE_ID, "text", "a note"), Some(&cookie)).await;
    assert_eq!(
        (missing.status, missing.json()),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "reason": "vault_missing" })
        )
    );
    assert!(files(&vault).is_empty(), "no folder was created");

    // The control: once the inbox exists again, the longest safe capture id is accepted.
    fs::create_dir(vault.join(INBOX)).expect("the inbox folder returns");
    let saved = post(&app, &capture(&longest_id, "text", "a note"), Some(&cookie)).await;
    assert_eq!(saved.status, StatusCode::CREATED, "{}", saved.body);
}
