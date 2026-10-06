//! The linking routes over the shell's layers: who reaches them, the ceremony cookie, the ceremony
//! bound, the cross-site bound and the owner's methods list (SPEC-359 R6 to R10; A30 to A34).
//!
//! Every clock is a `ManualClock`. The owner and the bot are synthetic: a user id of fewer than
//! seven digits and a token that never has the Bot API token's shape.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{RETRY_AFTER, SET_COOKIE};
use axum::http::{HeaderMap, Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::{Freshness, LinkingConfig, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Db, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

/// The synthetic bot token.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// The synthetic owner.
const OWNER: i64 = 4242;
/// 2025-01-15T03:30:10Z, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The web client's origin the API is configured with.
const ORIGIN: &str = "https://app.example";
/// The owner's launch data, validly signed for [`BOT_TOKEN`] ten seconds before [`STARTED_AT`]
/// (the payload `tests/session_routes.rs` signs).
const OWNER_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=79502d49032542c5030e80b666d03f5af53e053f97d387806adb0aa843ddeb2d",
);
/// A credential id the A34 row holds: bytes 0xA0 to 0xAF, as `SQLite`'s hex spells them.
const CREDENTIAL_HEX: &str = "A0A1A2A3A4A5A6A7A8A9AAABACADAEAF";
/// The same credential id, base64url.
const CREDENTIAL_B64: &str = "oKGio6SlpqeoqaqrrK2urw";
/// A user handle the A34 row holds: bytes 0xC0 to 0xCF, as `SQLite`'s hex spells them.
const HANDLE_HEX: &str = "C0C1C2C3C4C5C6C7C8C9CACBCCCDCECF";
/// The same user handle, base64url.
const HANDLE_B64: &str = "wMHCw8TFxsfIycrLzM3Ozw";
/// A JSON request a browser sends from the API's own origin.
const SAME_ORIGIN: [(&str, &str); 2] = [
    ("content-type", "application/json"),
    ("sec-fetch-site", "same-origin"),
];
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

/// The API as the daemon builds it, for the synthetic owner and bot, linking on at [`ORIGIN`],
/// over a migrated temporary database, on a manual clock.
struct App {
    router: Router,
    db: Db,
    /// The database's directory: a test binds it, or the directory is deleted under the database.
    _dir: TempDir,
}

/// The API as the daemon builds it.
async fn app() -> App {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck-streak.db"))
        .await
        .expect("the database opens and migrates");
    let readiness = Readiness::new();
    readiness.database_opened(db.clone());
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let owner = Owner::new(TelegramUserId::new(OWNER));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        owner,
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    let config = LinkingConfig::from_setting(Some(ORIGIN)).expect("the origin is an https origin");
    let router = router(
        ApiState::new(readiness)
            .with_owner(access)
            .with_linking(config, owner),
    );
    App {
        router,
        db,
        _dir: dir,
    }
}

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

/// A same-origin JSON `method` on `path` with `body`, carrying `cookie` when one is given.
async fn call(app: &Router, method: &str, path: &str, cookie: Option<&str>, body: &str) -> Answer {
    let mut headers = SAME_ORIGIN.to_vec();
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(app, method, path, &headers, body).await
}

/// The `Set-Cookie` headers an answer carries, each as its `name=value` and its attributes by
/// lowercased name, each with its value (empty for a flag).
fn set_cookies(answer: &Answer) -> Vec<(String, BTreeMap<String, String>)> {
    answer
        .headers
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| {
            let mut parts = value.to_str().expect("a text header").split(';');
            let pair = parts.next().expect("a name and value").trim().to_owned();
            let attributes = parts
                .map(|part| {
                    let (name, value) = part.trim().split_once('=').unwrap_or((part.trim(), ""));
                    (name.to_ascii_lowercase(), value.to_owned())
                })
                .collect();
            (pair, attributes)
        })
        .collect()
}

/// The session cookie an answer opens, as the request header that carries it back.
fn session_cookie(answer: &Answer) -> String {
    set_cookies(answer)
        .into_iter()
        .map(|(pair, _)| pair)
        .find(|pair| pair.starts_with("__Host-deckstreak_session=") && pair.len() > 26)
        .unwrap_or_else(|| panic!("no session cookie opened: {:?}", answer.headers))
}

/// The owner's handshake: a `telegram` session's cookie.
async fn telegram_session(app: &Router) -> String {
    let body = json!({ "init_data": OWNER_PAYLOAD }).to_string();
    let answer = call(app, "POST", "/api/session", None, &body).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    session_cookie(&answer)
}

