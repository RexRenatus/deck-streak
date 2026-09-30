//! The owner's read tap over the shell's layers (SPEC-047 A11, R9): the route answers the owner's
//! live session alone, a stranger and a visitor change no state, and the owner's answer carries the
//! reading's covered and studied counts and its verdict.
//!
//! The launch payloads are SPEC-024's, signed for a synthetic bot token that never has the Bot API
//! token's shape, all dated 2025-01-15T03:30:00Z. The tap itself is a fake behind the coordination
//! port, so the test reaches no vault and no ledger.

// An integration test is test code: its helpers panic on a failed fixture, and counts are printed
// on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_coordination::readings::generate::PortFuture;
use deck_streak_coordination::readings::read_tap::{ReadTapPort, TapAnswer, TapError, Verdict};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use serde_json::{Value, json};
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
/// 2025-01-15T03:30:10Z, ten seconds after the payloads were signed, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 64 * 1024;
/// The id of the one reading the fake tap knows: 32 lowercase hex digits.
const KNOWN_ID: &str = "0123456789abcdef0123456789abcdef";
/// The route of a reading's read tap.
fn tap_path(id: &str) -> String {
    format!("/api/readings/{id}/read")
}

/// A tap that knows one reading and counts every call it receives.
#[derive(Default)]
struct FakeTap {
    calls: AtomicU32,
}

impl ReadTapPort for FakeTap {
    fn tap<'a>(&'a self, id: &'a str) -> PortFuture<'a, Result<TapAnswer, TapError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let known = id == KNOWN_ID;
        Box::pin(async move {
            if known {
                Ok(TapAnswer {
                    id: KNOWN_ID.to_owned(),
                    read_at: UtcMillis::from_epoch_millis(STARTED_AT),
                    first: true,
                    covered: 5,
                    studied: 4,
                    verdict: Verdict::Studied,
                })
            } else {
                Err(TapError::NotFound)
            }
        })
    }
}

fn app(tap: &Arc<FakeTap>) -> Router {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    let port: Arc<dyn ReadTapPort> = tap.clone();
    router(
        ApiState::new(Readiness::new())
            .with_owner(access)
            .with_readings(port),
    )
}

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

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

fn cookie_of(answer: &Answer) -> String {
    let value = answer
        .headers
        .get(SET_COOKIE)
        .expect("a Set-Cookie")
        .to_str()
        .expect("a text header");
    value.split(';').next().expect("a pair").trim().to_owned()
}

async fn post(app: &Router, path: &str, cookie: Option<&str>) -> Answer {
    let mut headers = vec![("sec-fetch-site", "same-origin")];
    if let Some(cookie) = cookie {
        headers.push((COOKIE.as_str(), cookie));
    }
    send(app, "POST", path, &headers, String::new()).await
}

#[tokio::test]
async fn the_read_route_answers_only_the_owner() {
    let tap = Arc::new(FakeTap::default());
    let app = app(&tap);

    // A visitor with no session, or an unknown one, changes nothing.
    let unknown = format!("__Host-deckstreak_session={}", "0".repeat(64));
    for cookie in [None, Some(unknown.as_str())] {
        let refused = post(&app, &tap_path(KNOWN_ID), cookie).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{cookie:?}");
    }

    // A stranger never gets a session, so never reaches the tap.
    let stranger = handshake(&app, STRANGER_PAYLOAD).await;
    assert_eq!(stranger.status, StatusCode::FORBIDDEN, "{}", stranger.body);
    assert_eq!(tap.calls.load(Ordering::SeqCst), 0, "no tap was made");
    println!("examined 3 refused request(s)");

    // The owner's live session taps the reading; R9: the answer names covered, studied, verdict.
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let answer = post(&app, &tap_path(KNOWN_ID), Some(&owner)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let body: Value = serde_json::from_str(&answer.body).expect("a JSON body");
    assert_eq!(body["id"], json!(KNOWN_ID), "{body}");
    assert_eq!(body["first"], json!(true), "{body}");
    assert_eq!(body["covered"], json!(5), "{body}");
    assert_eq!(body["studied"], json!(4), "{body}");
    assert_eq!(body["verdict"], json!("studied"), "{body}");
    assert_eq!(tap.calls.load(Ordering::SeqCst), 1, "one tap");

    // An unknown reading is 404 with a reason code alone.
    let missing = post(
        &app,
        &tap_path("ffffffffffffffffffffffffffffffff"),
        Some(&owner),
    )
    .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.body);
    let reason: Value = serde_json::from_str(&missing.body).expect("a JSON body");
    assert_eq!(reason, json!({ "reason": "reading_not_found" }));
}
