//! The sync seal release through the daemon's composed state (SPEC-363 R5, B14): the `api` role
//! composes the seal secret its credentials hold, so with none the release route is off, and with
//! one the owner's session is released the key identity makes for its seal id.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::ffi::OsString;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{Request, StatusCode};
use deck_streak_api::sync_seal_routes::SEAL_KEY_PATH;
use deck_streak_api::{OwnerAccess, Readiness, router};
use deck_streak_daemon::role_api::{api_state, with_seal_secret};
use deck_streak_identity::sync_seal::{SealId, SealSecret};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
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
const BODY_READ_LIMIT: usize = 64 * 1024;
/// Sixteen bytes 0 to 15, as a seal id.
const SEAL_ID: &str = "AAECAwQFBgcICQoLDA0ODw";

/// The seal secret's text, made from its parts at run time.
fn secret_text() -> String {
    ["synthetic", "composed", "seal", "secret", "for", "b14"].join("-")
}

/// The seal secret the role's credentials would hold.
fn secret() -> SealSecret {
    SealSecret::new(secret_text().as_bytes()).expect("the secret is long enough")
}

/// The router the `api` role serves over `db`, its state composed with `secret` as the role does.
fn composed(db: &Db, secret: Option<SealSecret>) -> Router {
    let env = Environment::from_vars(Vec::<(&str, OsString)>::new());
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
    router(with_seal_secret(
        api_state(&env, &offload, readiness, access),
        secret,
    ))
}

/// The owner's session cookie, opened through `app`.
async fn owner_cookie(app: &Router) -> String {
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
    opened
        .headers()
        .get(SET_COOKIE)
        .expect("a Set-Cookie")
        .to_str()
        .expect("text")
        .split(';')
        .next()
        .expect("a name and value")
        .to_owned()
}

/// The owner's release of [`SEAL_ID`] through `app`: its status and its JSON body.
async fn release(app: &Router) -> (StatusCode, Value) {
    let cookie = owner_cookie(app).await;
    let answer = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(SEAL_KEY_PATH)
                .header("content-type", "application/json")
                .header("sec-fetch-site", "same-origin")
                .header("cookie", cookie)
                .body(Body::from(json!({ "seal_id": SEAL_ID }).to_string()))
                .expect("a request"),
        )
        .await
        .expect("the router is infallible");
    let status = answer.status();
    let body = to_bytes(answer.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a body");
    (status, serde_json::from_slice(&body).expect("a JSON body"))
}

#[tokio::test]
async fn an_owner_session_gets_the_release_when_the_role_holds_a_seal_secret() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let expected = secret()
        .key_for(&SealId::parse(SEAL_ID).expect("the id parses"))
        .encoded();

    let (status, body) = release(&composed(&db, Some(secret()))).await;
    assert_eq!(
        (status, body["key"].as_str().unwrap_or_default()),
        (StatusCode::OK, expected.as_str()),
        "the owner's session is released the key identity makes for its seal id"
    );
}

#[tokio::test]
async fn the_release_is_off_when_the_role_holds_no_seal_secret() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database opens");

    let (status, body) = release(&composed(&db, None)).await;
    assert_eq!(
        (status, body["reason"].as_str().unwrap_or_default()),
        (StatusCode::NOT_FOUND, "sync_seal_off"),
        "with no seal secret the owner's session finds the release off"
    );
    assert!(body.get("key").is_none(), "no key is released");
}
