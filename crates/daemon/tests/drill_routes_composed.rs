//! The drill routes through the daemon's composed router (SPEC-110 R13, R15): the `api` role's
//! state, built over a configured vault, answers the owner's list and refuses a caller with no
//! session, so the vault the daemon opens is the one the routes read.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{Request, Response, StatusCode};
use deck_streak_api::{OwnerAccess, Readiness, router};
use deck_streak_coordination::drills::{ARCHIVE_FOLDER, READINGS_FOLDER, VAULT_ROOT};
use deck_streak_daemon::role_api::api_state;
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{
    Clock, Environment, ManualClock, Offload, OffloadWorkers, StudyDayRule, TelegramUserId,
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
/// A synthetic active note.
const NOTE: &str = "---\ntype: drill-irac\nsubject: \"Torts\"\ncreated: 2026-02-20\nstatus: active\n---\n# Title one\n\nThe prompt.\n\n## Free Recall\n\n## Self-Check\n\n- [ ] the rule stated\n- [ ] **Ready for grading**\n";

/// The router the `api` role serves over the vault at `root`.
fn composed(root: &str) -> Router {
    let env = Environment::from_vars([
        (VAULT_ROOT, root),
        (READINGS_FOLDER, "12-Readings"),
        (ARCHIVE_FOLDER, "Archive"),
    ]);
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

/// The router's answer to `request`.
async fn answer(app: &Router, request: Request<Body>) -> Response<Body> {
    app.clone()
        .oneshot(request)
        .await
        .expect("the router is infallible")
}

#[tokio::test]
async fn the_composed_router_serves_the_configured_vaults_drills_to_the_owner_alone() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let vault = dir.path().join("vault");
    let active = vault.join("11-Drills").join("Active");
    fs::create_dir_all(&active).expect("the active folder");
    fs::write(active.join("irac-one.md"), NOTE).expect("a note");
    let app = composed(vault.to_str().expect("a utf-8 path"));

    let anonymous = answer(
        &app,
        Request::builder()
            .uri("/api/drills")
            .body(Body::empty())
            .expect("a request"),
    )
    .await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    let opened = answer(
        &app,
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
    .await;
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
    let listed = answer(
        &app,
        Request::builder()
            .uri("/api/drills")
            .header("cookie", cookie)
            .body(Body::empty())
            .expect("a request"),
    )
    .await;
    assert_eq!(listed.status(), StatusCode::OK);
    let bytes = to_bytes(listed.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    let body: Value = serde_json::from_slice(&bytes).expect("a JSON body");
    let ids: Vec<&str> = body["drills"]
        .as_array()
        .expect("the drills")
        .iter()
        .map(|drill| drill["drill_id"].as_str().expect("an id"))
        .collect();
    println!("examined {} drill(s) listed", ids.len());
    assert_eq!(ids, vec!["irac-one"]);
    assert_eq!(body["awaiting_grading"], 0);
    assert_eq!(body["deferred"], 0);
}
