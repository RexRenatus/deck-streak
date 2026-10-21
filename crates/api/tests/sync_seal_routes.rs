//! The release of the sealing key over the shell's layers: the owner's sessions it serves, the seal
//! id it reads, its own bound, its answer's caching and the cross-site guard, the route off without
//! a seal secret, and the logs it leaves (SPEC-363 R6, R7; A11 to A16).
//!
//! The launch payload was signed by Python's standard `hmac` and `hashlib` for a synthetic bot
//! token, dated 2025-01-15T03:30:00Z, as `session_routes.rs` uses it. Every clock is a
//! `ManualClock` started ten seconds later. The seal secret is built from its parts at run time, and
//! each expected key is identity's own, which `crates/identity/tests/sync_seal.rs` holds to a hand
//! computation.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

use std::fmt;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::FromRef;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, RETRY_AFTER};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use deck_streak_api::sync_seal_routes::{RELEASES_PER_MINUTE, SEAL_KEY_PATH};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::session::SESSION_COOKIE;
use deck_streak_identity::sync_seal::{SealId, SealSecret};
use deck_streak_identity::{Freshness, Owner, OwnerGate, Sessions, WebAppKey};
use deck_streak_kernel::{ManualClock, StudyDayRule, TelegramUserId, UtcMillis, logging};
use tower::ServiceExt;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

/// The synthetic bot token Python signed the payload for.
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
/// 2025-01-15T03:30:10Z, ten seconds after the payload was signed, in milliseconds: ten seconds
/// into its minute.
const STARTED_AT: i64 = 1_736_911_810_000;
/// A seal id: the 16 bytes 0 to 15, in unpadded base64url.
const SEAL_ID: &str = "AAECAwQFBgcICQoLDA0ODw";
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 64 * 1024;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The seal secret's text, made from its parts at run time.
fn secret_text() -> String {
    ["synthetic", "seal", "secret", "for", "these", "tests"].join("-")
}

/// The key identity makes for `seal_id` under [`secret_text`].
fn expected_key(seal_id: &str) -> String {
    SealSecret::new(secret_text().as_bytes())
        .expect("the secret is long enough")
        .key_for(&SealId::parse(seal_id).expect("the id parses"))
        .encoded()
}

/// The API as the daemon builds it, for the synthetic owner, on a manual clock, with the seal
/// secret when `sealed`; and the access's own sessions.
fn app(sealed: bool) -> (Arc<ManualClock>, Sessions, Router) {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock.clone(), StudyDayRule::default());
    let sessions = Sessions::from_ref(&access);
    let mut state = ApiState::new(Readiness::new()).with_owner(access);
    if sealed {
        state = state.with_seal(
            SealSecret::new(secret_text().as_bytes()).expect("the secret is long enough"),
        );
    }
    (clock, sessions, router(state))
}

/// The owner, as a session names them.
fn owner() -> Owner {
    Owner::new(TelegramUserId::new(OWNER))
}

/// The `Cookie` header that presents a session `token`.
fn cookie(token: &str) -> String {
    format!("{SESSION_COOKIE}={token}")
}

/// A `telegram` session's cookie.
fn telegram(sessions: &Sessions) -> String {
    cookie(
        sessions
            .open(owner())
            .expect("a telegram session opens")
            .expose(),
    )
}

/// An answer: its status, its headers and its body.
struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
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

/// The release's JSON body for `seal_id`, as `JSON.stringify` writes it.
fn release_body(seal_id: &str) -> String {
    serde_json::json!({ "seal_id": seal_id }).to_string()
}

/// A release the Worker sends: JSON, same-origin, with `cookie` when there is one.
async fn release(app: &Router, cookie: Option<&str>, body: String) -> Answer {
    let mut headers = vec![
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
    ];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(app, "POST", SEAL_KEY_PATH, &headers, body).await
}

/// The reason code a refusal's JSON body names.
fn reason(answer: &Answer) -> String {
    let body: serde_json::Value = serde_json::from_str(&answer.body).expect("a JSON body");
    body["reason"].as_str().unwrap_or_default().to_owned()
}

/// The key a release's JSON body carries.
fn key(answer: &Answer) -> String {
    let body: serde_json::Value = serde_json::from_str(&answer.body).expect("a JSON body");
    body["key"].as_str().unwrap_or_default().to_owned()
}

