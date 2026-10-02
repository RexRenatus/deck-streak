//! The owner's badge, record and milestone routes over the shell's layers (SPEC-073 A20; R16,
//! R17): the three routes answer the owner's live session alone, with 401 or 403 and no data
//! otherwise; the owner reads the earned badges most recently awarded first, the locked catalog
//! with each stored input against its threshold, each record's distance from today, and the
//! milestone's `pending`.
//!
//! The launch payloads are SPEC-024's, the same synthetic ones the level routes' tests use, dated
//! 2025-01-15T03:30:00Z; the clock starts ten seconds later, so the study day is the 14th.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

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
/// Another user's launch data, validly signed for [`BOT_TOKEN`].
const STRANGER_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A777%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=2c30fb14ffed3eb8ddf5d9836d760774b91a30db863140cb57e05ea512f308dd",
);
/// The owner's fields, signed for another bot's token: forged, for this bot.
const FORGED_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=eb0bcf2041fa3ec65b71d7b00947ed06e6898d26f8f9591aca35caf1e10fdc3f",
);
/// 2025-01-15T03:30:10Z, ten seconds after the payloads were signed, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The server's study day at [`STARTED_AT`]: 2025-01-14, as an epoch day.
const TODAY: i64 = 20_102;

/// The badges view.
const BADGES_PATH: &str = "/api/badges";
/// The records view.
const RECORDS_PATH: &str = "/api/records";
/// The next milestone.
const MILESTONE_PATH: &str = "/api/milestone";
/// The largest body a test reads.
const BODY_READ_LIMIT: usize = 256 * 1024;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Writes today's rollup: 42 study reviews over two decks in 600 seconds, 64 mature cards
/// recorded, and a score of 77.
async fn seed_today(db: &Db) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
         mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
         consistency, retention, workload, volume, mastery, score_at_close, settled_at, \
         fingerprint, created_at, updated_at) \
         VALUES (?1, 42, 0, 42, 0, 0, 600.0, 40, 36, 90.0, 1, 2, 14.0, 0, 0, 40, 36, 64, 60, 2, \
         5, 30, 'live:1736911800000', 77, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, \
         'synthetic', 1000, 1000)",
    )
    .bind(TODAY)
    .execute(&mut *write)
    .await
    .expect("the synthetic rollup is written");
    sqlx::query(
        "INSERT INTO streak_state (track, current_days, longest_days, freezes, last_study_day, \
         comeback_armed, created_at) VALUES ('language', 3, 9, 1, ?1, 0, 1000)",
    )
    .bind(TODAY)
    .execute(&mut *write)
    .await
    .expect("the synthetic streak is written");
    write.commit().await.expect("the commit");
}

/// Writes one earned badge, awarded at `at`.
async fn award(db: &Db, key: &str, name: &str, emoji: &str, day: i64, at: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO badges_earned (badge_key, tier, name, emoji, study_day, celebrated_at, \
         created_at) VALUES (?1, 0, ?2, ?3, ?4, ?5, ?5)",
    )
    .bind(key)
    .bind(name)
    .bind(emoji)
    .bind(day)
    .bind(at)
    .execute(&mut *write)
    .await
    .expect("the synthetic badge is written");
    write.commit().await.expect("the commit");
}

/// Writes one stored record.
async fn record(db: &Db, kind: &str, value: i64, day: i64, previous: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO records (kind, value, study_day, previous, celebrated_at, created_at) \
         VALUES (?1, ?2, ?3, ?4, 1000, 1000)",
    )
    .bind(kind)
    .bind(value)
    .bind(day)
    .bind(previous)
    .execute(&mut *write)
    .await
    .expect("the synthetic record is written");
    write.commit().await.expect("the commit");
}

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
async fn the_badge_record_and_milestone_routes_answer_only_the_owner() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    let paths = examined(
        "badge, record and milestone route(s)",
        vec![BADGES_PATH, RECORDS_PATH, MILESTONE_PATH],
    );

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
    for path in &paths {
        for cookie in [None, Some(unknown.as_str()), Some(ended.as_str())] {
            let refused = get(&app, path, cookie).await;
            assert_eq!(
                refused.status,
                StatusCode::UNAUTHORIZED,
                "{path} {cookie:?}"
            );
            assert_eq!(refused.json(), json!({"reason": "no_session"}), "{path}");
        }
    }

    // Another user, and the owner's fields forged for another bot, never get a session.
    for (payload, status) in [
        (STRANGER_PAYLOAD, StatusCode::FORBIDDEN),
        (FORGED_PAYLOAD, StatusCode::UNAUTHORIZED),
    ] {
        let refused = handshake(&app, payload).await;
        assert_eq!(refused.status, status, "{}", refused.body);
        assert!(
            refused.headers.get(SET_COOKIE).is_none(),
            "no session opens"
        );
    }

    // The owner's live session reads each route as JSON.
    for path in &paths {
        let answer = get(&app, path, Some(&owner)).await;
        assert_eq!(answer.status, StatusCode::OK, "{path} {}", answer.body);
        assert_eq!(
            answer.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
            Some(&b"application/json"[..]),
            "{path}"
        );
    }
    db.close().await;
}

#[tokio::test]
async fn the_milestone_route_answers_pending() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    seed_today(&db).await;
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);

    // Road to C2 supplies no mature-card sum, so the milestone is pending whatever is stored.
    let answer = get(&app, MILESTONE_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.json(), json!({"status": "pending"}));
    db.close().await;
}

