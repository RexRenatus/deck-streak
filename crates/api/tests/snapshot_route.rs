//! The snapshot answer over the shell's layers (SPEC-377 R12 to R14; B1 to B3): found and its age
//! from a listing's sealed pairs, unknown with no lister, the owner's session it serves, the window
//! it keeps a listing for, its own bound, and the records it leaves, which name no object.
//!
//! The launch payload was signed by Python's standard `hmac` and `hashlib` for a synthetic bot
//! token, as `session_routes.rs` uses it. Every clock is a `ManualClock` started at
//! 2025-01-15T03:30:10Z, so the stamp `20250115T033000Z` is ten seconds old. Every expected answer
//! is written out by hand.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

use std::fmt;
use std::fmt::Write as _;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::FromRef;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, RETRY_AFTER};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use deck_streak_api::snapshot_routes::{READS_PER_MINUTE, SNAPSHOT_PATH, SnapshotLister};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::session::SESSION_COOKIE;
use deck_streak_identity::{Freshness, Owner, OwnerGate, Sessions, WebAppKey};
use deck_streak_kernel::{ManualClock, StudyDayRule, TelegramUserId, UtcMillis, logging};
use serde_json::{Value, json};
use tower::ServiceExt;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

/// The synthetic bot token Python signed the payload for.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// The synthetic owner.
const OWNER: i64 = 4242;
/// 2025-01-15T03:30:10Z in milliseconds: ten seconds into its minute.
const STARTED_AT: i64 = 1_736_911_810_000;
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

/// A lister that answers what the test sets, and counts how often it was asked.
#[derive(Default)]
struct Listing {
    names: Mutex<Option<Vec<String>>>,
    asked: AtomicUsize,
}

impl Listing {
    /// A lister answering `names`.
    fn of(names: &[&str]) -> Arc<Self> {
        let listing = Arc::new(Self::default());
        listing.set(Some(names));
        listing
    }

    /// From now on, answer `names`, or nothing.
    fn set(&self, names: Option<&[&str]>) {
        *self.names.lock().unwrap_or_else(PoisonError::into_inner) =
            names.map(|names| names.iter().map(|name| (*name).to_owned()).collect());
    }

    /// How often the route asked for a listing.
    fn asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }
}

impl SnapshotLister for Listing {
    fn list(&self) -> Pin<Box<dyn Future<Output = Option<Vec<String>>> + Send + '_>> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        let names = self
            .names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        Box::pin(async move { names })
    }
}

/// The API as the daemon builds it, for the synthetic owner, on a manual clock, with `lister` when
/// one is wired; and the access's own sessions.
fn app(lister: Option<Arc<Listing>>) -> (Arc<ManualClock>, Sessions, Router) {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock.clone(), StudyDayRule::default());
    let sessions = Sessions::from_ref(&access);
    let mut state = ApiState::new(Readiness::new()).with_owner(access);
    if let Some(lister) = lister {
        state = state.with_snapshot(lister);
    }
    (clock, sessions, router(state))
}

/// A `telegram` session's cookie.
fn telegram(sessions: &Sessions) -> String {
    let session = sessions
        .open(Owner::new(TelegramUserId::new(OWNER)))
        .expect("a telegram session opens");
    format!("{SESSION_COOKIE}={}", session.expose())
}

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

    /// The answer's `Cache-Control`.
    fn cache(&self) -> Option<&[u8]> {
        self.headers.get(CACHE_CONTROL).map(HeaderValue::as_bytes)
    }
}

/// `GET /api/sync/snapshot` as the Worker sends it, with `cookie` when there is one.
async fn read(app: &Router, cookie: Option<&str>) -> Answer {
    let mut request = Request::builder().method("GET").uri(SNAPSHOT_PATH);
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
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    Answer {
        status,
        headers,
        body: String::from_utf8(bytes.to_vec()).expect("a UTF-8 body"),
    }
}

/// The owner's answer from an app whose lister answers `names`.
async fn answer_for(names: &[&str]) -> Value {
    let (_, sessions, app) = app(Some(Listing::of(names)));
    let telegram = telegram(&sessions);
    let answer = read(&app, Some(&telegram)).await;
    assert_eq!(
        answer.status,
        StatusCode::OK,
        "the listing {names:?} is answered"
    );
    answer.json()
}

