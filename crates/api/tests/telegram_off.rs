//! The API with Telegram off (SPEC-582 A6; R4): no owner's gate is held, so the handshake and the
//! launch refuse a well-formed body as `telegram_off` and keep `init_data_invalid` for a malformed
//! one; the two link-code routes answer `telegram_off` after the state-change guard and before
//! `linking_off`; the passkey routes and the owner's methods answer `linking_off`; and the owner's
//! day answers `no_session`.
//!
//! The router is built the way `session_routes.rs` builds it, with
//! [`OwnerAccess::telegram_off`] in place of the owner's gate. The launch payload is that file's
//! owner payload: signed for a synthetic bot token, and well formed, which is all an off gate reads.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_kernel::{ManualClock, StudyDayRule, UtcMillis};
use tower::ServiceExt;

/// The owner's launch data as `session_routes.rs` holds it, signed for its synthetic bot token.
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
/// The headers of a same-origin JSON request: what the state-change guard admits.
const STATE_CHANGE: [(&str, &str); 2] = [
    ("content-type", "application/json"),
    ("sec-fetch-site", "same-origin"),
];
/// The answer body of `telegram_off`.
const TELEGRAM_OFF: &str = "{\"reason\":\"telegram_off\"}";
/// The answer body of `linking_off`.
const LINKING_OFF: &str = "{\"reason\":\"linking_off\"}";

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The API as the daemon builds it with neither Telegram credential held, on a manual clock.
fn app() -> Router {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let access = OwnerAccess::telegram_off(clock, StudyDayRule::default());
    router(ApiState::new(Readiness::new()).with_owner(access))
}

/// `method` on `path` with `headers` and `body`, through the whole app: the status and the body.
async fn send(
    app: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: String,
) -> (StatusCode, String) {
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
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    (
        status,
        String::from_utf8(bytes.to_vec()).expect("a UTF-8 body"),
    )
}

#[tokio::test]
async fn a_well_formed_handshake_or_launch_answers_telegram_off_and_a_malformed_one_keeps_init_data_invalid()
 {
    let app = app();
    let well_formed = [
        serde_json::json!({ "init_data": OWNER_PAYLOAD }).to_string(),
        serde_json::json!({ "init_data": "not launch data" }).to_string(),
    ];
    for path in examined("telegram route(s)", vec!["/api/session", "/api/launch"]) {
        // A well-formed body, whatever its launch data, is refused as Telegram off: no gate is
        // held to judge it.
        for body in &well_formed {
            assert_eq!(
                send(&app, "POST", path, &STATE_CHANGE, body.clone()).await,
                (StatusCode::NOT_FOUND, TELEGRAM_OFF.to_owned()),
                "{path} with {body}"
            );
        }
        // A body that is not the handshake's JSON keeps its own refusal.
        assert_eq!(
            send(
                &app,
                "POST",
                path,
                &STATE_CHANGE,
                "not the handshake's JSON".to_owned()
            )
            .await,
            (
                StatusCode::UNAUTHORIZED,
                "{\"reason\":\"init_data_invalid\"}".to_owned()
            ),
            "{path} with a malformed body"
        );
        // The state-change guard still comes first.
        let cross_site = [
            ("content-type", "application/json"),
            ("sec-fetch-site", "cross-site"),
        ];
        let (status, _) = send(&app, "POST", path, &cross_site, well_formed[0].clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path} across sites");
    }
}

#[tokio::test]
async fn the_link_code_routes_answer_telegram_off_and_the_passkey_routes_linking_off() {
    let app = app();
    let redeem = serde_json::json!({ "code": "synthetic-code" }).to_string();

    // The link code is Telegram's: with Telegram off both of its routes say so, before linking_off,
    // which they would answer otherwise (no linking configuration is held).
    let telegram_off = [
        ("/api/link/code", String::new()),
        ("/api/link/redeem", redeem.clone()),
    ];
    for (path, body) in examined("link-code route(s)", telegram_off.to_vec()) {
        assert_eq!(
            send(&app, "POST", path, &STATE_CHANGE, body.clone()).await,
            (StatusCode::NOT_FOUND, TELEGRAM_OFF.to_owned()),
            "{path}"
        );
        // After the state-change guard: a cross-site request is refused before Telegram is judged.
        let cross_site = [
            ("content-type", "application/json"),
            ("sec-fetch-site", "cross-site"),
        ];
        let (status, _) = send(&app, "POST", path, &cross_site, body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path} across sites");
    }

    // The passkey routes and the owner's methods are linking's, and linking is off.
    let linking_off = [
        ("POST", "/api/passkeys/register/start", "{}"),
        ("POST", "/api/passkeys/register/finish", "{}"),
        ("POST", "/api/passkeys/sign-in/start", "{}"),
        ("POST", "/api/passkeys/sign-in/finish", "{}"),
        ("GET", "/api/identities", ""),
        ("DELETE", "/api/identities/1", ""),
    ];
    for (method, path, body) in examined("linking route(s)", linking_off.to_vec()) {
        assert_eq!(
            send(&app, method, path, &STATE_CHANGE, body.to_owned()).await,
            (StatusCode::NOT_FOUND, LINKING_OFF.to_owned()),
            "{method} {path}"
        );
    }

    // The owner's day needs a session, and none can be opened.
    assert_eq!(
        send(&app, "GET", "/api/me", &[], String::new()).await,
        (
            StatusCode::UNAUTHORIZED,
            "{\"reason\":\"no_session\"}".to_owned()
        ),
        "GET /api/me"
    );
}
