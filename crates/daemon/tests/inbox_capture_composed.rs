//! The quick capture through the daemon's composed router (SPEC-118 R4, R10): the `api` role's
//! state, built over a configured vault root and the layout in force, serves
//! `POST /api/inbox/captures` into that vault's inbox, so the inbox the daemon opens is the one the
//! route writes. A role with no vault root, a root of the wrong shape, or a layout file that cannot
//! be read serves the route as 503 `vault_not_open` and writes nothing.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{Request, StatusCode};
use deck_streak_api::{OwnerAccess, Readiness, router};
use deck_streak_daemon::role_api::api_state;
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{
    Clock, Db, Environment, ManualClock, Offload, OffloadWorkers, StudyDayRule, TelegramUserId,
    UtcMillis,
};
use deck_streak_vault::config::{VAULT_LAYOUT, VAULT_ROOT};
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
/// The inbox folder of the vendored layout (`crates/vault/data/layout.json`), in force while
/// `DECKSTREAK_VAULT_LAYOUT` is unset.
const INBOX: &str = "90-Inbox";
/// A capture id as the Mini App mints one: 32 hexadecimal digits.
const CAPTURE_ID: &str = "6f1d2c9a0b1c4d5e8f9a0b1c2d3e4f50";

/// The router the `api` role serves with the settings `vars`, over an open database in `scratch`.
async fn composed(scratch: &Path, vars: &[(&str, &str)]) -> Router {
    let env = Environment::from_vars(vars.iter().copied());
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
    fs::create_dir_all(scratch).expect("a database folder");
    let readiness = Readiness::new();
    readiness.database_opened(
        Db::open(&scratch.join("deck_streak.db"))
            .await
            .expect("the database opens"),
    );
    router(api_state(&env, &offload, readiness, access))
}

/// The owner's quick capture of `text` through `app`, after a same-origin handshake: the answer's
/// status and its JSON body.
async fn owner_capture(app: &Router, text: &str) -> (StatusCode, Value) {
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
    assert_eq!(opened.status(), StatusCode::OK, "the owner's handshake");
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
    let answer = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/inbox/captures")
                .header("content-type", "application/json")
                .header("sec-fetch-site", "same-origin")
                .header("cookie", cookie)
                .body(Body::from(
                    json!({ "capture_id": CAPTURE_ID, "kind": "text", "text": text }).to_string(),
                ))
                .expect("a request"),
        )
        .await
        .expect("the router is infallible");
    let status = answer.status();
    let bytes = to_bytes(answer.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    (status, serde_json::from_slice(&bytes).expect("a JSON body"))
}

/// The names in `folder`, sorted.
fn files(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .expect("the folder lists")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .into_string()
                .expect("a UTF-8 name")
        })
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn the_api_role_serves_the_quick_capture_over_its_configured_vault_alone() {
    let scratch = tempfile::tempdir().expect("a scratch folder");
    let vault = scratch.path().join("vault");
    let inbox = vault.join(INBOX);
    fs::create_dir_all(&inbox).expect("the inbox folder");
    let root = vault.to_str().expect("a UTF-8 path");

    // A root and the vendored layout: the capture lands in that vault's inbox.
    let configured = composed(&scratch.path().join("configured"), &[(VAULT_ROOT, root)]).await;
    let (status, body) = owner_capture(&configured, "call the clinic").await;
    let name = format!("2025-01-15-text-{CAPTURE_ID}.md");
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body, json!({ "name": name }));
    assert_eq!(files(&inbox), vec![name]);

    // No usable vault: no root, a root that is not absolute, or a layout file that cannot be read.
    let absent = scratch.path().join("absent-layout.json");
    let absent = absent.to_str().expect("a UTF-8 path");
    let unusable: [&[(&str, &str)]; 3] = [
        &[],
        &[(VAULT_ROOT, "relative/vault")],
        &[(VAULT_ROOT, root), (VAULT_LAYOUT, absent)],
    ];
    for (index, vars) in unusable.iter().enumerate() {
        let app = composed(&scratch.path().join(format!("unusable-{index}")), vars).await;
        let (status, body) = owner_capture(&app, "a second thought").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "case {index}: {body}");
        assert_eq!(body["reason"], "vault_not_open", "case {index}");
    }
    println!(
        "examined 1 configured and {} unusable vault setting(s)",
        unusable.len()
    );
    assert_eq!(files(&inbox).len(), 1, "no unusable vault wrote a capture");
}