#[tokio::test]
async fn a_listed_sealed_snapshot_answers_found_and_its_age() {
    let listing = Listing::of(&[
        "sync-20250115T033000Z.tar.age",
        "sync-20250115T033000Z.sha256.age",
    ]);
    let (_, sessions, app) = app(Some(listing.clone()));
    let telegram = telegram(&sessions);
    let answer = read(&app, Some(&telegram)).await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.json(), json!({ "found": true, "age_seconds": 10 }));
    assert_eq!(answer.cache(), Some(b"no-store".as_slice()));
    assert_eq!(
        answer.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(b"application/json".as_slice())
    );
    assert_eq!(listing.asked(), 1, "the answer came from one listing");
    // A name is read by its last path segment.
    assert_eq!(
        answer_for(&[
            "archive/sync-20250115T033000Z.tar.age",
            "archive/sync-20250115T033000Z.sha256.age",
        ])
        .await,
        json!({ "found": true, "age_seconds": 10 })
    );
    // A listing with no sealed pair is not found.
    assert_eq!(answer_for(&[]).await, json!({ "found": false }));
}

#[tokio::test]
async fn the_route_refuses_without_an_owner_session() {
    let listing = Listing::of(&[
        "sync-20250115T033000Z.tar.age",
        "sync-20250115T033000Z.sha256.age",
    ]);
    let (_, _, app) = app(Some(listing.clone()));
    let presented = examined(
        "requests with no owner session",
        vec![
            ("no cookie", None),
            (
                "an unknown session",
                Some(format!("{SESSION_COOKIE}=unknown")),
            ),
        ],
    );
    for (what, cookie) in presented {
        let answer = read(&app, cookie.as_deref()).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{what} is refused");
        assert_eq!(answer.json()["reason"], "no_session", "{what} is told why");
        assert_eq!(answer.cache(), Some(b"no-store".as_slice()), "{what}");
    }
    assert_eq!(listing.asked(), 0, "a refused request lists nothing");
}

#[tokio::test]
async fn no_answer_names_an_object() {
    let captured = Captured::default();
    // The route's events reach the capture as they reach a log: through the kernel's silence.
    let _logging = log_capture::hold_capture(logging::silence(captured.clone()));
    let prefix = ["planted", "archive", "prefix"].join("-");
    let names = [
        format!("{prefix}/sync-20250115T033000Z.tar.age"),
        format!("{prefix}/sync-20250115T033000Z.sha256.age"),
        format!("{prefix}/stray-object.bin"),
    ];
    let listed: Vec<&str> = names.iter().map(String::as_str).collect();
    let listing = Listing::of(&listed);
    let (clock, sessions, app) = app(Some(listing.clone()));
    let telegram = telegram(&sessions);
    let found = read(&app, Some(&telegram)).await;
    let refused = read(&app, None).await;
    clock.advance(Duration::from_mins(1));
    listing.set(Some(&[listed[2]]));
    let not_found = read(&app, Some(&telegram)).await;
    clock.advance(Duration::from_mins(1));
    listing.set(None);
    let unknown = read(&app, Some(&telegram)).await;
    let answers = [&found, &refused, &not_found, &unknown];
    assert_eq!(
        answers.map(|answer| answer.status),
        [
            StatusCode::OK,
            StatusCode::UNAUTHORIZED,
            StatusCode::OK,
            StatusCode::OK
        ]
    );
    assert_eq!(
        [&found, &not_found, &unknown].map(Answer::json),
        [
            json!({ "found": true, "age_seconds": 10 }),
            json!({ "found": false }),
            json!({ "found": null }),
        ],
        "each answer is the found word and an age alone"
    );

    let texts: Vec<String> = captured.lines().iter().map(Line::text).collect();
    // The positive controls: each answer left its own line, by its word.
    for word in ["found", "not_found", "unknown"] {
        assert!(
            texts.iter().any(|text| {
                text.contains("the snapshot was answered")
                    && text.contains(&format!("answer={word}"))
            }),
            "the {word} answer left its line: {texts:?}"
        );
    }
    let secrets = examined(
        "object names and the prefix",
        names.iter().cloned().chain([prefix.clone()]).collect(),
    );
    let records: Vec<&String> = examined(
        "records",
        texts
            .iter()
            .chain(answers.iter().map(|answer| &answer.body))
            .collect(),
    );
    for secret in &secrets {
        for record in &records {
            assert!(
                !record.contains(secret.as_str()),
                "{secret} reaches a record: {record}"
            );
        }
    }
    // The listing's own names reach no record either, by their last segment.
    for record in &records {
        assert!(
            !record.contains("stray-object"),
            "an object reaches a record: {record}"
        );
        assert!(
            !record.contains(".tar.age"),
            "an archive reaches a record: {record}"
        );
    }
}

