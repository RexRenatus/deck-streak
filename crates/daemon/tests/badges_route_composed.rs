//! The badges route through the daemon's composed router (SPEC-073 R16): the `api` role's state
//! renders the locked catalog's descriptions from the courses its settings name, so the courses
//! the daemon reads are the ones the owner's gallery shows, and with no courses file the catalog
//! reads the defaults.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::ffi::OsString;
use std::fs;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{Request, StatusCode};
use deck_streak_api::{OwnerAccess, Readiness, router};
use deck_streak_daemon::role_api::api_state;
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::courses::COURSES_FILE;
use deck_streak_kernel::{
    Clock, Db, Environment, ManualClock, Offload, OffloadWorkers, StudyDayRule, TelegramUserId,
    UtcMillis,
};
use serde_json::{Value, json};
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
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 256 * 1024;
/// Two synthetic courses.
const COURSES: &str = r#"{
  "schema": "deckstreak.courses.v1",
  "courses": [
    {"code": "qaa", "name": "Course Qaa", "flag": "F", "deck_root": "Qaa", "alias": "a",
     "writing": false, "unit_bands": {"A1": [1, 5]}},
    {"code": "qab", "name": "Course Qab", "flag": "F", "deck_root": "Qab Deck", "alias": "b",
     "writing": true, "unit_bands": {}}
  ],
  "focus_subjects": []
}"#;

/// The router the `api` role serves under `vars`, over the migrated database `db`.
fn composed(vars: Vec<(&str, OsString)>, db: &Db) -> Router {
    let env = Environment::from_vars(vars);
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
    let readiness = Readiness::new();
    readiness.database_opened(db.clone());
    router(api_state(&env, &offload, readiness, access))
}

/// The owner's `GET /api/badges` through `app`: each locked badge's key and criteria.
async fn locked_criteria(app: &Router) -> Vec<(String, String)> {
    let opened = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/session")
                .header("content-type", "application/json")
                .header("sec-fetch-site", "same-origin")
                .body(Body::from(
                    json!({ "init_data": OWNER_PAYLOAD }).to_string(),
                ))
                .expect("a request"),
        )
        .await
        .expect("the router is infallible");
    let cookie = opened
        .headers()
        .get(SET_COOKIE)
        .expect("a Set-Cookie")
        .to_str()
        .expect("text")
        .split(';')
        .next()
        .expect("a name and value")
        .to_owned();
    let read = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/badges")
                .header("cookie", cookie)
                .body(Body::empty())
                .expect("a request"),
        )
        .await
        .expect("the router is infallible");
    assert_eq!(read.status(), StatusCode::OK);
    let bytes = to_bytes(read.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    let body: Value = serde_json::from_slice(&bytes).expect("a JSON body");
    body["locked"]
        .as_array()
        .expect("the locked badges")
        .iter()
        .map(|badge| {
            (
                badge["key"].as_str().expect("a key").to_owned(),
                badge["criteria"].as_str().expect("criteria").to_owned(),
            )
        })
        .collect()
}

/// The criteria `locked` gives the badge `key`.
fn criteria_of<'a>(locked: &'a [(String, String)], key: &str) -> Option<&'a str> {
    locked
        .iter()
        .find(|(locked_key, _)| locked_key == key)
        .map(|(_, criteria)| criteria.as_str())
}

#[tokio::test]
async fn the_composed_badges_route_renders_the_configured_courses() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let file = dir.path().join("courses.json");
    fs::write(&file, COURSES).expect("the courses file");

    let configured =
        locked_criteria(&composed(vec![(COURSES_FILE, file.into_os_string())], &db)).await;
    let defaults = locked_criteria(&composed(Vec::new(), &db)).await;
    println!(
        "examined {} and {} locked badge(s)",
        configured.len(),
        defaults.len()
    );
    assert_eq!(
        criteria_of(&configured, "polyglot_reader"),
        Some("Read in all 2 courses in one week")
    );
    assert_eq!(
        criteria_of(&defaults, "polyglot_reader"),
        Some("Read in every course in one week")
    );
    db.close().await;
}