/// A link code minted in `telegram` and redeemed: the `link` session's cookie.
async fn link_session(app: &Router, telegram: &str) -> String {
    let minted = call(app, "POST", "/api/link/code", Some(telegram), "{}").await;
    assert_eq!(minted.status, StatusCode::OK, "{}", minted.body);
    let minted: Value = serde_json::from_str(&minted.body).expect("a JSON body");
    let code = minted["code"].as_str().expect("a code");
    let body = json!({ "code": code }).to_string();
    let redeemed = call(app, "POST", "/api/link/redeem", None, &body).await;
    assert_eq!(redeemed.status, StatusCode::NO_CONTENT, "{}", redeemed.body);
    session_cookie(&redeemed)
}

/// A33: every state-changing route of this SPEC refuses a cross-site request 403
/// `cross_site_request` before it reads a byte of the body, over the routes the app serves.
#[tokio::test]
async fn every_linking_route_refuses_a_cross_site_request() {
    let App {
        router: app, _dir, ..
    } = app().await;
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

/// A30: without a session, the linking routes that need one answer 401; a `link` session reaches
/// registration and nothing else of the owner's.
#[tokio::test]
async fn a_link_session_reaches_only_the_linking_routes() {
    let App {
        router: app, _dir, ..
    } = app().await;
    let needs_a_session = [
        ("POST", "/api/link/code"),
        ("POST", "/api/passkeys/register/start"),
        ("POST", "/api/passkeys/register/finish"),
        ("GET", "/api/identities"),
        ("DELETE", "/api/identities/1"),
    ];
    for (method, path) in examined("routes that need a session", needs_a_session.to_vec()) {
        let answer = call(&app, method, path, None, "{}").await;
        assert_eq!(
            answer.status,
            StatusCode::UNAUTHORIZED,
            "{method} {path} answered without a session: {}",
            answer.body
        );
    }

    let telegram = telegram_session(&app).await;
    let link = link_session(&app, &telegram).await;
    let owner_routes = [
        ("GET", "/api/me"),
        ("GET", "/api/identities"),
        ("DELETE", "/api/identities/1"),
        ("POST", "/api/link/code"),
    ];
    for (method, path) in examined("owner routes", owner_routes.to_vec()) {
        let answer = call(&app, method, path, Some(&link), "{}").await;
        assert_eq!(
            answer.status,
            StatusCode::UNAUTHORIZED,
            "a link session reached {method} {path}: {}",
            answer.body
        );
    }
    let registration = call(
        &app,
        "POST",
        "/api/passkeys/register/start",
        Some(&link),
        "{}",
    )
    .await;
    assert_eq!(
        registration.status,
        StatusCode::OK,
        "a link session did not reach registration: {}",
        registration.body
    );
}

/// A31: a registration start sets the ceremony cookie, `__Host-` prefixed, strict and as short-
/// lived as the ceremony; its finish clears it.
#[tokio::test]
async fn the_ceremony_cookie_is_host_prefixed_strict_and_short_lived() {
    let App {
        router: app, _dir, ..
    } = app().await;
    let telegram = telegram_session(&app).await;
    let link = link_session(&app, &telegram).await;

    let started = call(
        &app,
        "POST",
        "/api/passkeys/register/start",
        Some(&link),
        "{}",
    )
    .await;
    assert_eq!(started.status, StatusCode::OK, "{}", started.body);
    let cookies = set_cookies(&started);
    assert_eq!(cookies.len(), 1, "one Set-Cookie: {cookies:?}");
    let (pair, attributes) = &cookies[0];
    let flow = pair
        .strip_prefix("__Host-deckstreak_ceremony=")
        .unwrap_or_else(|| panic!("the start set another cookie: {pair}"));
    assert_eq!(flow.len(), 64, "the flow id is 32 bytes as hex");
    assert!(flow.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let strict = |max_age: &str| -> BTreeMap<String, String> {
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
    };
    assert_eq!(attributes, &strict("300"));

    let ceremony = format!("{link}; {pair}");
    let finished = call(
        &app,
        "POST",
        "/api/passkeys/register/finish",
        Some(&ceremony),
        "{}",
    )
    .await;
    let cleared: Vec<_> = set_cookies(&finished)
        .into_iter()
        .filter(|(pair, _)| pair.starts_with("__Host-deckstreak_ceremony="))
        .collect();
    assert_eq!(
        cleared,
        [("__Host-deckstreak_ceremony=".to_owned(), strict("0"))],
        "the finish did not clear the ceremony cookie"
    );
}

/// A32: thirty ceremony starts in a minute are served; the thirty-first, and a redeem, are refused
/// 429 `too_many_ceremonies` with `Retry-After`, and the handshake still answers.
#[tokio::test]
async fn a_ceremony_flood_is_bounded_and_spares_the_handshake() {
    let App {
        router: app, _dir, ..
    } = app().await;
    let starts: Vec<u32> = (1..=30).collect();
    for attempt in examined("ceremony starts", starts) {
        let answer = call(&app, "POST", "/api/passkeys/sign-in/start", None, "{}").await;
        assert_ne!(
            answer.status,
            StatusCode::TOO_MANY_REQUESTS,
            "ceremony start {attempt} was throttled"
        );
    }
    let code = json!({ "code": "AAAAAAAAAAAAAAAAAAAAAA" }).to_string();
    for (path, body) in [
        ("/api/passkeys/sign-in/start", "{}"),
        ("/api/link/redeem", code.as_str()),
    ] {
        let refused = call(&app, "POST", path, None, body).await;
        assert_eq!(
            (refused.status, refused.body.as_str()),
            (
                StatusCode::TOO_MANY_REQUESTS,
                r#"{"reason":"too_many_ceremonies"}"#
            ),
            "{path} was not bounded"
        );
        // 03:30:10 is fifty seconds before the minute turns.
        assert_eq!(
            refused
                .headers
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok()),
            Some("50"),
            "{path} named no Retry-After"
        );
    }
    let body = json!({ "init_data": OWNER_PAYLOAD }).to_string();
    let handshake = call(&app, "POST", "/api/session", None, &body).await;
    assert_eq!(
        handshake.status,
        StatusCode::OK,
        "the ceremony bound throttled the handshake: {}",
        handshake.body
    );
}

/// A34: the methods list names Telegram first, then each passkey by its row id and instants, and
/// never a credential id or a user handle.
#[tokio::test]
async fn the_identities_list_names_methods_without_credential_ids() {
    let App {
        router: app,
        db,
        _dir,
    } = app().await;
    let mut write = db.write().await.expect("a write");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO passkeys (telegram_user_id, credential_id, user_handle, credential, counter, \
         backup_state, created_at) VALUES (?1, X'{CREDENTIAL_HEX}', X'{HANDLE_HEX}', '{{}}', 0, 0, \
         ?2)"
    )))
    .bind(OWNER)
    .bind(STARTED_AT - 10_000)
    .execute(&mut *write)
    .await
    .expect("the row inserts");
    write.commit().await.expect("the row commits");

    let telegram = telegram_session(&app).await;
    let listed = call(&app, "GET", "/api/identities", Some(&telegram), "").await;
    assert_eq!(listed.status, StatusCode::OK, "{}", listed.body);
    let methods: Value = serde_json::from_str(&listed.body).expect("a JSON body");
    assert_eq!(
        methods,
        json!([
            {"id": 0, "kind": "telegram", "created_at": null, "last_used_at": null},
            {"id": 1, "kind": "passkey", "created_at": STARTED_AT - 10_000, "last_used_at": null},
        ]),
        "the methods list is not Telegram then the passkey"
    );
    let body = listed.body.to_ascii_uppercase();
    for secret in [CREDENTIAL_HEX, HANDLE_HEX] {
        assert!(!body.contains(secret), "the list shows a blob as hex");
    }
    for secret in [CREDENTIAL_B64, HANDLE_B64] {
        assert!(
            !listed.body.contains(secret),
            "the list shows a blob as base64url"
        );
    }
}