#[tokio::test]
async fn a_stale_answer_is_listed_again() {
    let listing = Listing::of(&[
        "sync-20250115T033000Z.tar.age",
        "sync-20250115T033000Z.sha256.age",
    ]);
    let (clock, sessions, app) = app(Some(listing.clone()));
    let telegram = telegram(&sessions);
    let first = read(&app, Some(&telegram)).await;
    assert_eq!(first.json(), json!({ "found": true, "age_seconds": 10 }));
    // The archive empties, but the kept listing answers for its window, its age still counting.
    listing.set(Some(&[]));
    clock.advance(Duration::from_secs(1));
    let kept = read(&app, Some(&telegram)).await;
    assert_eq!(kept.json(), json!({ "found": true, "age_seconds": 11 }));
    clock.set(UtcMillis::from_epoch_millis(STARTED_AT + 59_999));
    let last = read(&app, Some(&telegram)).await;
    assert_eq!(last.json(), json!({ "found": true, "age_seconds": 69 }));
    assert_eq!(listing.asked(), 1, "inside its window the listing is kept");
    // Sixty seconds after it was listed, the archive is listed again.
    clock.set(UtcMillis::from_epoch_millis(STARTED_AT + 60_000));
    let again = read(&app, Some(&telegram)).await;
    assert_eq!(again.json(), json!({ "found": false }));
    assert_eq!(listing.asked(), 2, "a stale listing is listed again");
}

#[tokio::test]
async fn a_listing_the_lister_could_not_make_answers_unknown_and_is_not_kept() {
    let listing = Arc::new(Listing::default());
    let (_, sessions, app) = app(Some(listing.clone()));
    let telegram = telegram(&sessions);
    let unknown = read(&app, Some(&telegram)).await;
    assert_eq!(unknown.status, StatusCode::OK);
    assert_eq!(unknown.json(), json!({ "found": null }));
    assert_eq!(unknown.cache(), Some(b"no-store".as_slice()));
    listing.set(Some(&[
        "sync-20250115T033000Z.tar.age",
        "sync-20250115T033000Z.sha256.age",
    ]));
    let found = read(&app, Some(&telegram)).await;
    assert_eq!(found.json(), json!({ "found": true, "age_seconds": 10 }));
    assert_eq!(listing.asked(), 2, "an unknown listing is not kept");
}

#[tokio::test]
async fn an_absent_list_credential_answers_unknown() {
    let (_, sessions, app) = app(None);
    let telegram = telegram(&sessions);
    let presented = examined(
        "requests to a route with no lister",
        vec![
            ("the owner's session", Some(telegram)),
            ("no session", None),
        ],
    );
    for (what, cookie) in presented {
        let answer = read(&app, cookie.as_deref()).await;
        assert_eq!(
            answer.status,
            StatusCode::OK,
            "{what} is answered, never 404"
        );
        assert_eq!(
            answer.json(),
            json!({ "found": null }),
            "{what} is told unknown"
        );
        assert_eq!(answer.cache(), Some(b"no-store".as_slice()), "{what}");
    }
}

#[tokio::test]
async fn an_archive_without_its_manifest_is_not_found() {
    let cases = examined(
        "listings with an unpaired archive",
        vec![
            (
                vec!["sync-20250115T033000Z.tar.age"],
                json!({ "found": false }),
            ),
            (
                vec![
                    "sync-20250115T033000Z.tar.age",
                    "sync-20250115T032900Z.sha256.age",
                ],
                json!({ "found": false }),
            ),
            (
                vec![
                    "sync-20250115T033005Z.tar.age",
                    "sync-20250115T032950Z.tar.age",
                    "sync-20250115T032950Z.sha256.age",
                ],
                json!({ "found": true, "age_seconds": 20 }),
            ),
        ],
    );
    for (names, expected) in cases {
        assert_eq!(answer_for(&names).await, expected, "{names:?}");
    }
}

