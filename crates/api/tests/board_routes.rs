//! The owner's board and exchange routes over the shell's layers (SPEC-075 R2, R7, R11; A2, A14,
//! A15, A17; #79, #80): both answer the owner's live session alone, with 401 and no data
//! otherwise; the board answers its rows in order; the exchange route refuses a window that is
//! not an integer and answers an undefined rate as JSON null.
//!
//! The launch payloads are SPEC-024's, the same synthetic ones the badge routes' tests use, dated
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

/// The personal board.
const BOARD_PATH: &str = "/api/board";
/// The XP exchange readout.
const EXCHANGE_PATH: &str = "/api/xp/exchange";
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

/// Runs `statement` with `binds`, committed.
async fn write(db: &Db, statement: &'static str, binds: &[Value]) {
    let mut write = db.write().await.expect("a write");
    let mut query = sqlx::query(statement);
    for bind in binds {
        query = match bind {
            Value::String(text) => query.bind(text.clone()),
            other => query.bind(other.as_i64().expect("an integer bind")),
        };
    }
    query
        .execute(&mut *write)
        .await
        .expect("the synthetic row is written");
    write.commit().await.expect("the commit");
}

/// Writes a rollup of `day` scoring `score`, with `graduations` graduated cards.
async fn rollup(db: &Db, day: i64, score: i64, graduations: i64) {
    write(
        db,
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
         mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
         consistency, retention, workload, volume, mastery, score_at_close, settled_at, \
         fingerprint, created_at, updated_at) \
         VALUES (?1, 42, 0, 42, 0, 0, 600.0, 40, 36, 90.0, ?2, 2, 14.0, 0, 0, 40, 36, 64, 60, 2, \
         5, 30, 'live:1736911800000', ?3, 76.0, 85.0, 70.0, 60.5, 55.0, NULL, NULL, \
         'synthetic', 1000, 1000)",
        &[json!(day), json!(graduations), json!(score)],
    )
    .await;
}

/// Writes `track`'s stored streak.
async fn streak(db: &Db, track: &str, current: i64, longest: i64) {
    write(
        db,
        "INSERT INTO streak_state (track, current_days, longest_days, freezes, last_study_day, \
         comeback_armed, created_at) VALUES (?1, ?2, ?3, 1, ?4, 0, 1000)",
        &[json!(track), json!(current), json!(longest), json!(TODAY)],
    )
    .await;
}

/// Writes one grant to `xp_ledger`.
async fn grant(db: &Db, day: i64, source: &str, amount: i64) {
    write(
        db,
        "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
         VALUES (?1, ?2, 'language', ?3, 'per-day', 1000)",
        &[json!(day), json!(source), json!(amount)],
    )
    .await;
}

/// Writes one settled row to `xp_settlement`.
async fn settle(db: &Db, day: i64, source: &str, amount: i64) {
    write(
        db,
        "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
         VALUES (?1, ?2, 'language', ?3, 1, 1000)",
        &[json!(day), json!(source), json!(amount)],
    )
    .await;
}

#[tokio::test]
async fn the_board_and_exchange_routes_answer_only_the_owner() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    let paths = examined(
        "board and exchange route(s)",
        vec![BOARD_PATH, EXCHANGE_PATH],
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
async fn the_board_route_answers_its_rows_in_order() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    rollup(&db, TODAY - 3, 90, 1).await;
    rollup(&db, TODAY, 77, 1).await;
    streak(&db, "language", 3, 9).await;
    streak(&db, "law", 20, 30).await;
    grant(&db, TODAY, "quest:1", 2_000).await;
    settle(&db, TODAY, "reviews", 2_000).await;
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);

    // The best day is 2025-01-11's 90, today the 14th's 77, the language streak 3 of 9, and both
    // tables' 4000 XP reach level 9.
    let answer = get(&app, BOARD_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let body = answer.json();
    println!(
        "examined {} board row(s)",
        body["rows"].as_array().map_or(0, Vec::len)
    );
    assert_eq!(
        body,
        json!({"rows": [
            {"kind": "best_day", "emoji": "\u{1f3c5}", "label": "Best day", "value": 90,
             "study_day": "2025-01-11"},
            {"kind": "today", "emoji": "\u{1f4c5}", "label": "Today", "value": 77,
             "study_day": "2025-01-14"},
            {"kind": "streak", "emoji": "\u{1f525}", "label": "Streak", "value": 3, "longest": 9},
            {"kind": "level", "emoji": "\u{26a1}", "label": "Level", "value": 9,
             "title": "Seedling"},
        ]})
    );
    db.close().await;
}

