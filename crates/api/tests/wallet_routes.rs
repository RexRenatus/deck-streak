//! The owner's wallet route over the shell's layers (SPEC-082 A15; R15, R17; ADR-315): the wallet
//! answers the owner's live session alone, with 401 and a body naming the reason alone otherwise,
//! and the owner reads the balance, today's loss cap and what is left of it, and the coin movements
//! newest first, a page at a time.
//!
//! The launch payload is SPEC-024's, the same synthetic one the level routes' tests use, dated
//! 2025-01-15T03:30:00Z; the clock starts ten seconds later, so the study day is the 14th. Every
//! movement is synthetic and written past the ports.

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
/// The wallet view.
const WALLET_PATH: &str = "/api/wallet";
/// The most a test reads of a body.
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

/// Writes four movements, in this order: a payout two days ago, yesterday's mint, today's mint and
/// a fine today. The balance is 103; today starts with 100, so its loss cap is 30, and the fine
/// leaves 25 of it.
async fn seed(db: &Db) {
    let mut write = db.write().await.expect("a write");
    for (day, source, reference, delta) in [
        (TODAY - 2, "payout", "p1", 60),
        (TODAY - 1, "mint", "", 40),
        (TODAY, "mint", "", 8),
        (TODAY, "fine", "f1", -5),
    ] {
        sqlx::query(
            "INSERT INTO coin_ledger (study_day, source, reference, delta, created_at) \
             VALUES (?1, ?2, ?3, ?4, 1000)",
        )
        .bind(day)
        .bind(source)
        .bind(reference)
        .bind(delta)
        .execute(&mut *write)
        .await
        .expect("the synthetic movement is written");
    }
    write.commit().await.expect("the commit");
}

/// The API as the daemon builds it, over a migrated database holding the seeded movements, for the
/// synthetic owner and bot, on a manual clock.
async fn app(scratch: &TempDir) -> (Db, Router) {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    seed(&db).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    let readiness = Readiness::new();
    readiness.database_opened(db.clone());
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
async fn the_wallet_and_shop_routes_answer_only_the_owner() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    // The routes that answer the owner alone. E3 adds its shop routes to this population.
    let paths = examined(
        "wallet route(s)",
        vec![WALLET_PATH.to_owned(), format!("{WALLET_PATH}?before=3")],
    );

    // No session, a cookie that names none, and a session the owner logged out of: 401, and the
    // body names the reason alone, so nothing of the wallet is learnt.
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
    let mut refusals = Vec::new();
    for path in &paths {
        for cookie in [None, Some(unknown.as_str()), Some(ended.as_str())] {
            let refused = get(&app, path, cookie).await;
            assert_eq!(
                refused.status,
                StatusCode::UNAUTHORIZED,
                "{path} {cookie:?}"
            );
            assert_eq!(refused.json(), json!({"reason": "no_session"}), "{path}");
            refusals.push(path);
        }
    }
    assert_eq!(examined("refusal(s)", refusals).len(), 6);

    // The owner's live session reads the wallet: the balance, today's loss cap and what is left of
    // it, and the movements newest first, by study day and then the reverse of the order written.
    let answer = get(&app, WALLET_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    assert_eq!(
        answer.json(),
        json!({
            "study_day": "2025-01-14",
            "balance": 103,
            "loss_cap": 30,
            "loss_cap_left": 25,
            "movements": [
                {"id": 4, "study_day": "2025-01-14", "source": "fine", "amount": -5},
                {"id": 3, "study_day": "2025-01-14", "source": "mint", "amount": 8},
                {"id": 2, "study_day": "2025-01-13", "source": "mint", "amount": 40},
                {"id": 1, "study_day": "2025-01-12", "source": "payout", "amount": 60}
            ],
            "next": null
        })
    );

    // The page after a movement starts after it.
    let older = get(&app, &format!("{WALLET_PATH}?before=3"), Some(&owner)).await;
    assert_eq!(older.status, StatusCode::OK, "{}", older.body);
    assert_eq!(
        older.json()["movements"],
        json!([
            {"id": 2, "study_day": "2025-01-13", "source": "mint", "amount": 40},
            {"id": 1, "study_day": "2025-01-12", "source": "payout", "amount": 60}
        ])
    );

    // A cursor that is not a movement's id is refused, and the body names the reason alone.
    let malformed = get(
        &app,
        &format!("{WALLET_PATH}?before=yesterday"),
        Some(&owner),
    )
    .await;
    assert_eq!(
        malformed.status,
        StatusCode::BAD_REQUEST,
        "{}",
        malformed.body
    );
    assert_eq!(malformed.json(), json!({"reason": "invalid_cursor"}));
    db.close().await;
}

#[tokio::test]
async fn a_wallet_that_cannot_be_read_answers_500_with_a_reason_code_alone() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let answer = get(&app, WALLET_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

    // The database goes away under the route: the read fails, and the body names the reason alone,
    // so neither the error nor anything of the wallet reaches the screen.
    db.close().await;
    let failed = get(&app, WALLET_PATH, Some(&owner)).await;
    assert_eq!(
        failed.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        failed.body
    );
    assert_eq!(
        failed.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    assert_eq!(failed.json(), json!({"reason": "wallet_unreadable"}));
}