#[tokio::test]
async fn the_seal_key_is_released_only_to_an_owner_session() {
    let (_, sessions, app) = app(true);
    let telegram = telegram(&sessions);
    let linked = cookie(
        sessions
            .open_linked(owner(), 1)
            .expect("a linked session opens")
            .expose(),
    );
    let link = cookie(
        sessions
            .open_link(owner())
            .expect("a link session opens")
            .expose(),
    );
    for (what, presented) in examined(
        "owner sessions",
        vec![
            ("a telegram session", &telegram),
            ("a linked session", &linked),
        ],
    ) {
        let answer = release(&app, Some(presented), release_body(SEAL_ID)).await;
        assert_eq!(answer.status, StatusCode::OK, "{what} is released the key");
        assert_eq!(
            key(&answer),
            expected_key(SEAL_ID),
            "{what} is released the sealing key for its seal id"
        );
    }
    let unknown = cookie("unknown");
    let refused = examined(
        "requests with no owner session",
        vec![
            ("a link session", Some(link.as_str())),
            ("an unknown session", Some(unknown.as_str())),
            ("no session", None),
        ],
    );
    for (what, presented) in refused {
        let answer = release(&app, presented, release_body(SEAL_ID)).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{what} is refused");
        assert_eq!(
            reason(&answer),
            "no_session",
            "{what} is refused as no session"
        );
        assert!(key(&answer).is_empty(), "{what} is given no key");
    }
}

