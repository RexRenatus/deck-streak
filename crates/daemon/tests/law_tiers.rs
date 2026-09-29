//! The law tiers through the daemon's composed router (SPEC-072 A31; R24): the `api` role's state,
//! built over a seeded collection copy, answers the owner with the law-track cards counted by tier
//! and today's law review XP split by each review's card tier, and answers no one else.
//!
//! The collection is the engine's own empty copy with notes, cards and reviews inserted by hand,
//! all synthetic, in the engine's default deck, which the law root setting names.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, Request, StatusCode};
use deck_streak_api::{OwnerAccess, Readiness, router};
use deck_streak_daemon::role_api::api_state;
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_ingest::engine::{AnkiEngine, RslibEngine};
use deck_streak_ingest::reader::Review;
use deck_streak_ingest::settings::{LAW_DECK_ROOT, STATE_DIRECTORY, SYNC_ENDPOINT, SyncSettings};
use deck_streak_ingest::tier::Tier;
use deck_streak_kernel::{
    Clock, Environment, ManualClock, Offload, OffloadWorkers, StudyDayRule, TelegramUserId,
    UtcMillis,
};
use deck_streak_progression::review_xp::review_xp;
use serde_json::{Value, json};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, Connection};
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
/// 2025-01-15T03:30:10Z, ten seconds after the payloads were signed: the study day is the 14th.
const STARTED_AT: i64 = 1_736_911_810_000;
/// 2025-01-14T12:00:00Z, noon of the study day the clock reads.
const NOON_TODAY: i64 = 1_736_856_000_000;
/// One day, in milliseconds.
const DAY_MS: i64 = 86_400_000;
/// The law tiers view.
const LAW_TIERS_PATH: &str = "/api/level/law-tiers";
/// The engine's default deck, which the law root names here.
const LAW_ROOT: &str = "Default";
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 256 * 1024;

/// The cards of the seeded collection: (id, the note's tags, the tier the reader must find).
const CARDS: [(i64, &str, Option<Tier>); 7] = [
    (1001, "T1", Some(Tier::T1)),
    (1002, "t1 vocab", Some(Tier::T1)),
    (1003, "T2", Some(Tier::T2)),
    (1004, "T3", Some(Tier::T3)),
    (1005, "T4", Some(Tier::T4)),
    (1006, "vocab", None),
    (1007, "", None),
];

/// The reviews of the seeded collection: (offset from noon today in days, the card).
const REVIEWS: [(i64, i64); 9] = [
    (0, 1001),
    (0, 1002),
    (0, 1003),
    (0, 1003),
    (0, 1004),
    (0, 1005),
    (0, 1006),
    // A review of the day before, which today's XP leaves out.
    (-1, 1001),
    (-1, 1005),
];

/// The review a seeded row stands for, as the reader hands it back.
fn review_of(place: usize, days: i64, card: i64) -> Review {
    Review {
        id: NOON_TODAY + days * DAY_MS + i64::try_from(place).expect("a small index"),
        card_id: card,
        ease: 3,
        interval: 10,
        last_interval: 5,
        factor: 2500,
        taken_ms: 4000,
        kind: 1,
    }
}

