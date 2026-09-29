//! The owner's law-drill routes over the shell's layers (SPEC-110 A19; R9): the list, the single
//! view and the answer are served to the owner's live session alone, and the answer appends to the
//! synthetic note once: a second answer to the same drill is refused and the note is unchanged.
//!
//! The launch payloads are SPEC-024's, as in the rollup routes' test. The drill notes are synthetic
//! files in a temporary vault.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_coordination::drills::{
    ARCHIVE_FOLDER, DrillNotes, READINGS_FOLDER, Rails, RealFs, VAULT_ROOT, VaultSettings,
};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Db, Environment, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
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

/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 256 * 1024;
/// The drill every test answers.
const DRILL: &str = "irac-one";

/// A synthetic active note.
fn note() -> String {
    "---\ntype: drill-irac\nsubject: \"Torts\"\ncreated: 2026-02-20\nstatus: active\n---\n# Title one\n\nThe prompt.\n\n## Free Recall\n\n## Self-Check\n\n- [ ] the rule stated\n- [ ] **Ready for grading**\n".to_owned()
}

/// The API over a temporary database and a temporary vault holding one drill; the note's path.
async fn app(scratch: &TempDir) -> (Router, PathBuf) {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let vault = scratch.path().join("vault");
    let active = vault.join("11-Drills").join("Active");
    fs::create_dir_all(&active).expect("the active folder");
    let path = active.join(format!("{DRILL}.md"));
    fs::write(&path, note()).expect("a note");
    let env = Environment::from_vars([
        (VAULT_ROOT, vault.to_str().expect("a utf-8 path")),
        (READINGS_FOLDER, "12-Readings"),
        (ARCHIVE_FOLDER, "Archive"),
    ]);
    let settings = VaultSettings::from_env(&env).expect("settings");
    let notes = DrillNotes::open(&settings, RealFs, Rails::vendored().expect("the rails"))
        .expect("the vault opens");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    let readiness = Readiness::new();
    readiness.database_opened(db);
    let state = ApiState::new(readiness)
        .with_owner(access)
        .with_drills(Arc::new(notes));
    (router(state), path)
}

/// `POST path` with a JSON `body` carrying `cookie`, same-origin.
async fn post(app: &Router, path: &str, body: &Value, cookie: Option<&str>) -> Answer {
    let mut headers = vec![
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
    ];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(app, "POST", path, &headers, body.to_string()).await
}

#[tokio::test]
async fn the_drill_routes_answer_only_the_owner() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (app, note_path) = app(&scratch).await;
    let single = format!("/api/drills/{DRILL}");
    let answer_path = format!("/api/drills/{DRILL}/answer");
    let body = json!({ "answer": "The rule is stated." });

    // No session, a stranger and a forgery: refused, and the note is untouched.
    for (label, answer) in [
        ("list, no session", get(&app, "/api/drills", None).await),
        ("view, no session", get(&app, &single, None).await),
        (
            "answer, no session",
            post(&app, &answer_path, &body, None).await,
        ),
    ] {
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{label}");
    }
    let stranger = handshake(&app, STRANGER_PAYLOAD).await;
    assert_eq!(stranger.status, StatusCode::FORBIDDEN);
    let forged = handshake(&app, FORGED_PAYLOAD).await;
    assert_eq!(forged.status, StatusCode::UNAUTHORIZED);
    assert_eq!(fs::read_to_string(&note_path).expect("the note"), note());

    // The owner lists, reads and answers.
    let cookie = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let list = get(&app, "/api/drills", Some(&cookie)).await;
    assert_eq!(list.status, StatusCode::OK, "{}", list.body);
    let drills = list.json()["drills"]
        .as_array()
        .expect("the drills")
        .clone();
    println!("examined {} drill(s) listed", drills.len());
    assert_eq!(drills.len(), 1, "the one synthetic drill is listed");
    assert_eq!(drills[0]["drill_id"], DRILL);

    let view = get(&app, &single, Some(&cookie)).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.body);
    assert_eq!(
        view.json()["prompt"]
            .as_str()
            .map(|p| p.contains("The prompt.")),
        Some(true)
    );
    let missing = get(&app, "/api/drills/no-such-drill", Some(&cookie)).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);

    let first = post(&app, &answer_path, &body, Some(&cookie)).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let written = fs::read_to_string(&note_path).expect("the note");
    assert!(written.contains("The rule is stated."), "{written}");

    // A second answer is refused, and the note keeps the first.
    let second = post(
        &app,
        &answer_path,
        &json!({ "answer": "Another." }),
        Some(&cookie),
    )
    .await;
    assert_eq!(second.status, StatusCode::CONFLICT, "{}", second.body);
    assert_eq!(second.json()["reason"], "already_answered");
    assert_eq!(fs::read_to_string(&note_path).expect("the note"), written);

    // An empty answer is refused and writes nothing.
    let empty = post(
        &app,
        &answer_path,
        &json!({ "answer": "  " }),
        Some(&cookie),
    )
    .await;
    assert_eq!(
        empty.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        empty.body
    );
}
