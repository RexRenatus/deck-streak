//! The linking routes over the shell's layers: who reaches them, the ceremony cookie, the ceremony
//! bound, the cross-site bound and the owner's methods list (SPEC-359 R6 to R10; A30 to A34).
//!
//! Every clock is a `ManualClock`. The owner and the bot are synthetic: a user id of fewer than
//! seven digits and a token that never has the Bot API token's shape.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use tower::ServiceExt;

/// The synthetic bot token.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// The synthetic owner.
const OWNER: i64 = 4242;
/// 2025-01-15T03:30:10Z, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 64 * 1024;
/// Every state-changing route of SPEC-359, by its method and a path it answers on (R10).
const STATE_CHANGING_ROUTES: [(&str, &str); 7] = [
    ("POST", "/api/link/code"),
    ("POST", "/api/link/redeem"),
    ("POST", "/api/passkeys/register/start"),
    ("POST", "/api/passkeys/register/finish"),
    ("POST", "/api/passkeys/sign-in/start"),
    ("POST", "/api/passkeys/sign-in/finish"),
    ("DELETE", "/api/identities/1"),
];

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The API as the daemon builds it, for the synthetic owner and bot, on a manual clock.
fn app() -> (Arc<ManualClock>, Router) {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock.clone(), StudyDayRule::default());
    let app = router(ApiState::new(Readiness::new()).with_owner(access));
    (clock, app)
}

/// An answer: its status and its body.
struct Answer {
    status: StatusCode,
    body: String,
}

/// `method` on `path` with `headers` and `body`, through the whole app.
async fn send(
    app: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> Answer {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(body.to_owned()))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router is infallible");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    Answer {
        status,
        body: String::from_utf8(bytes.to_vec()).expect("a UTF-8 body"),
    }
}

/// A33: every state-changing route of this SPEC refuses a cross-site request 403
/// `cross_site_request` before it reads a byte of the body, over the routes the app serves.
#[tokio::test]
async fn every_linking_route_refuses_a_cross_site_request() {
    let (_clock, app) = app();
    let same_origin = [
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
    ];
    // A route the app serves answers a same-origin request with a status of its own or a reason;
    // an unrouted path is the router's bare 404, with no body.
    let mut served = Vec::new();
    for (method, path) in STATE_CHANGING_ROUTES {
        let answer = send(&app, method, path, &same_origin, "{}").await;
        if !(answer.status == StatusCode::NOT_FOUND && answer.body.is_empty()) {
            served.push((method, path));
        }
    }
    let served = examined("linking routes served", served);
    assert_eq!(
        served.len(),
        STATE_CHANGING_ROUTES.len(),
        "a state-changing route is not served: {served:?}"
    );
    let cross_site = [
        ("content-type", "application/json"),
        ("sec-fetch-site", "cross-site"),
    ];
    for (method, path) in served {
        // The body is not JSON: a route that read it first would answer something else.
        let answer = send(&app, method, path, &cross_site, "{\"unterminated").await;
        assert_eq!(
            (answer.status, answer.body.as_str()),
            (StatusCode::FORBIDDEN, r#"{"reason":"cross_site_request"}"#),
            "{method} {path} answered a cross-site request otherwise"
        );
    }
}
