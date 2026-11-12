//! The owner's Road to C2 route over the shell's layers (SPEC-077 A16; R15): it answers the owner's
//! live session alone, with 401 and the reason alone otherwise; the owner reads an empty list before
//! a recompute stores a course, then the stored progress of each configured course, and never a
//! stale row of a course no longer configured.
//!
//! The launch payload is SPEC-024's, the same synthetic one the badge routes' tests use, dated
//! 2025-01-15T03:30:00Z; the clock starts ten seconds later. The courses are synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Courses, Db, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

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
/// 2025-01-15T03:30:10Z, ten seconds after the payload was signed, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;

/// Road to C2's progress.
const PROGRESS_PATH: &str = "/api/progress";
/// The largest body a test reads.
const BODY_READ_LIMIT: usize = 256 * 1024;

/// Synthetic courses: `be` is configured; `zz` is not.
const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[
{"code":"be","name":"Beta","flag":"b","deck_root":"Beta Course","alias":"b","writing":false,
"unit_bands":{"A1":[1,4]}}]}"#;

/// Writes one course's stored progress as the progress step stores it.
async fn store(db: &Db, course: &str, name: &str, band: &str, unit: Option<i64>, bands: &Value) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO language_progress (course, name, flag, mastery_pct, current_band, \
         mature_cards, total_cards, current_unit, bands, updated_at, created_at) \
         VALUES (?1, ?2, 'f', 42.5, ?3, 14, 30, ?4, ?5, 1000, 1000)",
    )
    .bind(course)
    .bind(name)
    .bind(band)
    .bind(unit)
    .bind(bands.to_string())
    .execute(&mut *write)
    .await
    .expect("the synthetic progress is written");
    write.commit().await.expect("the commit");
}

/// The API as the daemon builds it, over a migrated and empty database, for the synthetic owner
/// and bot and the synthetic courses, on a manual clock.
async fn app(scratch: &TempDir) -> (Db, Router) {
    app_with(scratch, true).await
}

/// The same app; when `open` is false the readiness never learns of the database.
async fn app_with(scratch: &TempDir, open: bool) -> (Db, Router) {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    let readiness = Readiness::new();
    if open {
        readiness.database_opened(db.clone());
    }
    let courses = Courses::parse(COURSES).expect("the synthetic courses parse");
    let state = ApiState::new(readiness)
        .with_owner(access)
        .with_courses(courses);
    (db, router(state))
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

/// A same-origin JSON handshake with `init_data`.
async fn handshake(app: &Router, init_data: &str) -> Answer {
    send(
        app,
        "POST",
        "/api/session",
        &[
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
        ],
        json!({ "init_data": init_data }).to_string(),
    )
    .await
}

/// The cookie a browser sends back after `answer`: its `name=value`.
fn cookie_of(answer: &Answer) -> String {
    let value = answer
        .headers
        .get(SET_COOKIE)
        .expect("a Set-Cookie")
        .to_str()
        .expect("a text header");
    value
        .split(';')
        .next()
        .expect("a name and value")
        .trim()
        .to_owned()
}

/// `GET path` carrying `cookie`, or no cookie.
async fn get(app: &Router, path: &str, cookie: Option<&str>) -> Answer {
    let headers: Vec<(&str, &str)> = cookie
        .map(|cookie| ("cookie", cookie))
        .into_iter()
        .collect();
    send(app, "GET", path, &headers, String::new()).await
}

#[tokio::test]
async fn the_progress_route_answers_only_the_owner() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;

    // No session, a cookie that names none, and a session the owner logged out of: 401, and the
    // body names the reason alone.
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let ended = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let logout = send(
        &app,
        "DELETE",
        "/api/session",
        &[
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
            ("cookie", &ended),
        ],
        String::new(),
    )
    .await;
    assert_eq!(logout.status, StatusCode::NO_CONTENT);
    let unknown = format!("__Host-deckstreak_session={}", "0".repeat(64));
    for cookie in [None, Some(unknown.as_str()), Some(ended.as_str())] {
        let refused = get(&app, PROGRESS_PATH, cookie).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{cookie:?}");
        assert_eq!(
            refused.json(),
            json!({"reason": "no_session"}),
            "{cookie:?}"
        );
    }

    // Before a recompute stores a course, the owner reads an empty list.
    let before = get(&app, PROGRESS_PATH, Some(&owner)).await;
    assert_eq!(before.status, StatusCode::OK, "{}", before.body);
    assert_eq!(before.json(), json!({"courses": []}));

    // A configured course's stored progress, beside a stale row of a course no longer configured.
    let bands = json!([
        {"band": "A1", "total": 10, "mature": 9, "pct": 90.0, "achieved": true},
        {"band": "A2", "total": 20, "mature": 5, "pct": 25.0, "achieved": false}
    ]);
    store(&db, "be", "Beta", "A2", Some(7), &bands).await;
    store(&db, "zz", "Stale", "C1", None, &json!([])).await;
    let answer = get(&app, PROGRESS_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    assert_eq!(
        answer.json(),
        json!({"courses": [{
            "code": "be",
            "name": "Beta",
            "flag": "f",
            "mastery_pct": 42.5,
            "current_band": "A2",
            "current_unit": 7,
            "bands": bands
        }]}),
        "the configured course's stored progress, whole; the stale row is absent"
    );
    db.close().await;
}

/// The progress route answers 503 `database_not_open` while the database is not open, and 500
/// `progress_unreadable`, its reason code alone, when the stored progress cannot be read, each as
/// JSON, as the badge routes do. Mutation coverage beside A16, not a criterion.
#[tokio::test]
async fn the_progress_route_names_why_it_cannot_answer() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (closed_db, closed) = app_with(&scratch, false).await;
    let owner = cookie_of(&handshake(&closed, OWNER_PAYLOAD).await);
    let refused = get(&closed, PROGRESS_PATH, Some(&owner)).await;
    assert_eq!(refused.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(refused.json(), json!({"reason": "database_not_open"}));
    assert_eq!(
        refused.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    closed_db.close().await;

    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, open) = app(&scratch).await;
    let owner = cookie_of(&handshake(&open, OWNER_PAYLOAD).await);
    let mut write = db.write().await.expect("a write");
    sqlx::query("ALTER TABLE language_progress RENAME TO gone_language_progress")
        .execute(&mut *write)
        .await
        .expect("the table is renamed away");
    write.commit().await.expect("the commit");
    let broken = get(&open, PROGRESS_PATH, Some(&owner)).await;
    assert_eq!(broken.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(broken.json(), json!({"reason": "progress_unreadable"}));
    assert_eq!(
        broken.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    db.close().await;
}