/// Inserts the synthetic notes, cards and reviews into the copy at `path`.
async fn seed(path: &Path) {
    let mut connection = SqliteConnectOptions::from_str("sqlite://")
        .expect("options")
        .filename(path)
        .connect()
        .await
        .expect("the copy opens for writing");
    for (id, tags, _) in CARDS {
        sqlx::query(
            "INSERT INTO notes (id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data) \
             VALUES (?1, ?2, 1, 0, 0, ?3, 'front\u{1f}back', 'front', 0, 0, '')",
        )
        .bind(id)
        .bind(format!("guid{id}"))
        .bind(format!(" {tags} "))
        .execute(&mut connection)
        .await
        .expect("a note");
        sqlx::query(
            "INSERT INTO cards (id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, \
             reps, lapses, left, odue, odid, flags, data) \
             VALUES (?1, ?1, 1, 0, 0, 0, 2, 2, 100, 10, 2500, 1, 0, 0, 0, 0, 0, '{}')",
        )
        .bind(id)
        .execute(&mut connection)
        .await
        .expect("a card");
    }
    for (place, (days, card)) in REVIEWS.into_iter().enumerate() {
        let review = review_of(place, days, card);
        sqlx::query(
            "INSERT INTO revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
             VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(review.id)
        .bind(review.card_id)
        .bind(review.ease)
        .bind(review.interval)
        .bind(review.last_interval)
        .bind(review.factor)
        .bind(review.taken_ms)
        .bind(review.kind)
        .execute(&mut connection)
        .await
        .expect("a review");
    }
    connection.close().await.expect("the copy closes");
}

/// The tier of the card `card` of the seeded collection.
fn tier_of(card: i64) -> Option<Tier> {
    CARDS
        .iter()
        .find(|(id, _, _)| *id == card)
        .and_then(|(_, _, tier)| *tier)
}

/// The place of `tier` in the view: `T1` to `T4`, then none.
const fn slot(tier: Option<Tier>) -> usize {
    match tier {
        Some(tier) => tier as usize,
        None => 4,
    }
}

/// The router the `api` role serves, over a seeded copy, for the synthetic owner and bot.
async fn composed(scratch: &TempDir, law_root: Option<&str>) -> Router {
    let state = scratch.path().join("state");
    std::fs::create_dir_all(&state).expect("a state directory");
    let mut variables = vec![
        (SYNC_ENDPOINT, OsStr::new("http://127.0.0.1:9/")),
        (STATE_DIRECTORY, state.as_os_str()),
    ];
    if let Some(root) = &law_root {
        variables.push((LAW_DECK_ROOT, OsStr::new(root)));
    }
    let env = Environment::from_vars(variables);
    let settings = SyncSettings::from_env(&env).expect("the settings");
    RslibEngine
        .new_card_queue(&settings.copy_path())
        .expect("the engine creates the copy");
    seed(&settings.copy_path()).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let offload = Offload::new(
        OffloadWorkers::new(1).expect("one worker"),
        clock.clone() as Arc<dyn Clock>,
    );
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    router(api_state(&env, &offload, Readiness::new(), access))
}

/// An answer: its status, its headers and its body.
struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

/// `method` on `path` with `headers` and `body`, through the whole router.
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
        .oneshot(request.body(Body::from(body)).expect("a request"))
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

#[tokio::test]
async fn the_composed_router_serves_the_law_tiers_to_the_owner_alone() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let app = composed(&scratch, Some(LAW_ROOT)).await;

    let mut cards = [0_u64; 5];
    for (_, _, tier) in CARDS {
        cards[slot(tier)] += 1;
    }
    let mut xp = [0_u64; 5];
    for (place, (days, card)) in REVIEWS.into_iter().enumerate() {
        if days == 0 {
            let tier = tier_of(card);
            xp[slot(tier)] += u64::from(review_xp(&review_of(place, days, card), tier));
        }
    }
    println!("examined {} seeded card(s)", CARDS.len());
    assert!(
        xp.iter().all(|amount| *amount > 0),
        "every tier earned: {xp:?}"
    );

    // No session: 401, and no data.
    let anonymous = send(&app, "GET", LAW_TIERS_PATH, &[], String::new()).await;
    assert_eq!(
        anonymous.status,
        StatusCode::UNAUTHORIZED,
        "{}",
        anonymous.body
    );
    // Another user never gets a session, so never gets the view.
    let stranger = handshake(&app, STRANGER_PAYLOAD).await;
    assert_eq!(stranger.status, StatusCode::FORBIDDEN, "{}", stranger.body);
    assert!(
        stranger.headers.get(SET_COOKIE).is_none(),
        "no session opens"
    );

    // The owner reads the seeded counts and today's XP by tier.
    let opened = handshake(&app, OWNER_PAYLOAD).await;
    let cookie = opened
        .headers
        .get(SET_COOKIE)
        .expect("a Set-Cookie")
        .to_str()
        .expect("text")
        .split(';')
        .next()
        .expect("a name and value")
        .to_owned();
    let answer = send(
        &app,
        "GET",
        LAW_TIERS_PATH,
        &[("cookie", &cookie)],
        String::new(),
    )
    .await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let body: Value = serde_json::from_str(&answer.body).expect("a JSON body");
    let named = |amounts: [u64; 5]| {
        json!({
            "T1": amounts[0], "T2": amounts[1], "T3": amounts[2], "T4": amounts[3],
            "none": amounts[4],
        })
    };
    assert_eq!(
        body,
        json!({"cards": named(cards), "xp_today": named(xp)}),
        "the seeded counts and the seeded XP"
    );
}

#[tokio::test]
async fn a_collection_with_no_law_root_has_no_law_cards_and_no_law_xp() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let app = composed(&scratch, None).await;
    let opened = handshake(&app, OWNER_PAYLOAD).await;
    let cookie = opened
        .headers
        .get(SET_COOKIE)
        .expect("a Set-Cookie")
        .to_str()
        .expect("text")
        .split(';')
        .next()
        .expect("a name and value")
        .to_owned();
    let answer = send(
        &app,
        "GET",
        LAW_TIERS_PATH,
        &[("cookie", &cookie)],
        String::new(),
    )
    .await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let body: Value = serde_json::from_str(&answer.body).expect("a JSON body");
    let zeros = json!({"T1": 0, "T2": 0, "T3": 0, "T4": 0, "none": 0});
    assert_eq!(
        body,
        json!({"cards": zeros, "xp_today": zeros}),
        "every seeded card is a language card, so none is a law card and none earned law XP"
    );
}