#[tokio::test]
async fn the_exchange_route_refuses_a_malformed_window() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let malformed = examined("malformed window(s)", vec!["days=x", "days=1.5", "days="]);
    for query in malformed {
        let refused = get(&app, &format!("{EXCHANGE_PATH}?{query}"), Some(&owner)).await;
        assert_eq!(refused.status, StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(refused.json(), json!({"reason": "invalid_days"}), "{query}");
    }
    // An integer window, and no window at all, are read.
    for query in ["days=3", "days=-2", "other=1"] {
        let answer = get(&app, &format!("{EXCHANGE_PATH}?{query}"), Some(&owner)).await;
        assert_eq!(answer.status, StatusCode::OK, "{query} {}", answer.body);
    }
    db.close().await;
}

#[tokio::test]
async fn the_exchange_route_answers_null_for_an_undefined_rate() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    rollup(&db, TODAY, 77, 0).await;
    rollup(&db, TODAY - 1, 60, 4).await;
    grant(&db, TODAY, "focus", 50).await;
    settle(&db, TODAY - 1, "reviews", 10).await;
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);

    // focus paid only on the 14th, when nothing graduated: no rate, and `rate_defined` false.
    let every = get(&app, EXCHANGE_PATH, Some(&owner)).await;
    assert_eq!(every.status, StatusCode::OK, "{}", every.body);
    let rates = json!([
        {"source": "focus", "total_xp": 50, "graduated_cards": 0, "rate": null,
         "rate_defined": false},
        {"source": "reviews", "total_xp": 10, "graduated_cards": 4, "rate": 2.5,
         "rate_defined": true},
    ]);
    assert_eq!(every.json(), json!({"window": null, "rates": rates}));

    // A two-day window names its first and last day.
    let windowed = get(&app, &format!("{EXCHANGE_PATH}?days=2"), Some(&owner)).await;
    assert_eq!(windowed.status, StatusCode::OK, "{}", windowed.body);
    assert_eq!(
        windowed.json(),
        json!({"window": {"first": "2025-01-13", "last": "2025-01-14"}, "rates": rates})
    );
    db.close().await;
}

/// The board and exchange routes answer 503 `database_not_open` while the database is not open,
/// and 500 with their own reason when a table they read cannot be read (SPEC-075 R11).
#[tokio::test]
async fn the_board_and_exchange_routes_name_why_they_cannot_answer() {
    let mut refusals = 0_u32;
    let routes = examined(
        "board and exchange route(s)",
        vec![
            (
                BOARD_PATH,
                "ALTER TABLE daily_rollup RENAME TO gone_daily_rollup",
                "board_unreadable",
            ),
            (
                EXCHANGE_PATH,
                "ALTER TABLE xp_settlement RENAME TO gone_xp_settlement",
                "exchange_unreadable",
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
        closed_db.close().await;

        let scratch = tempfile::tempdir().expect("a temporary directory");
        let (db, open) = app(&scratch).await;
        let owner = cookie_of(&handshake(&open, OWNER_PAYLOAD).await);
        write(&db, rename, &[]).await;
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
    println!("board and exchange refusals: {refusals}");
    assert_eq!(refusals, 4);
}
