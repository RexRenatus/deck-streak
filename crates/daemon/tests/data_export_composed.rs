//! The owner's export through the daemon's composed state (SPEC-516 R2, B2): over one seeded
//! database, the route the `api` role serves answers the very bytes `deckstreakd data export`
//! writes to standard output.

// An integration test is test code: its helpers panic on a failed fixture, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::ffi::OsString;
use std::process::Command;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{Request, StatusCode};
use deck_streak_api::data_export_routes::EXPORT_PATH;
use deck_streak_api::{OwnerAccess, Readiness, router};
use deck_streak_daemon::role_api::api_state;
use deck_streak_daemon::wiring::DATABASE_FILE;
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_ingest::sync_runs::{ReasonCode, SqliteSyncRuns, SyncRun, SyncRunStore, Trigger};
use deck_streak_kernel::{
    Clock, Db, Environment, ManualClock, Offload, OffloadWorkers, StudyDay, StudyDayRule,
    TelegramUserId, UtcMillis,
};
use serde_json::json;
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
const BODY_READ_LIMIT: usize = 1024 * 1024;

/// Records one synthetic sync run in the role's database in `directory`, so the export holds a
/// row beside its singletons.
async fn with_one_sync_run(directory: &std::path::Path) {
    let db = Db::open(&directory.join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    SqliteSyncRuns::new(db.clone())
        .record(&SyncRun {
            trigger: Trigger::Owner,
            started_at: UtcMillis::from_epoch_millis(1_000),
            finished_at: UtcMillis::from_epoch_millis(2_000),
            study_day: StudyDay::from_epoch_day(20_000),
            outcome: Err(ReasonCode::ServerError),
            attempts: 3,
            full_download: false,
        })
        .await
        .expect("a synthetic run is recorded");
    db.close().await;
}

/// What `deckstreakd data export` writes to standard output over the state directory `directory`.
fn data_role_export(directory: &std::path::Path) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_deckstreakd"))
        .args(["data", "export"])
        .env_clear()
        .env("STATE_DIRECTORY", directory.as_os_str())
        .output()
        .expect("the binary runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// The router the `api` role serves over `db`, its state composed as the role composes it.
fn composed(db: &Db) -> Router {
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
    router(api_state(&env, &offload, readiness, access))
}

/// The owner's session cookie, opened through `app` from the signed launch data.
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

#[tokio::test]
async fn the_route_answers_the_bytes_the_data_role_writes() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    with_one_sync_run(dir.path()).await;
    let written = data_role_export(dir.path());

    let db = Db::open(&dir.path().join(DATABASE_FILE))
        .await
        .expect("the role's database opens");
    let app = composed(&db);
    let cookie = owner_cookie(&app).await;
    let answer = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(EXPORT_PATH)
                .header("content-type", "application/json")
                .header("sec-fetch-site", "same-origin")
                .header("cookie", cookie)
                .body(Body::from("{}"))
                .expect("a request"),
        )
        .await
        .expect("the router is infallible");
    let status = answer.status();
    let served = to_bytes(answer.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a body")
        .to_vec();
    db.close().await;

    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&served)
    );
    // The seeded run is in both, so the equality is over a document that holds a row.
    let lines: Vec<&str> = String::from_utf8_lossy(&written)
        .lines()
        .map(|_| "line")
        .collect();
    println!("examined {} line(s) the data role wrote", lines.len());
    assert_eq!(lines.len(), 1, "the role writes one line");
    let document: serde_json::Value =
        serde_json::from_slice(&written).expect("the role writes one JSON object");
    assert_eq!(
        document["sync_runs"].as_array().map(Vec::len),
        Some(1),
        "{document}"
    );
    println!(
        "examined {} byte(s) written and {} byte(s) served",
        written.len(),
        served.len()
    );
    assert_eq!(served, written, "the route answers the data role's bytes");
}