#[tokio::test]
async fn a_name_outside_the_archive_form_is_ignored() {
    let stamps = examined(
        "stamps outside the archive's form",
        vec![
            "20250115T033000Zextra",
            "20250115T0330000",
            "20250115X033000Z",
            "20250115T03+000Z",
            "20250115T243000Z",
            "20250115T036000Z",
            "20250115T033060Z",
            "20251315T033000Z",
            "20250132T033000Z",
            "2025011T033000Z",
        ],
    );
    for stamp in stamps {
        let archive = format!("sync-{stamp}.tar.age");
        let manifest = format!("sync-{stamp}.sha256.age");
        assert_eq!(
            answer_for(&[&archive, &manifest]).await,
            json!({ "found": false }),
            "{stamp} is not a stamp"
        );
    }
    let names = examined(
        "names outside the archive's form",
        vec![
            [
                "snap-20250115T033000Z.tar.age",
                "snap-20250115T033000Z.sha256.age",
            ],
            [
                "sync-20250115T033000Z.tar.gz",
                "sync-20250115T033000Z.sha256.gz",
            ],
            ["sync-20250115T033000Z.tar", "sync-20250115T033000Z.sha256"],
        ],
    );
    for pair in names {
        assert_eq!(
            answer_for(&pair).await,
            json!({ "found": false }),
            "{pair:?}"
        );
    }
}

#[tokio::test]
async fn the_age_is_the_newest_sealed_stamps() {
    // 03:29:59 is eleven seconds old and newer than the other day's pair.
    assert_eq!(
        answer_for(&[
            "sync-20250114T235959Z.tar.age",
            "sync-20250115T032959Z.sha256.age",
            "sync-20250114T235959Z.sha256.age",
            "sync-20250115T032959Z.tar.age",
        ])
        .await,
        json!({ "found": true, "age_seconds": 11 })
    );
    // A day and an hour, a minute and a second before the clock.
    assert_eq!(
        answer_for(&[
            "sync-20250114T022909Z.tar.age",
            "sync-20250114T022909Z.sha256.age"
        ])
        .await,
        json!({ "found": true, "age_seconds": 90_061 })
    );
    // A stamp past the clock is no age below zero.
    assert_eq!(
        answer_for(&[
            "sync-20250115T033100Z.tar.age",
            "sync-20250115T033100Z.sha256.age"
        ])
        .await,
        json!({ "found": true, "age_seconds": 0 })
    );
}

#[tokio::test]
async fn the_route_has_a_bound_of_its_own() {
    let listing = Listing::of(&[]);
    let (clock, sessions, app) = app(Some(listing));
    let telegram = telegram(&sessions);
    // The SPEC's 30, written as a number: a window read from the constant would move with it.
    let admitted = examined("reads inside one minute", (1..=30_u32).collect::<Vec<_>>());
    for index in admitted {
        let answer = read(&app, Some(&telegram)).await;
        assert_eq!(answer.status, StatusCode::OK, "read {index} is admitted");
    }
    // Half a second later, 49.5 seconds are left in the minute: whole seconds round up.
    clock.advance(Duration::from_millis(500));
    let refused = read(&app, Some(&telegram)).await;
    assert_eq!(
        refused.status,
        StatusCode::TOO_MANY_REQUESTS,
        "the 31st is refused"
    );
    assert_eq!(refused.json()["reason"], "too_many_snapshot_reads");
    assert_eq!(refused.cache(), Some(b"no-store".as_slice()));
    assert_eq!(READS_PER_MINUTE, 30, "the bound the route names");
    assert_eq!(
        refused.headers.get(RETRY_AFTER).map(HeaderValue::as_bytes),
        Some(b"50".as_slice())
    );
    // The last millisecond of the minute is still inside it.
    clock.advance(Duration::from_millis(49_499));
    let last = read(&app, Some(&telegram)).await;
    assert_eq!(
        last.status,
        StatusCode::TOO_MANY_REQUESTS,
        "the minute is not over"
    );
    assert_eq!(
        last.headers.get(RETRY_AFTER).map(HeaderValue::as_bytes),
        Some(b"1".as_slice())
    );
    // The next minute admits again.
    clock.advance(Duration::from_millis(1));
    let next = read(&app, Some(&telegram)).await;
    assert_eq!(next.status, StatusCode::OK, "the next minute admits a read");
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
