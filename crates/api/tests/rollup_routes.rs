//! The owner's analytics routes over the shell's layers (SPEC-071 A22, A23; R10, R20): a day whose
//! card state was never recorded, and a day with no answered review, render their card state and
//! their retention as null, never as 0; and the rollups and the score answer the owner's live
//! session alone, with 401 or 403 and no data otherwise.
//!
//! The launch payloads are SPEC-024's: signed by Python's standard `hmac` and `hashlib` for a
//! synthetic bot token that never has the Bot API token's shape, for user ids of fewer than seven
//! digits, all dated 2025-01-15T03:30:00Z. The clock is a `ManualClock` started ten seconds later,
//! so the payloads are fresh and the server's study day is still the 14th. Every rollup is a
//! synthetic row written into a temporary database.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_coordination::score::day_score;
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, TelegramUserId, UtcMillis};
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
/// A day whose card state no recompute recorded: 2025-01-12.
const NEVER_RECORDED: i64 = TODAY - 2;
/// A day with no answered review: 2025-01-13.
const NO_ANSWER: i64 = TODAY - 1;
/// The rollups of the three seeded days.
const DAYS_PATH: &str = "/api/analytics/days?from=2025-01-12&to=2025-01-14";
/// The current study day's score.
const SCORE_PATH: &str = "/api/score";
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

/// One synthetic rollup row: its day, its answered and passed counts and true retention, whether
/// a recompute recorded its card state, and its score and pillars.
struct Seed {
    day: i64,
    reviews: i64,
    answered: i64,
    passed: i64,
    true_retention: f64,
    card_state: Option<[i64; 5]>,
    score: i64,
    pillars: [f64; 5],
}

/// The three days: one whose card state was never recorded, one with no answered review, and the
/// current day, whole.
const SEEDS: [Seed; 3] = [
    Seed {
        day: NEVER_RECORDED,
        reviews: 12,
        answered: 10,
        passed: 9,
        true_retention: 90.0,
        card_state: None,
        score: 64,
        pillars: [76.0, 80.0, 0.0, 51.5, 0.0],
    },
    Seed {
        day: NO_ANSWER,
        reviews: 0,
        answered: 0,
        passed: 0,
        true_retention: 0.0,
        card_state: Some([118, 61, 2, 9, 27]),
        score: 12,
        pillars: [24.0, 0.0, 0.0, 0.0, 30.0],
    },
    Seed {
        day: TODAY,
        reviews: 40,
        answered: 24,
        passed: 21,
        true_retention: 87.5,
        card_state: Some([120, 60, 2, 5, 30]),
        score: 72,
        pillars: [76.0, 85.0, 70.0, 60.5, 55.0],
    },
];

/// Writes `seed` as its day's `daily_rollup` row.
async fn seed(db: &Db, seed: &Seed) {
    let [mature, young, leeches, backlog, due] =
        seed.card_state.map_or([None; 5], |counts| counts.map(Some));
    let source = seed.card_state.map(|_| "live:1736911800000");
    let [consistency, retention, workload, volume, mastery] = seed.pillars;
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO daily_rollup (study_day, reviews, learn_count, review_count, relearn_count, \
         filtered_count, seconds, answered, passed, true_retention, graduations, decks_studied, \
         avg_answer_seconds, young_answered, young_passed, mature_answered, mature_passed, \
         mature_count, young_count, leech_active, backlog, due_today, card_state_src, score, \
         consistency, retention, workload, volume, mastery, score_at_close, settled_at, \
         fingerprint, created_at, updated_at) \
         VALUES (?1, ?2, 0, ?2, 0, 0, ?3, ?4, ?5, ?6, 1, 1, 30.0, 0, 0, ?4, ?5, ?7, ?8, ?9, ?10, \
         ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, NULL, NULL, 'synthetic', 1000, 1000)",
    )
    .bind(seed.day)
    .bind(seed.reviews)
    .bind(30.0 * f64::from(u32::try_from(seed.reviews).expect("a small count")))
    .bind(seed.answered)
    .bind(seed.passed)
    .bind(seed.true_retention)
    .bind(mature)
    .bind(young)
    .bind(leeches)
    .bind(backlog)
    .bind(due)
    .bind(source)
    .bind(seed.score)
    .bind(consistency)
    .bind(retention)
    .bind(workload)
    .bind(volume)
    .bind(mastery)
    .execute(&mut *write)
    .await
    .expect("the synthetic rollup is written");
    write.commit().await.expect("the commit");
}