#[tokio::test]
async fn a_seal_id_that_is_not_sixteen_bytes_is_refused() {
    let (_, sessions, app) = app(true);
    let telegram = telegram(&sessions);
    let accepted = release(&app, Some(&telegram), release_body(SEAL_ID)).await;
    assert_eq!(
        accepted.status,
        StatusCode::OK,
        "a 16-byte seal id is released"
    );

    let refused = examined(
        "bodies that are not one seal id of 16 bytes",
        vec![
            ("15 bytes", release_body("AAECAwQFBgcICQoLDA0O")),
            ("17 bytes", release_body("AAECAwQFBgcICQoLDA0ODxA")),
            (
                "16 bytes with padding",
                release_body("AAECAwQFBgcICQoLDA0ODw=="),
            ),
            (
                "16 bytes in the standard alphabet",
                release_body("+/ECAwQFBgcICQoLDA0ODw"),
            ),
            ("an empty id", release_body("")),
            ("an id that is a number", r#"{"seal_id": 16}"#.to_owned()),
            ("no seal id", "{}".to_owned()),
            (
                "another field beside the id",
                format!(r#"{{"seal_id": "{SEAL_ID}", "user": 1}}"#),
            ),
            ("a body that is not JSON", format!("seal_id={SEAL_ID}")),
            ("an empty body", String::new()),
        ],
    );
    for (what, body) in refused {
        let answer = release(&app, Some(&telegram), body).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST, "{what} is refused");
        assert_eq!(
            reason(&answer),
            "seal_id_invalid",
            "{what} is refused by its code"
        );
        assert!(key(&answer).is_empty(), "{what} is given no key");
    }

    let oversized = release(
        &app,
        Some(&telegram),
        format!("{:<300}", release_body(SEAL_ID)),
    )
    .await;
    assert_eq!(
        oversized.status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "a body past the release's limit is not read"
    );
}

#[tokio::test]
async fn the_release_has_a_bound_of_its_own() {
    let (clock, sessions, app) = app(true);
    let telegram = telegram(&sessions);
    // The SPEC's 30, written as a number: a window read from the constant would move with it.
    let admitted = examined(
        "releases inside one minute",
        (1..=30_u32).collect::<Vec<_>>(),
    );
    for index in admitted {
        let answer = release(&app, Some(&telegram), release_body(SEAL_ID)).await;
        assert_eq!(answer.status, StatusCode::OK, "release {index} is admitted");
    }

    // Half a second later, 49.5 seconds are left in the minute: whole seconds round up.
    clock.advance(Duration::from_millis(500));
    let refused = release(&app, Some(&telegram), release_body(SEAL_ID)).await;
    assert_eq!(
        refused.status,
        StatusCode::TOO_MANY_REQUESTS,
        "the 31st is refused"
    );
    assert_eq!(reason(&refused), "too_many_releases");
    assert_eq!(RELEASES_PER_MINUTE, 30, "the bound the SPEC names");
    assert_eq!(
        refused.headers.get(RETRY_AFTER).map(HeaderValue::as_bytes),
        Some(b"50".as_slice()),
        "the refusal says how many whole seconds are left in the minute"
    );

    // The handshake's window is the handshake's own: a flood of releases leaves it open.
    let handshake = send(
        &app,
        "POST",
        "/api/session",
        &[
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
        ],
        serde_json::json!({ "init_data": OWNER_PAYLOAD }).to_string(),
    )
    .await;
    assert_eq!(
        handshake.status,
        StatusCode::OK,
        "a handshake is still admitted"
    );

    // The last millisecond of the minute is still inside it.
    clock.advance(Duration::from_millis(49_499));
    let last = release(&app, Some(&telegram), release_body(SEAL_ID)).await;
    assert_eq!(
        last.status,
        StatusCode::TOO_MANY_REQUESTS,
        "the minute is not over"
    );
    assert_eq!(
        last.headers.get(RETRY_AFTER).map(HeaderValue::as_bytes),
        Some(b"1".as_slice()),
        "one millisecond left is one whole second"
    );

    // The next minute admits again.
    clock.advance(Duration::from_millis(1));
    let next = release(&app, Some(&telegram), release_body(SEAL_ID)).await;
    assert_eq!(
        next.status,
        StatusCode::OK,
        "the next minute admits a release"
    );
}

#[tokio::test]
async fn the_release_is_not_stored_and_not_cross_site() {
    let (_, sessions, app) = app(true);
    let telegram = telegram(&sessions);
    let answer = release(&app, Some(&telegram), release_body(SEAL_ID)).await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(
        answer.headers.get(CACHE_CONTROL).map(HeaderValue::as_bytes),
        Some(b"no-store".as_slice()),
        "no cache keeps a released key"
    );
    assert_eq!(
        answer.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(b"application/json".as_slice())
    );

    let refused = examined(
        "requests that are cross-site or not JSON",
        vec![
            (
                "a cross-site request",
                vec![
                    ("content-type", "application/json"),
                    ("sec-fetch-site", "cross-site"),
                    ("cookie", telegram.as_str()),
                ],
                "cross_site_request",
            ),
            (
                "a same-site request from another origin",
                vec![
                    ("content-type", "application/json"),
                    ("sec-fetch-site", "same-site"),
                    ("cookie", telegram.as_str()),
                ],
                "cross_site_request",
            ),
            (
                "a form post",
                vec![
                    ("content-type", "application/x-www-form-urlencoded"),
                    ("sec-fetch-site", "same-origin"),
                    ("cookie", telegram.as_str()),
                ],
                "not_json",
            ),
            (
                "a request with no content type",
                vec![
                    ("sec-fetch-site", "same-origin"),
                    ("cookie", telegram.as_str()),
                ],
                "not_json",
            ),
        ],
    );
    for (what, headers, code) in refused {
        let answer = send(&app, "POST", SEAL_KEY_PATH, &headers, release_body(SEAL_ID)).await;
        assert_eq!(answer.status, StatusCode::FORBIDDEN, "{what} is refused");
        assert_eq!(reason(&answer), code, "{what} is refused by its code");
        assert!(key(&answer).is_empty(), "{what} is given no key");
    }
}

#[tokio::test]
async fn the_release_is_off_without_the_seal_secret() {
    let (_, sessions, app) = app(false);
    let telegram = telegram(&sessions);
    let off = examined(
        "requests to a release with no seal secret",
        vec![
            ("the owner's session", Some(telegram.as_str())),
            ("no session", None),
        ],
    );
    for (what, presented) in off {
        let answer = release(&app, presented, release_body(SEAL_ID)).await;
        assert_eq!(
            answer.status,
            StatusCode::NOT_FOUND,
            "{what} finds the route off"
        );
        assert_eq!(
            reason(&answer),
            "sync_seal_off",
            "{what} is told the route is off"
        );
        assert!(key(&answer).is_empty(), "{what} is given no key");
    }
}

/// One captured event or span: its level and its fields.
#[derive(Clone)]
struct Line {
    level: String,
    fields: Vec<(String, String)>,
}

impl Line {
    /// The line as text: its level and every field, so a search reads all of it.
    fn text(&self) -> String {
        let mut text = self.level.clone();
        for (name, value) in &self.fields {
            let _ = write!(text, " {name}={value}");
        }
        text
    }
}

/// A subscriber that keeps every event and span it is given.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<Line>>>);

impl Captured {
    /// Every line captured so far.
    fn lines(&self) -> Vec<Line> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn push(&self, line: Line) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(line);
    }
}

