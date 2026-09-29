//! The owner's level routes over the shell's layers (SPEC-072 A26; R23, R24): the level and the law
//! tiers answer the owner's live session alone, with 401 or 403 and no data otherwise, and the
//! owner reads the level, the title, today's XP marked provisional or settled, the run and the
//! one-miss preview.
//!
//! The launch payloads are SPEC-024's, the same synthetic ones the analytics routes' tests use,
//! dated 2025-01-15T03:30:00Z; the clock starts ten seconds later, so the study day is the 14th.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_coordination::progression::level_view::{LawTierSource, LawTiers};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{
    Db, KernelError, ManualClock, StudyDay, StudyDayRule, TelegramUserId, UtcMillis,
};
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
/// The level view.
const LEVEL_PATH: &str = "/api/level";
/// The law tiers view.
const LAW_TIERS_PATH: &str = "/api/level/law-tiers";
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

/// Writes today's settled XP: the language reviews still provisional, the law reviews settled, and
/// an Ascendant buff.
async fn seed(db: &Db) {
    let mut write = db.write().await.expect("a write");
    for (source, track, amount, closed) in [
        ("reviews", "language", 30, 0),
        ("reviews_law", "law", 12, 1),
    ] {
        sqlx::query(
            "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 1000)",
        )
        .bind(TODAY)
        .bind(source)
        .bind(track)
        .bind(amount)
        .bind(closed)
        .execute(&mut *write)
        .await
        .expect("the synthetic settlement is written");
    }
    sqlx::query("INSERT INTO buffs (study_day, kind, created_at) VALUES (?1, 'ascendant', 1000)")
        .bind(TODAY)
        .execute(&mut *write)
        .await
        .expect("the synthetic buff is written");
    write.commit().await.expect("the commit");
}

/// The API as the daemon builds it, over a migrated database holding the three seeded days, for
/// the synthetic owner and bot, on a manual clock.
async fn app(scratch: &TempDir) -> (Db, Router) {
    app_with(scratch, None).await
}

/// [`app`], with the law tiers' `source` when there is one.
async fn app_with(scratch: &TempDir, source: Option<Arc<dyn LawTierSource>>) -> (Db, Router) {
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
    let mut state = ApiState::new(readiness).with_owner(access);
    if let Some(source) = source {
        state = state.with_law_tiers(source);
    }
    (db, router(state))
}

/// A law-tier source that answers `tiers`, or a database refusal when it holds none.
#[derive(Debug)]
struct FixedTiers(Option<LawTiers>);

impl LawTierSource for FixedTiers {
    fn law_tiers<'a>(
        &'a self,
        _today: StudyDay,
    ) -> Pin<Box<dyn Future<Output = Result<LawTiers, KernelError>> + Send + 'a>> {
        Box::pin(async move {
            self.0
                .clone()
                .ok_or(KernelError::Database(sqlx::Error::PoolClosed))
        })
    }
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
async fn the_level_routes_answer_only_the_owner() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    let paths = examined("level route(s)", vec![LEVEL_PATH, LAW_TIERS_PATH]);

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

    // The owner's live session reads the level: the seeded day's rows, the provisional one marked.
    let answer = get(&app, LEVEL_PATH, Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    assert_eq!(
        answer.json(),
        json!({
            "study_day": "2025-01-14",
            "level": 1,
            "title": "Sprout",
            "emoji": "\u{1f423}",
            "total_xp": 42,
            "xp_into_level": 42,
            "xp_for_next": 100,
            "today": [
                {"source": "reviews", "track": "language", "amount": 30, "state": "provisional"},
                {"source": "reviews_law", "track": "law", "amount": 12, "state": "settled"}
            ],
            "run": 0,
            "multiplier": 1.0,
            "multiplier_after_a_miss": 1.0,
            "ascendant": true
        })
    );

    // The law tiers come from the collection, which this API was not given: the owner is told so.
    let tiers = get(&app, LAW_TIERS_PATH, Some(&owner)).await;
    assert_eq!(
        tiers.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "{}",
        tiers.body
    );
    assert_eq!(tiers.json(), json!({"reason": "law_tiers_unavailable"}));
    db.close().await;
}

#[tokio::test]
async fn the_law_tiers_answer_each_tier_and_a_failed_read_answers_500() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let tiers = LawTiers {
        cards: [1, 2, 3, 4, 5],
        xp: [10, 20, 30, 40, 50],
    };
    let (db, app) = app_with(&scratch, Some(Arc::new(FixedTiers(Some(tiers))))).await;
    let cookie = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let answer = get(&app, LAW_TIERS_PATH, Some(&cookie)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    assert_eq!(
        answer.json(),
        json!({
            "cards": {"T1": 1, "T2": 2, "T3": 3, "T4": 4, "none": 5},
            "xp_today": {"T1": 10, "T2": 20, "T3": 30, "T4": 40, "none": 50}
        })
    );
    db.close().await;

    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app_with(&scratch, Some(Arc::new(FixedTiers(None)))).await;
    let cookie = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let failed = get(&app, LAW_TIERS_PATH, Some(&cookie)).await;
    assert_eq!(failed.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        failed.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    assert_eq!(failed.json(), json!({"reason": "level_unreadable"}));

    // A level read over a database that has gone away fails the same way.
    db.close().await;
    let gone = get(&app, LEVEL_PATH, Some(&cookie)).await;
    assert_eq!(
        gone.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        gone.body
    );
    assert_eq!(gone.json(), json!({"reason": "level_unreadable"}));
}