/// The badge and record routes answer 503 `database_not_open` while the database is not open, and
/// 500 with their own reason when their table cannot be read, each as JSON, as the streak routes
/// do (SPEC-073 R16, R17).
#[tokio::test]
async fn the_badge_and_record_routes_name_why_they_cannot_answer() {
    let mut refusals = 0_u32;
    let routes = examined(
        "badge and record route(s)",
        vec![
            (
                BADGES_PATH,
                "ALTER TABLE badges_earned RENAME TO gone_badges_earned",
                "badges_unreadable",
            ),
            (
                RECORDS_PATH,
                "ALTER TABLE records RENAME TO gone_records",
                "records_unreadable",
            ),
        ],
    );
    for (path, rename, reason) in routes {
        let scratch = tempfile::tempdir().expect("a temporary directory");
        let (closed_db, closed) = app_with(&scratch, false).await;
        let owner = cookie_of(&handshake(&closed, OWNER_PAYLOAD).await);
        let refused = get(&closed, path, Some(&owner)).await;
        assert_eq!(refused.status, StatusCode::SERVICE_UNAVAILABLE, "{path}");
        assert_eq!(
            refused.json(),
            json!({"reason": "database_not_open"}),
            "{path}"
        );
        assert_eq!(
            refused.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
            Some(&b"application/json"[..]),
            "{path}"
        );
        closed_db.close().await;

        let scratch = tempfile::tempdir().expect("a temporary directory");
        let (db, open) = app(&scratch).await;
        let owner = cookie_of(&handshake(&open, OWNER_PAYLOAD).await);
        let mut write = db.write().await.expect("a write");
        sqlx::query(rename)
            .execute(&mut *write)
            .await
            .expect("the table is renamed away");
        write.commit().await.expect("the commit");
        let broken = get(&open, path, Some(&owner)).await;
        assert_eq!(broken.status, StatusCode::INTERNAL_SERVER_ERROR, "{path}");
        assert_eq!(broken.json(), json!({"reason": reason}), "{path}");
        assert_eq!(
            broken.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
            Some(&b"application/json"[..]),
            "{path}"
        );
        db.close().await;
        refusals += 2;
    }
    println!("badge and record refusals: {refusals}");
    assert_eq!(refusals, 4);
}

#[tokio::test]
async fn the_badges_route_answers_the_earned_newest_first_and_the_locked_with_progress() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    seed_today(&db).await;
    award(
        &db,
        "week_warrior",
        "Week Warrior",
        "\u{1f525}",
        TODAY - 2,
        2_000,
    )
    .await;
    award(
        &db,
        "centurion_day",
        "Centurion Day",
        "\u{1f4af}",
        TODAY - 1,
        3_000,
    )
    .await;
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);

    let answer = get(&app, BADGES_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let body = answer.json();
    assert_eq!(
        body["earned"],
        json!([
            {"key": "centurion_day", "tier": 0, "name": "Centurion Day", "emoji": "\u{1f4af}",
             "study_day": "2025-01-13"},
            {"key": "week_warrior", "tier": 0, "name": "Week Warrior", "emoji": "\u{1f525}",
             "study_day": "2025-01-12"}
        ])
    );
    let locked = examined(
        "locked badge(s)",
        body["locked"].as_array().expect("a locked list").clone(),
    );
    assert_eq!(locked.len(), 38);
    let of = |key: &str| {
        locked
            .iter()
            .find(|line| line["key"] == key)
            .unwrap_or_else(|| panic!("{key} is locked"))
            .clone()
    };
    assert_eq!(
        of("monthly_monk"),
        json!({"key": "monthly_monk", "name": "Monthly Monk", "emoji": "\u{1f9d8}",
               "criteria": "30-day streak", "family": "study",
               "progress": {"value": 3, "threshold": 30}})
    );
    for (key, value, threshold) in [
        ("legendary_day", 77, 100),
        ("maturity_milestone", 64, 100),
        ("forest_guardian", 64, 1000),
        ("polyglot", 2, 3),
    ] {
        assert_eq!(
            of(key)["progress"],
            json!({"value": value, "threshold": threshold}),
            "{key}"
        );
    }
    assert_eq!(of("first_steps")["progress"], Value::Null);
    assert_eq!(of("first_page")["family"], "habit");
    assert_eq!(of("first_page")["progress"], Value::Null);
    db.close().await;
}

#[tokio::test]
async fn the_records_route_answers_each_distance_from_today() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    seed_today(&db).await;
    record(&db, "best_score", 120, TODAY - 5, 100).await;
    record(&db, "most_reviews", 300, TODAY - 4, 250).await;
    record(&db, "most_minutes", 8, TODAY - 3, 5).await;
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);

    // Today holds 77 points, 42 reviews and ten minutes: the minutes record is already reached, so
    // its distance is none, and the closest record ahead is the best score.
    let answer = get(&app, RECORDS_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer.json(),
        json!({
            "records": [
                {"kind": "best_score", "label": "Best daily score", "value": 120,
                 "study_day": "2025-01-09", "previous": 100, "today": 77, "distance": 43},
                {"kind": "most_reviews", "label": "Most reviews in a day", "value": 300,
                 "study_day": "2025-01-10", "previous": 250, "today": 42, "distance": 258},
                {"kind": "most_minutes", "label": "Most minutes in a day", "value": 8,
                 "study_day": "2025-01-11", "previous": 5, "today": 10, "distance": 0}
            ],
            "chase": {"kind": "best_score", "label": "Best daily score", "gap": 43}
        })
    );
    db.close().await;
}