/// The fields of one event or span, as they are recorded.
#[derive(Default)]
struct Fields(Vec<(String, String)>);

impl Visit for Fields {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name().to_owned(), value.to_owned()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.push((field.name().to_owned(), format!("{value:?}")));
    }
}

impl Subscriber for Captured {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut fields = Fields::default();
        span.record(&mut fields);
        self.push(Line {
            level: "span".to_owned(),
            fields: fields.0,
        });
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, values: &Record<'_>) {
        let mut fields = Fields::default();
        values.record(&mut fields);
        self.push(Line {
            level: "span".to_owned(),
            fields: fields.0,
        });
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = Fields::default();
        event.record(&mut fields);
        self.push(Line {
            level: event.metadata().level().to_string(),
            fields: fields.0,
        });
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

#[tokio::test]
async fn no_seal_secret_id_or_key_reaches_a_record() {
    let captured = Captured::default();
    // The route's events reach the capture as they reach a log: through the kernel's silence.
    let _logging = log_capture::hold_capture(logging::silence(captured.clone()));
    let (_, sessions, app) = app(true);
    let telegram = telegram(&sessions);
    let other_id = "_-_-AwQFBgcICQoLDA0ODw";
    let released = release(&app, Some(&telegram), release_body(SEAL_ID)).await;
    let released_key = key(&released);
    let other = release(&app, Some(&telegram), release_body(other_id)).await;
    let other_key = key(&other);
    let bad_id = "AAECAwQFBgcICQoLDA0O";
    let refused = release(&app, Some(&telegram), release_body(bad_id)).await;
    let anonymous = release(&app, None, release_body(SEAL_ID)).await;
    let planted = ["planted", "seal", "marker"].join("-");
    tracing::warn!(marker = %planted, "a planted line the capture must catch");

    let lines = captured.lines();
    let texts: Vec<String> = lines.iter().map(Line::text).collect();
    assert!(
        texts.iter().any(|text| text.contains(&planted)),
        "the capture caught the planted line: the positive control"
    );
    assert!(
        texts
            .iter()
            .any(|text| text.contains("a sealing key was released")),
        "the capture caught the release's own line"
    );
    let secret = secret_text();
    let secret_hex = secret.bytes().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    let secrets = examined(
        "seal secrets, ids and keys",
        vec![
            ("the seal secret", secret),
            ("the seal secret in hex", secret_hex),
            ("the seal id", SEAL_ID.to_owned()),
            ("the other seal id", other_id.to_owned()),
            ("the refused seal id", bad_id.to_owned()),
            ("the released key", released_key),
            ("the other released key", other_key),
        ],
    );
    examined("captured lines", texts.clone());
    // The state's debug line is a record too: it names the seal port it holds and no byte of it.
    let state_line = format!(
        "{:?}",
        ApiState::new(Readiness::new()).with_seal(
            SealSecret::new(secret_text().as_bytes()).expect("the secret is long enough"),
        )
    );
    assert!(
        state_line.ends_with(", seal: true }"),
        "the state names the seal port it holds: {state_line}"
    );
    for (what, secret) in &secrets {
        assert!(!secret.is_empty(), "{what} is known to the test");
        for text in texts.iter().chain([&state_line]) {
            assert!(
                !text.contains(secret.as_str()),
                "{what} reaches a record: {text}"
            );
        }
    }
    // The requests took each path the route has: two releases, a refused id and no session.
    let statuses = [
        released.status,
        other.status,
        refused.status,
        anonymous.status,
    ];
    assert_eq!(
        statuses,
        [
            StatusCode::OK,
            StatusCode::OK,
            StatusCode::BAD_REQUEST,
            StatusCode::UNAUTHORIZED
        ],
        "the captured requests released twice, refused an id and refused no session"
    );
}