/// The API as [`app`] builds it, on `clock`, which the test keeps so it can move the minute.
async fn app_on(clock: Arc<ManualClock>) -> App {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&dir.path().join("deck-streak.db"))
        .await
        .expect("the database opens and migrates");
    let readiness = Readiness::new();
    readiness.database_opened(db.clone());
    let owner = Owner::new(TelegramUserId::new(OWNER));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        owner,
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    let config = LinkingConfig::from_setting(Some(ORIGIN)).expect("the origin is an https origin");
    let router = router(
        ApiState::new(readiness)
            .with_owner(access)
            .with_linking(config, owner),
    );
    App {
        router,
        db,
        _dir: dir,
    }
}

/// The ceremony bound counts in the kernel clock's minute: the thirty-first start is refused until
/// the minute turns, and served once it has (SPEC-359 R10).
#[tokio::test]
async fn the_ceremony_bound_serves_again_when_the_minute_turns() {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let App {
        router: app, _dir, ..
    } = app_on(Arc::clone(&clock)).await;
    let path = "/api/passkeys/sign-in/start";
    let served = (
        StatusCode::UNAUTHORIZED,
        r#"{"reason":"not_linked"}"#.to_owned(),
    );
    let throttled = (
        StatusCode::TOO_MANY_REQUESTS,
        r#"{"reason":"too_many_ceremonies"}"#.to_owned(),
    );
    let starts: Vec<u32> = (1..=30).collect();
    for attempt in examined("ceremony starts", starts) {
        let answer = call(&app, "POST", path, None, "{}").await;
        assert_eq!((answer.status, answer.body), served, "start {attempt}");
    }
    let refused = call(&app, "POST", path, None, "{}").await;
    assert_eq!(
        (refused.status, refused.body),
        throttled,
        "the thirty-first start"
    );
    // 03:30:10 plus 49 seconds is still the same minute; one more second turns it.
    clock.advance(Duration::from_secs(49));
    let still = call(&app, "POST", path, None, "{}").await;
    assert_eq!(
        (still.status, still.body),
        throttled,
        "a start before the minute turned"
    );
    clock.advance(Duration::from_secs(1));
    let turned = call(&app, "POST", path, None, "{}").await;
    assert_eq!(
        (turned.status, turned.body),
        served,
        "a start once the minute turned"
    );
}