/// The API as the daemon builds it, over a migrated database holding the three seeded days, for
/// the synthetic owner and bot, on a manual clock.
async fn app(scratch: &TempDir) -> (Db, Router) {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    for day in &SEEDS {
        seed(&db, day).await;
    }
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

/// The owner's day `day` among the rollups `body` answers.
fn rollup_of(days: &[Value], day: &str) -> Value {
    days.iter()
        .find(|rollup| rollup["study_day"] == day)
        .unwrap_or_else(|| panic!("the rollup of {day} is answered"))
        .clone()
}

#[tokio::test]
async fn a_day_never_recorded_and_a_day_with_no_answer_render_as_null() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    let cookie = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);

    let answer = get(&app, DAYS_PATH, Some(&cookie)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer
            .headers
            .get(CONTENT_TYPE)
            .map(|value| value.as_bytes()),
        Some(&b"application/json"[..])
    );
    let body = answer.json();
    let days = examined(
        "rollup(s) answered",
        body["days"].as_array().expect("the days").clone(),
    );
    let answered: Vec<&str> = days
        .iter()
        .map(|rollup| rollup["study_day"].as_str().expect("an ISO date"))
        .collect();
    assert_eq!(
        answered,
        ["2025-01-12", "2025-01-13", "2025-01-14"],
        "every seeded day of the range, oldest first"
    );

    // A day whose card state no recompute recorded: its card state and its provenance are null,
    // never five zeros; its retention, over ten answers, is a number.
    let never = rollup_of(&days, "2025-01-12");
    assert_eq!(never["card_state"], Value::Null, "{never}");
    assert_eq!(never["card_state_src"], Value::Null, "{never}");
    assert_eq!(never["true_retention"], json!(90.0), "{never}");
    assert_eq!(
        never["score"]["pillars"]["retention"],
        json!(80.0),
        "{never}"
    );

    // A day with no answered review: its retention and its retention pillar are null, never 0,
    // though the row stores the golden's 0; its recorded card state is the five counts.
    let empty = rollup_of(&days, "2025-01-13");
    assert_eq!(empty["answered"], json!(0), "{empty}");
    assert_eq!(empty["true_retention"], Value::Null, "{empty}");
    assert_eq!(
        empty["score"]["pillars"]["retention"],
        Value::Null,
        "{empty}"
    );
    assert_eq!(empty["score"]["retention"], Value::Null, "{empty}");
    assert_eq!(
        empty["card_state"],
        json!({
            "mature_count": 118,
            "young_count": 61,
            "leech_active": 2,
            "backlog": 9,
            "due_today": 27
        }),
        "{empty}"
    );
    assert_eq!(empty["card_state_src"], json!("live:1736911800000"));

    // The current day, whole.
    let today = rollup_of(&days, "2025-01-14");
    assert_eq!(today["true_retention"], json!(87.5), "{today}");
    assert_eq!(today["card_state"]["due_today"], json!(30), "{today}");
    db.close().await;
}

#[tokio::test]
async fn the_analytics_routes_answer_only_the_owner() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    let paths = examined("analytics route(s)", vec![DAYS_PATH, SCORE_PATH]);

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

    // Another user, and the owner's fields forged for another bot, never get a session: 403 or
    // 401 at the handshake, with no cookie, and no data from either route after it.
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

    // The owner's live session reads both: the score route answers the numbers of the current
    // day's score, the one read the bot's /score shares.
    for path in &paths {
        let answer = get(&app, path, Some(&owner)).await;
        assert_eq!(answer.status, StatusCode::OK, "{path}: {}", answer.body);
    }
    let today = day_score(&db, StudyDay::from_epoch_day(TODAY))
        .await
        .expect("the score reads")
        .expect("the current day has a rollup");
    let answer = get(&app, SCORE_PATH, Some(&owner)).await.json();
    assert_eq!(
        answer,
        json!({
            "study_day": "2025-01-14",
            "score": {
                "total": today.total,
                "grade": {"label": today.grade_label, "emoji": today.grade_emoji},
                "pillars": {
                    "consistency": today.pillars.consistency,
                    "retention": today.pillars.retention,
                    "workload": today.pillars.workload,
                    "volume": today.pillars.volume,
                    "mastery": today.pillars.mastery
                },
                "reviews": today.reviews,
                "retention": today.retention
            }
        })
    );
    assert_eq!(
        (
            answer["score"]["total"].clone(),
            answer["score"]["grade"]["label"].clone()
        ),
        (json!(72), json!("SOLID")),
        "the seeded day's own numbers"
    );
    db.close().await;
}
