//! The owner's export over the shell's layers (SPEC-516 R2, R3; B2, B3): the export document to
//! the owner's session, never stored; the state-change guard before the session; the body bound
//! and the window of the route's own; and the database the route needs.
//!
//! Every clock is a `ManualClock` started at 2025-01-15T03:30:10Z, ten seconds into its minute, so
//! a refusal at the window's bound waits 50 seconds. Every expected answer is written out by hand,
//! the bounds among them, never read from the route's constants.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::FromRef;
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, RETRY_AFTER};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use deck_streak_api::data_export_routes::{
    EXPORT_BODY_LIMIT_BYTES, EXPORT_PATH, EXPORTS_PER_MINUTE,
};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::session::SESSION_COOKIE;
use deck_streak_identity::{Freshness, Owner, OwnerGate, Sessions, WebAppKey};
use deck_streak_kernel::{Db, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use serde_json::Value;
use tower::ServiceExt;

/// The synthetic bot token the gate is keyed with.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// The synthetic owner.
const OWNER: i64 = 4242;
/// 2025-01-15T03:30:10Z in milliseconds: ten seconds into its minute.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 1024 * 1024;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// A migrated database in a temporary directory, which lives as long as the directory.
async fn database() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (dir, db)
}

/// The API as the daemon builds it, for the synthetic owner, on a manual clock, over `db` when the
/// test opened one; and the access's own sessions.
fn app(db: Option<&Db>) -> (Arc<ManualClock>, Sessions, Router) {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock.clone(), StudyDayRule::default());
    let sessions = Sessions::from_ref(&access);
    let readiness = Readiness::new();
    if let Some(db) = db {
        readiness.database_opened(db.clone());
    }
    let state = ApiState::new(readiness).with_owner(access);
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
    body: Vec<u8>,
}

impl Answer {
    /// The body as text.
    fn text(&self) -> String {
        String::from_utf8(self.body.clone()).expect("a text body")
    }

    /// The body as JSON.
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("a JSON body")
    }

    /// One header's value, as bytes.
    fn header(&self, name: axum::http::HeaderName) -> Option<&[u8]> {
        self.headers.get(name).map(HeaderValue::as_bytes)
    }
}

/// The page's export: a same-origin JSON `POST` with `body`, with `cookie` when there is one.
async fn export(app: &Router, cookie: Option<&str>, body: &str) -> Answer {
    send(
        app,
        cookie,
        &[("sec-fetch-site", "same-origin")],
        "application/json",
        body,
    )
    .await
}

/// One `POST` to the export with `headers`, `content_type` and `body`, and `cookie` when given.
async fn send(
    app: &Router,
    cookie: Option<&str>,
    headers: &[(&str, &str)],
    content_type: &str,
    body: &str,
) -> Answer {
    let mut request = Request::builder()
        .method("POST")
        .uri(EXPORT_PATH)
        .header("content-type", content_type);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(body.to_owned()))
                .expect("a well-formed request"),
        )
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

#[tokio::test]
async fn the_export_is_the_owners_document_and_is_never_stored() {
    let (_dir, db) = database().await;
    let (_clock, sessions, app) = app(Some(&db));
    let cookie = telegram(&sessions);
    for body in examined("request body(ies)", vec!["{}", ""]) {
        let answer = export(&app, Some(&cookie), body).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.text());
        assert_eq!(answer.header(CONTENT_TYPE), Some(&b"application/json"[..]));
        assert_eq!(answer.header(CACHE_CONTROL), Some(&b"no-store"[..]));
        assert_eq!(
            answer.header(CONTENT_DISPOSITION),
            Some(&b"attachment; filename=\"deck-streak-export.json\""[..])
        );
        // One JSON object, as one line ended by its newline: the data role's own form.
        let text = answer.text();
        assert!(text.ends_with("}\n"), "one object and a newline: {text}");
        assert_eq!(text.lines().count(), 1, "{text}");
        let document = answer.json();
        assert_eq!(document["schema"], "deckstreak.export.v1");
        // A singleton's one row is in it, and an exempt table is not.
        assert_eq!(document["settings_generation"][0]["generation"], 0);
        assert!(document.get("cron_fires").is_none(), "{document}");
    }
}

/// A `link` session's cookie: the linking routes' session, which reads nothing of the owner's.
fn link(sessions: &Sessions) -> String {
    let session = sessions
        .open_link(Owner::new(TelegramUserId::new(OWNER)))
        .expect("a link session opens");
    format!("{SESSION_COOKIE}={}", session.expose())
}

