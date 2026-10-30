//! The owner's session routes over the shell's layers: the cookie's attributes, the owner's day
//! behind a live session, the cross-site bound, the handshake bound, the logout, and the Mini App's
//! own requests (SPEC-024 A9, A12 to A15; R5, R6, R8 to R10).
//!
//! The launch payloads were signed by Python's standard `hmac` and `hashlib` for a synthetic bot
//! token that never has the Bot API token's shape, for user ids of fewer than seven digits (R11),
//! all dated 2025-01-15T03:30:00Z. Every clock is a `ManualClock` started ten seconds later, so the
//! payloads are fresh, and the server's study day is still the 14th until 04:00.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{CONTENT_TYPE, RETRY_AFTER, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use deck_streak_api::session_routes::HANDSHAKE_BODY_LIMIT_BYTES;
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
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
/// The session cookie's name, as the Mini App's browser keeps it.
const SESSION_COOKIE: &str = "__Host-deckstreak_session";
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 64 * 1024;

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

/// A refused state change: its method, its headers, and the reason code it is refused with.
type Refused<'a> = (&'a str, Vec<(&'a str, &'a str)>, &'a str);

/// An answer: its status, its headers and its body.
struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
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

/// The Mini App's handshake body for `init_data`, as `JSON.stringify` writes it.
fn handshake_body(init_data: &str) -> String {
    serde_json::json!({ "init_data": init_data }).to_string()
}

/// A same-origin JSON handshake with `init_data`, carrying `cookie` when one is given.
async fn handshake(app: &Router, init_data: &str, cookie: Option<&str>) -> Answer {
    let mut headers = vec![
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
    ];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(
        app,
        "POST",
        "/api/session",
        &headers,
        handshake_body(init_data),
    )
    .await
}

/// A same-origin JSON launch validation with `init_data`, carrying `cookie` when one is given
/// (SPEC-403 R8).
async fn launch(app: &Router, init_data: &str, cookie: Option<&str>) -> Answer {
    let mut headers = vec![
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
    ];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(
        app,
        "POST",
        "/api/launch",
        &headers,
        handshake_body(init_data),
    )
    .await
}

/// `GET /api/me` carrying `cookie`, or no cookie.
async fn me(app: &Router, cookie: Option<&str>) -> Answer {
    let headers: Vec<(&str, &str)> = cookie
        .map(|cookie| ("cookie", cookie))
        .into_iter()
        .collect();
    send(app, "GET", "/api/me", &headers, String::new()).await
}

/// The one `Set-Cookie` an answer carries: its `name=value`, and its attributes by lowercased
/// name, each with its value (empty for a flag).
fn set_cookie(answer: &Answer) -> (String, BTreeMap<String, String>) {
    let values: Vec<&str> = answer
        .headers
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| value.to_str().expect("a text header"))
        .collect();
    assert_eq!(values.len(), 1, "one Set-Cookie: {values:?}");
    let mut parts = values[0].split(';');
    let pair = parts.next().expect("a name and value").trim().to_owned();
    let attributes = parts
        .map(|part| {
            let (name, value) = part.trim().split_once('=').unwrap_or((part.trim(), ""));
            (name.to_ascii_lowercase(), value.to_owned())
        })
        .collect();
    (pair, attributes)
}