/// With no passkey held, a sign-in start answers 401 `not_linked`; a sign-in finish with no
/// ceremony cookie answers 401 `challenge_invalid` and clears the cookie (SPEC-359 R6, R8).
#[tokio::test]
async fn a_sign_in_answers_its_refusals_over_the_routes() {
    let App {
        router: app, _dir, ..
    } = app().await;
    let started = call(&app, "POST", "/api/passkeys/sign-in/start", None, "{}").await;
    assert_eq!(
        (started.status, started.body.as_str()),
        (StatusCode::UNAUTHORIZED, r#"{"reason":"not_linked"}"#),
        "a sign-in start with no passkey held"
    );
    let finished = call(&app, "POST", "/api/passkeys/sign-in/finish", None, "{}").await;
    assert_eq!(
        (finished.status, finished.body.as_str()),
        (
            StatusCode::UNAUTHORIZED,
            r#"{"reason":"challenge_invalid"}"#
        ),
        "a sign-in finish with no ceremony cookie"
    );
    let cleared: Vec<String> = set_cookies(&finished)
        .into_iter()
        .map(|(pair, _)| pair)
        .collect();
    assert_eq!(
        cleared,
        ["__Host-deckstreak_ceremony=".to_owned()],
        "the refused finish did not clear the ceremony cookie"
    );
}

/// A registration finish takes its flow id from the one ceremony cookie: with it, the finish
/// reaches its ceremony and refuses the empty response `passkey_invalid`; without it, the finish
/// answers `challenge_invalid` (SPEC-359 R5, R8).
#[tokio::test]
async fn a_registration_finish_takes_its_flow_from_the_ceremony_cookie() {
    let App {
        router: app, _dir, ..
    } = app().await;
    let telegram = telegram_session(&app).await;
    let link = link_session(&app, &telegram).await;
    let start = "/api/passkeys/register/start";
    let finish = "/api/passkeys/register/finish";

    let started = call(&app, "POST", start, Some(&link), "{}").await;
    assert_eq!(started.status, StatusCode::OK, "{}", started.body);
    let (pair, _) = set_cookies(&started)
        .into_iter()
        .next()
        .expect("the start set the ceremony cookie");
    let ceremony = format!("{link}; {pair}");
    let reached = call(&app, "POST", finish, Some(&ceremony), "{}").await;
    assert_eq!(
        (reached.status, reached.body.as_str()),
        (StatusCode::UNAUTHORIZED, r#"{"reason":"passkey_invalid"}"#),
        "the finish did not reach the ceremony its cookie names"
    );

    let restarted = call(&app, "POST", start, Some(&link), "{}").await;
    assert_eq!(restarted.status, StatusCode::OK, "{}", restarted.body);
    let cookieless = call(&app, "POST", finish, Some(&link), "{}").await;
    assert_eq!(
        (cookieless.status, cookieless.body.as_str()),
        (
            StatusCode::UNAUTHORIZED,
            r#"{"reason":"challenge_invalid"}"#
        ),
        "a finish with no ceremony cookie"
    );
}