#[tokio::test]
async fn the_route_refuses_without_an_owner_session() {
    let (_dir, db) = database().await;
    let (_clock, sessions, app) = app(Some(&db));
    let forged = format!("{SESSION_COOKIE}=not-a-session-the-service-opened");
    let linking = link(&sessions);
    let cookies = vec![None, Some(forged.as_str()), Some(linking.as_str())];
    for cookie in examined("missing, unknown or link session(s)", cookies) {
        let answer = export(&app, cookie, "{}").await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{}", answer.text());
        assert_eq!(answer.json()["reason"], "no_session");
        assert!(
            !answer.text().contains("deckstreak.export"),
            "a refusal carries no document: {}",
            answer.text()
        );
    }
}

#[tokio::test]
async fn a_cross_site_or_non_json_request_is_refused_before_the_session() {
    let (_dir, db) = database().await;
    let (_clock, sessions, app) = app(Some(&db));
    let cookie = telegram(&sessions);
    let refused = [
        (
            &[("sec-fetch-site", "cross-site")][..],
            "application/json",
            "cross_site_request",
        ),
        (
            &[("sec-fetch-site", "same-site")][..],
            "application/json",
            "cross_site_request",
        ),
        (
            &[("sec-fetch-site", "same-origin")][..],
            "text/plain",
            "not_json",
        ),
        (
            &[("sec-fetch-site", "same-origin")][..],
            "application/x-www-form-urlencoded",
            "not_json",
        ),
    ];
    for (headers, content_type, reason) in examined("refused request(s)", refused.to_vec()) {
        // The guard answers first, so a request with no session reads 403, never 401.
        for cookie in [None, Some(cookie.as_str())] {
            let answer = send(&app, cookie, headers, content_type, "{}").await;
            assert_eq!(answer.status, StatusCode::FORBIDDEN, "{}", answer.text());
            assert_eq!(answer.json()["reason"], reason);
        }
    }
}

#[tokio::test]
async fn the_route_states_its_body_bound() {
    assert_eq!(EXPORT_BODY_LIMIT_BYTES, 64, "the bound the SPEC states");
    let (_dir, db) = database().await;
    let (_clock, sessions, app) = app(Some(&db));
    let cookie = telegram(&sessions);
    // 62 blanks and `{}`: at the bound, read; one byte more, refused.
    let at = format!("{}{{}}", " ".repeat(62));
    let past = format!("{}{{}}", " ".repeat(63));
    assert_eq!((at.len(), past.len()), (64, 65));
    let answer = export(&app, Some(&cookie), &at).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.text());
    let answer = export(&app, Some(&cookie), &past).await;
    assert_eq!(
        answer.status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "{}",
        answer.text()
    );
}

#[tokio::test]
async fn the_route_states_its_rate_bound() {
    assert_eq!(EXPORTS_PER_MINUTE, 6, "the bound the SPEC states");
    let (_dir, db) = database().await;
    let (clock, sessions, app) = app(Some(&db));
    let cookie = telegram(&sessions);
    let admitted: Vec<StatusCode> = {
        let mut statuses = Vec::new();
        for _ in 0..6 {
            statuses.push(export(&app, Some(&cookie), "{}").await.status);
        }
        examined("export(s) inside the minute", statuses)
    };
    assert_eq!(admitted, vec![StatusCode::OK; 6]);
    let refused = export(&app, Some(&cookie), "{}").await;
    assert_eq!(
        refused.status,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        refused.text()
    );
    assert_eq!(refused.json()["reason"], "too_many_exports");
    assert_eq!(refused.header(RETRY_AFTER), Some(&b"50"[..]));
    // A refusal without a session is the session's, and spends nothing of the window.
    let unsigned = export(&app, None, "{}").await;
    assert_eq!(unsigned.status, StatusCode::UNAUTHORIZED);
    // When the minute turns, the window admits again.
    clock.advance(std::time::Duration::from_secs(50));
    let answer = export(&app, Some(&cookie), "{}").await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.text());
}

#[tokio::test]
async fn without_its_database_the_route_answers_unavailable() {
    let (_clock, sessions, app) = app(None);
    let cookie = telegram(&sessions);
    let answer = export(&app, Some(&cookie), "{}").await;
    assert_eq!(
        answer.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{}",
        answer.text()
    );
    assert_eq!(answer.json()["reason"], "database_not_open");
}
