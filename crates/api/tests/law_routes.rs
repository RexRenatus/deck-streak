//! The owner's law block route over the shell's layers (SPEC-077 A17; R14, R16): it answers the
//! owner's live session alone, with 401 and the reason alone otherwise; over an empty database the
//! owner reads the block omitted, every count zero or pending, and each pending count `null` beside
//! its flag, never 0.
//!
//! The launch payload is SPEC-024's, the same synthetic one the badge routes' tests use, dated
//! 2025-01-15T03:30:00Z; the clock starts ten seconds later.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Db, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
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

/// The server's study day at [`STARTED_AT`]: 2025-01-14, as an epoch day.
const TODAY: i64 = 20_102;

/// The law block.
const LAW_PATH: &str = "/api/law";
/// The largest body a test reads.
const BODY_READ_LIMIT: usize = 256 * 1024;

/// The API as the daemon builds it, over a migrated and empty database, for the synthetic owner
/// and bot, on a manual clock.
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
    (db, router(ApiState::new(readiness).with_owner(access)))
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
async fn the_law_route_answers_only_the_owner() {
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
        let refused = get(&app, LAW_PATH, cookie).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{cookie:?}");
        assert_eq!(
            refused.json(),
            json!({"reason": "no_session"}),
            "{cookie:?}"
        );
    }

    // Over an empty database the block is omitted, and the dues, the leeches and the mastery
    // pillar are pending: null beside their flags, never 0.
    let answer = get(&app, LAW_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    assert_eq!(
        answer.json(),
        json!({
            "shown": false,
            "level_shown": false,
            "lines": [],
            "streak": 0,
            "xp_today": 0,
            "total_xp": 0,
            "level": 1,
            "dues": null,
            "dues_pending": true,
            "leeches": null,
            "leeches_pending": true,
            "mastery": null,
            "mastery_pending": true
        }),
        "the law block over an empty database, whole"
    );
    db.close().await;
}

/// Seeds a law streak of 4 days, 30 law XP today and 70 the day before in the grants' table, and
/// law dues of 2 overdue and 3 due today, as the recompute would store them.
async fn seed_law(db: &Db) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO streak_state (track, current_days, longest_days, freezes, last_study_day, \
         comeback_armed, created_at) VALUES ('law', 4, 9, 0, ?1, 0, 1000)",
    )
    .bind(TODAY)
    .execute(&mut *write)
    .await
    .expect("the synthetic law streak is written");
    for (study_day, source, amount) in [
        (TODAY, "law_route_today", 30),
        (TODAY - 1, "law_route_before", 70),
    ] {
        sqlx::query(
            "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
             VALUES (?1, ?2, 'law', ?3, 'per-day', 1000)",
        )
        .bind(study_day)
        .bind(source)
        .bind(amount)
        .execute(&mut *write)
        .await
        .expect("the synthetic law XP is written");
    }
    sqlx::query(
        "INSERT INTO law_dues (id, study_day, backlog, due_today, updated_at, created_at) \
         VALUES (1, ?1, 2, 3, 1000, 1000)",
    )
    .bind(TODAY)
    .execute(&mut *write)
    .await
    .expect("the synthetic law dues are written");
    write.commit().await.expect("the commit");
}

/// A stored law streak, XP and dues show the block: each line's key in order, the level named
/// beside the lifetime XP, the dues counted and not pending, and the leeches and the mastery pillar
/// still pending (#133). Mutation coverage beside A17, not a criterion: it observes the line keys.
#[tokio::test]
async fn the_law_route_answers_the_stored_block_with_its_line_keys() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    seed_law(&db).await;
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let answer = get(&app, LAW_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer.json(),
        json!({
            "shown": true,
            "level_shown": true,
            "lines": ["total_xp", "streak", "xp_today", "dues"],
            "streak": 4,
            "xp_today": 30,
            "total_xp": 100,
            "level": 2,
            "dues": 5,
            "dues_pending": false,
            "leeches": null,
            "leeches_pending": true,
            "mastery": null,
            "mastery_pending": true
        }),
        "the stored law block, whole"
    );
    db.close().await;
}

/// The law route answers 503 `database_not_open` while the database is not open, and 500
/// `law_unreadable`, its reason code alone, when the block cannot be read, each as JSON, as the
/// badge routes do. Mutation coverage beside A17, not a criterion.
#[tokio::test]
async fn the_law_route_names_why_it_cannot_answer() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (closed_db, closed) = app_with(&scratch, false).await;
    let owner = cookie_of(&handshake(&closed, OWNER_PAYLOAD).await);
    let refused = get(&closed, LAW_PATH, Some(&owner)).await;
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
    sqlx::query("ALTER TABLE law_dues RENAME TO gone_law_dues")
        .execute(&mut *write)
        .await
        .expect("the table is renamed away");
    write.commit().await.expect("the commit");
    let broken = get(&open, LAW_PATH, Some(&owner)).await;
    assert_eq!(broken.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(broken.json(), json!({"reason": "law_unreadable"}));
    assert_eq!(
        broken.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    db.close().await;
}