/// The attributes the session cookie must carry, with `max_age`.
fn session_attributes(max_age: &str) -> BTreeMap<String, String> {
    [
        ("path", "/"),
        ("max-age", max_age),
        ("secure", ""),
        ("httponly", ""),
        ("samesite", "Strict"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect()
}

/// Opens the owner's session and returns the cookie a browser sends back: `name=value`.
async fn signed_in(app: &Router) -> String {
    let answer = handshake(app, OWNER_PAYLOAD, None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    set_cookie(&answer).0
}

#[tokio::test]
async fn the_session_cookie_is_host_prefixed_secure_httponly_and_strict() {
    let (_clock, app) = app();
    let answer = handshake(&app, OWNER_PAYLOAD, None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

    let (pair, attributes) = set_cookie(&answer);
    let (name, value) = pair.split_once('=').expect("name=value");
    assert_eq!(name, SESSION_COOKIE);
    assert_eq!(value.len(), 64, "the id is 32 bytes of hex");
    // Exactly these: Path=/, a Max-Age of the eight-hour lifetime, Secure, HttpOnly and
    // SameSite=Strict; no Domain, which the __Host- prefix forbids, and no Expires.
    assert_eq!(attributes, session_attributes("28800"));
}

#[tokio::test]
async fn me_answers_the_study_day_only_with_a_live_session() {
    let (clock, app) = app();

    // No session: 401, naming why.
    let refused = me(&app, None).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    assert_eq!(refused.body, "{\"reason\":\"no_session\"}");

    // With the owner's session: the server's study day, which at 03:30 UTC is still the 14th.
    let cookie = signed_in(&app).await;
    let answer = me(&app, Some(&cookie)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    assert_eq!(answer.body, "{\"study_day\":\"2025-01-14\"}");

    // At 03:59 it is still the 14th; past the 04:00 rollover it is the 15th.
    clock.set(UtcMillis::from_epoch_millis(1_736_913_540_000));
    assert_eq!(
        me(&app, Some(&cookie)).await.body,
        "{\"study_day\":\"2025-01-14\"}"
    );
    clock.set(UtcMillis::from_epoch_millis(1_736_913_630_000));
    assert_eq!(
        me(&app, Some(&cookie)).await.body,
        "{\"study_day\":\"2025-01-15\"}"
    );

    // A session idle for 30 minutes has ended.
    clock.advance(Duration::from_mins(30));
    assert_eq!(
        me(&app, Some(&cookie)).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_cross_site_or_non_json_state_change_is_refused() {
    let (_clock, app) = app();
    let cookie = signed_in(&app).await;
    let body = handshake_body(OWNER_PAYLOAD);

    // The owner's own valid launch data, sent cross-site or as something other than JSON: 403,
    // before the route reads it. A cross-site logout carrying the live cookie is refused too.
    let refusals: Vec<Refused> = vec![
        (
            "POST",
            vec![
                ("content-type", "application/json"),
                ("sec-fetch-site", "cross-site"),
            ],
            "cross_site_request",
        ),
        (
            "POST",
            vec![
                ("content-type", "application/json"),
                ("sec-fetch-site", "same-site"),
            ],
            "cross_site_request",
        ),
        (
            "POST",
            vec![
                ("content-type", "application/json"),
                ("sec-fetch-site", "none"),
            ],
            "cross_site_request",
        ),
        (
            "POST",
            vec![
                ("content-type", "text/plain"),
                ("sec-fetch-site", "same-origin"),
            ],
            "not_json",
        ),
        (
            "POST",
            vec![("content-type", "application/x-www-form-urlencoded")],
            "not_json",
        ),
        (
            "POST",
            vec![("content-type", "multipart/form-data; boundary=x")],
            "not_json",
        ),
        ("POST", vec![], "not_json"),
        (
            "DELETE",
            vec![
                ("content-type", "application/json"),
                ("sec-fetch-site", "cross-site"),
                ("cookie", &cookie),
            ],
            "cross_site_request",
        ),
        (
            "DELETE",
            vec![("content-type", "text/plain"), ("cookie", &cookie)],
            "not_json",
        ),
    ];
    for (method, headers, reason) in examined("refused state change(s)", refusals) {
        let answer = send(&app, method, "/api/session", &headers, body.clone()).await;
        assert_eq!(
            (answer.status, answer.body.as_str()),
            (
                StatusCode::FORBIDDEN,
                format!("{{\"reason\":\"{reason}\"}}").as_str()
            ),
            "{method} {headers:?}"
        );
    }
    // The refused logouts ended nothing.
    assert_eq!(me(&app, Some(&cookie)).await.status, StatusCode::OK);

    // The same launch data as same-origin JSON is admitted, and so is JSON from a client that sends
    // no Sec-Fetch-Site at all.
    let admitted = [
        vec![
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
        ],
        vec![("content-type", "application/json; charset=utf-8")],
    ];
    for headers in admitted {
        let answer = send(&app, "POST", "/api/session", &headers, body.clone()).await;
        assert_eq!(
            answer.status,
            StatusCode::OK,
            "{headers:?}: {}",
            answer.body
        );
    }
}

#[tokio::test]
async fn handshakes_past_the_bound_are_refused_with_429() {
    let (clock, app) = app();

    // Thirty forged handshakes in the minute: none is throttled.
    let mut judged = Vec::new();
    for attempt in examined("handshake(s)", (1..=30).collect::<Vec<u32>>()) {
        let answer = handshake(&app, FORGED_PAYLOAD, None).await;
        assert_ne!(
            answer.status,
            StatusCode::TOO_MANY_REQUESTS,
            "handshake {attempt} was throttled"
        );
        judged.push(answer.status);
    }

    // The thirty-first is refused before it is judged, even the owner's: 429 until the minute
    // turns, fifty seconds from 03:30:10.
    let refused = handshake(&app, OWNER_PAYLOAD, None).await;
    assert_eq!(refused.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(refused.body, "{\"reason\":\"too_many_handshakes\"}");
    assert_eq!(
        refused.headers.get(RETRY_AFTER).map(HeaderValue::as_bytes),
        Some(&b"50"[..])
    );
    // The thirty it admitted were judged, and each refused as forged: the bound counts every
    // handshake, which is what caps the HMAC work a flood can buy.
    assert_eq!(judged, vec![StatusCode::UNAUTHORIZED; 30]);
    clock.set(UtcMillis::from_epoch_millis(1_736_911_859_500));
    let last = handshake(&app, OWNER_PAYLOAD, None).await;
    assert_eq!(last.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        last.headers.get(RETRY_AFTER).map(HeaderValue::as_bytes),
        Some(&b"1"[..])
    );

    // The minute turns, and the owner's handshake is admitted.
    clock.advance(Duration::from_millis(500));
    assert_eq!(
        handshake(&app, OWNER_PAYLOAD, None).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn logging_out_ends_the_session_on_the_server() {
    let (_clock, app) = app();
    let cookie = signed_in(&app).await;
    assert_eq!(me(&app, Some(&cookie)).await.status, StatusCode::OK);

    let headers = [
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
        ("cookie", cookie.as_str()),
    ];
    let logout = send(&app, "DELETE", "/api/session", &headers, String::new()).await;
    assert_eq!(logout.status, StatusCode::NO_CONTENT);

    // The server ended it: the same cookie, sent again as a browser that kept it would, is refused.
    let after = me(&app, Some(&cookie)).await;
    assert_eq!(
        (after.status, after.body.as_str()),
        (StatusCode::UNAUTHORIZED, "{\"reason\":\"no_session\"}")
    );

    // And the cookie is cleared: the same name, empty, with a Max-Age of zero and every attribute.
    let (pair, attributes) = set_cookie(&logout);
    assert_eq!(pair, format!("{SESSION_COOKIE}="));
    assert_eq!(attributes, session_attributes("0"));

    // Logging out again, with nothing live, still clears the cookie.
    let again = send(&app, "DELETE", "/api/session", &headers, String::new()).await;
    assert_eq!(again.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn a_handshake_ends_the_session_it_carried() {
    let (_clock, app) = app();
    let first = signed_in(&app).await;

    // A second handshake carrying the first session's cookie gets a new id, and the first ends.
    let answer = handshake(&app, OWNER_PAYLOAD, Some(&first)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let second = set_cookie(&answer).0;
    assert_ne!(second, first);
    assert_eq!(me(&app, Some(&second)).await.status, StatusCode::OK);
    assert_eq!(
        me(&app, Some(&first)).await.status,
        StatusCode::UNAUTHORIZED
    );

    // Another user's valid launch data is refused 403 and opens nothing; a forged one, 401.
    let stranger = handshake(&app, STRANGER_PAYLOAD, None).await;
    assert_eq!(
        (stranger.status, stranger.body.as_str()),
        (StatusCode::FORBIDDEN, "{\"reason\":\"not_owner\"}")
    );
    assert!(stranger.headers.get(SET_COOKIE).is_none());
    let forged = handshake(&app, FORGED_PAYLOAD, None).await;
    assert_eq!(
        (forged.status, forged.body.as_str()),
        (
            StatusCode::UNAUTHORIZED,
            "{\"reason\":\"init_data_invalid\"}"
        )
    );
}

#[tokio::test]
async fn the_mini_apps_own_requests_open_a_session_and_read_the_day() {
    let (_clock, app) = app();
    // Exactly what `web/app/src/lib/api.ts` sends: `content-type: application/json` and
    // `JSON.stringify({ init_data: launch })`, from the page's own origin; then the cookie alone.
    let body = format!(
        "{{\"init_data\":{}}}",
        serde_json::to_string(OWNER_PAYLOAD).expect("a JSON string")
    );
    let opened = send(
        &app,
        "POST",
        "/api/session",
        &[
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
        ],
        body,
    )
    .await;
    // The client reads `response.ok`: any 2xx opens the session.
    assert!(
        opened.status.is_success(),
        "{}: {}",
        opened.status,
        opened.body
    );
    let (cookie, _) = set_cookie(&opened);

    let day = send(
        &app,
        "GET",
        "/api/me",
        &[("cookie", cookie.as_str())],
        String::new(),
    )
    .await;
    assert_eq!(day.status, StatusCode::OK);
    // The client takes `study_day` when it matches its ISO_DATE, /^\d{4}-\d{2}-\d{2}$/.
    let value: serde_json::Value = serde_json::from_str(&day.body).expect("a JSON body");
    let study_day = value["study_day"].as_str().expect("a study_day string");
    let shape: Vec<usize> = study_day.split('-').map(str::len).collect();
    assert_eq!(shape, vec![4, 2, 2], "{study_day}");
    assert!(
        study_day
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'-')
    );
}

#[tokio::test]
async fn the_handshake_body_is_bounded_below_the_shells_limit() {
    assert_eq!(HANDSHAKE_BODY_LIMIT_BYTES, 16 * 1024);
    let (_clock, app) = app();
    let headers = [
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
    ];
    let padded = |length: usize| {
        let frame = handshake_body("").len();
        handshake_body(&"a".repeat(length - frame))
    };

    // A body at the handshake's limit is read, and refused as launch data it is not.
    let at_limit = send(&app, "POST", "/api/session", &headers, padded(16 * 1024)).await;
    assert_eq!(
        (at_limit.status, at_limit.body.as_str()),
        (
            StatusCode::UNAUTHORIZED,
            "{\"reason\":\"init_data_invalid\"}"
        )
    );
    // One byte more is refused 413: the shell's own limit, 2 MiB, would have read it.
    let over = send(
        &app,
        "POST",
        "/api/session",
        &headers,
        padded(16 * 1024 + 1),
    )
    .await;
    assert_eq!(over.status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn the_owner_access_debug_names_its_parts_and_never_the_signing_token() {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    let shown = format!("{access:?}");
    assert!(shown.starts_with("OwnerAccess {"), "{shown}");
    assert!(shown.contains("sessions:"), "{shown}");
    assert!(shown.contains("rule:"), "{shown}");
    assert!(!shown.contains(BOT_TOKEN), "{shown}");
}

#[tokio::test]
async fn a_launch_the_gate_admits_is_accepted_with_204_and_opens_no_session() {
    let (_clock, app) = app();

    // The owner's launch data: 204, no body, no cookie, and no session behind it.
    let accepted = launch(&app, OWNER_PAYLOAD, None).await;
    assert_eq!(accepted.status, StatusCode::NO_CONTENT, "{}", accepted.body);
    assert_eq!(accepted.body, "");
    assert!(
        accepted.headers.get(SET_COOKIE).is_none(),
        "{:?}",
        accepted.headers
    );
    assert_eq!(me(&app, None).await.status, StatusCode::UNAUTHORIZED);

    // A launch carrying a live session's cookie is accepted too, and ends nothing.
    let cookie = signed_in(&app).await;
    let carried = launch(&app, OWNER_PAYLOAD, Some(&cookie)).await;
    assert_eq!(carried.status, StatusCode::NO_CONTENT, "{}", carried.body);
    assert!(carried.headers.get(SET_COOKIE).is_none());
    assert_eq!(me(&app, Some(&cookie)).await.status, StatusCode::OK);
}

#[tokio::test]
async fn a_forged_foreign_stale_or_malformed_launch_is_refused_with_its_reason_alone() {
    let (clock, app) = app();
    let headers = [
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
    ];

    let forged = launch(&app, FORGED_PAYLOAD, None).await;
    let foreign = launch(&app, STRANGER_PAYLOAD, None).await;
    // The malformed launch is a JSON body with no `init_data`.
    let malformed = send(&app, "POST", "/api/launch", &headers, "{}".to_owned()).await;
    // The stale launch is the owner's, validly signed, read past the gate's freshness bound.
    clock.advance(Freshness::default().max_age() + Duration::from_secs(1));
    let stale = launch(&app, OWNER_PAYLOAD, None).await;

    let judged = [
        (
            forged,
            StatusCode::UNAUTHORIZED,
            "{\"reason\":\"init_data_invalid\"}",
        ),
        (foreign, StatusCode::FORBIDDEN, "{\"reason\":\"not_owner\"}"),
        (
            stale,
            StatusCode::UNAUTHORIZED,
            "{\"reason\":\"init_data_stale\"}",
        ),
        (
            malformed,
            StatusCode::UNAUTHORIZED,
            "{\"reason\":\"init_data_invalid\"}",
        ),
    ];
    for (answer, status, body) in examined("refused launch(es)", judged.into_iter().collect()) {
        assert_eq!((answer.status, answer.body.as_str()), (status, body));
        assert!(answer.headers.get(SET_COOKIE).is_none());
    }
}

#[tokio::test]
async fn a_cross_site_or_non_json_launch_is_refused() {
    let (_clock, app) = app();
    let body = handshake_body(OWNER_PAYLOAD);

    // The owner's own valid launch data, sent cross-site or as something other than JSON: 403,
    // before the route reads it.
    let refusals: Vec<Refused> = vec![
        (
            "POST",
            vec![
                ("content-type", "application/json"),
                ("sec-fetch-site", "cross-site"),
            ],
            "cross_site_request",
        ),
        (
            "POST",
            vec![
                ("content-type", "application/json"),
                ("sec-fetch-site", "same-site"),
            ],
            "cross_site_request",
        ),
        (
            "POST",
            vec![
                ("content-type", "text/plain"),
                ("sec-fetch-site", "same-origin"),
            ],
            "not_json",
        ),
        (
            "POST",
            vec![("content-type", "application/x-www-form-urlencoded")],
            "not_json",
        ),
        ("POST", vec![], "not_json"),
    ];
    for (method, headers, reason) in examined("refused launch(es)", refusals) {
        let answer = send(&app, method, "/api/launch", &headers, body.clone()).await;
        assert_eq!(
            (answer.status, answer.body.as_str()),
            (
                StatusCode::FORBIDDEN,
                format!("{{\"reason\":\"{reason}\"}}").as_str()
            ),
            "{method} {headers:?}"
        );
    }

    // The same launch data as same-origin JSON is admitted.
    assert_eq!(
        launch(&app, OWNER_PAYLOAD, None).await.status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn the_launch_body_is_bounded_like_the_handshake() {
    let (_clock, app) = app();
    let headers = [
        ("content-type", "application/json"),
        ("sec-fetch-site", "same-origin"),
    ];
    let padded = |length: usize| {
        let frame = handshake_body("").len();
        handshake_body(&"a".repeat(length - frame))
    };

    // A body at the handshake's limit is read, and refused as launch data it is not.
    let at_limit = send(
        &app,
        "POST",
        "/api/launch",
        &headers,
        padded(HANDSHAKE_BODY_LIMIT_BYTES),
    )
    .await;
    assert_eq!(
        (at_limit.status, at_limit.body.as_str()),
        (
            StatusCode::UNAUTHORIZED,
            "{\"reason\":\"init_data_invalid\"}"
        )
    );
    // One byte more is refused 413: the shell's own limit would have read it.
    let over = send(
        &app,
        "POST",
        "/api/launch",
        &headers,
        padded(HANDSHAKE_BODY_LIMIT_BYTES + 1),
    )
    .await;
    assert_eq!(over.status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn launches_and_handshakes_share_the_handshake_bound() {
    let (_clock, app) = app();

    // Thirty forged launches in the minute: each is judged and refused as forged.
    let mut judged = Vec::new();
    for _ in examined("launch(es)", (1..=30).collect::<Vec<u32>>()) {
        judged.push(launch(&app, FORGED_PAYLOAD, None).await.status);
    }
    assert_eq!(judged, vec![StatusCode::UNAUTHORIZED; 30]);

    // The owner's handshake is the thirty-first request the bound counts: refused before it is judged.
    let refused = handshake(&app, OWNER_PAYLOAD, None).await;
    assert_eq!(refused.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(refused.body, "{\"reason\":\"too_many_handshakes\"}");
}
