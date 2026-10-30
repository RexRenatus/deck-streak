//! The routes of the decks kept away from AI over the shell's layers (SPEC-381 A1, A10; R7): a
//! fresh server marks no deck, a mark and an unmark round-trip, an id that is not a positive
//! decimal integer is refused, and a cross-site or signed-out change marks nothing.
//!
//! The launch payload is SPEC-024's, as in the quick capture route's test. Every id is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Db, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use serde_json::{Value, json};
use tempfile::TempDir;
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
/// Ten seconds after the payload was signed, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 256 * 1024;
/// The marked decks' path.
const MARKED: &str = "/api/decks/sensitive";

/// An answer: its status and its body.
struct Answer {
    status: StatusCode,
    body: String,
}

impl Answer {
    /// The body as JSON, or `null` when it is not JSON: an answer with no JSON body then fails the
    /// assertion that compares it, beside its status, rather than this helper.
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

/// `method` on `path` with `headers` and `body`, through the whole app.
async fn send(
    app: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: String,
) -> (Answer, Option<String>) {
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
    let cookie = response
        .headers()
        .get(SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_owned());
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    (
        Answer {
            status,
            body: String::from_utf8(bytes.to_vec()).expect("a UTF-8 body"),
        },
        cookie,
    )
}

/// The owner's session cookie, from a same-origin JSON handshake: its `name=value`.
async fn owner_cookie(app: &Router) -> String {
    let (answer, cookie) = send(
        app,
        "POST",
        "/api/session",
        &[
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
        ],
        json!({ "init_data": OWNER_PAYLOAD }).to_string(),
    )
    .await;
    assert_eq!(answer.status, StatusCode::OK, "the owner's handshake");
    cookie.expect("a Set-Cookie")
}

/// `GET /api/decks/sensitive` carrying `cookie`.
async fn marked(app: &Router, cookie: &str) -> Answer {
    send(app, "GET", MARKED, &[("cookie", cookie)], String::new())
        .await
        .0
}

/// `PUT /api/decks/{id}/sensitive` with `sensitive`, from `site`, carrying `cookie` when given.
async fn set(app: &Router, id: &str, sensitive: bool, site: &str, cookie: Option<&str>) -> Answer {
    let mut headers = vec![
        ("content-type", "application/json"),
        ("sec-fetch-site", site),
    ];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(
        app,
        "PUT",
        &format!("/api/decks/{id}/sensitive"),
        &headers,
        json!({ "sensitive": sensitive }).to_string(),
    )
    .await
    .0
}

/// The API over a temporary database.
async fn app(scratch: &TempDir) -> Router {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    let readiness = Readiness::new();
    readiness.database_opened(db);
    router(ApiState::new(readiness).with_owner(access))
}

#[tokio::test]
async fn no_deck_is_kept_away_until_the_learner_marks_one() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let app = app(&scratch).await;
    let cookie = owner_cookie(&app).await;
    let fresh = marked(&app, &cookie).await;
    assert_eq!(
        (fresh.status, fresh.json()),
        (StatusCode::OK, json!({ "decks": [] })),
        "a fresh server marks no deck"
    );
    let mut answers = Vec::new();
    for (id, sensitive) in [("12", true), ("7", true), ("7", true), ("12", false)] {
        let answer = set(&app, id, sensitive, "same-origin", Some(&cookie)).await;
        answers.push((answer.status, answer.json()));
    }
    assert_eq!(
        answers,
        [
            (StatusCode::OK, json!({ "decks": ["12"] })),
            (StatusCode::OK, json!({ "decks": ["7", "12"] })),
            (StatusCode::OK, json!({ "decks": ["7", "12"] })),
            (StatusCode::OK, json!({ "decks": ["7"] })),
        ],
        "each change answers the new set, ascending"
    );
    let after = marked(&app, &cookie).await;
    assert_eq!(
        (after.status, after.json()),
        (StatusCode::OK, json!({ "decks": ["7"] }))
    );
    for refused in ["0", "-3", "%2B5", "abc", "1.5", "99999999999999999999"] {
        let answer = set(&app, refused, true, "same-origin", Some(&cookie)).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST, "{refused}");
    }
    let unchanged = marked(&app, &cookie).await;
    assert_eq!(
        unchanged.json(),
        json!({ "decks": ["7"] }),
        "a refused id marks nothing"
    );
}

#[tokio::test]
async fn a_cross_site_or_signed_out_change_marks_nothing() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let app = app(&scratch).await;
    let cookie = owner_cookie(&app).await;
    let cross_site = set(&app, "7", true, "cross-site", Some(&cookie)).await;
    let signed_out = set(&app, "7", true, "same-origin", None).await;
    assert_eq!(
        (cross_site.status, signed_out.status),
        (StatusCode::FORBIDDEN, StatusCode::UNAUTHORIZED)
    );
    let read = marked(&app, &cookie).await;
    assert_eq!(
        (read.status, read.json()),
        (StatusCode::OK, json!({ "decks": [] })),
        "neither change marked the deck"
    );
    let unauthenticated = send(&app, "GET", MARKED, &[], String::new()).await.0;
    assert_eq!(unauthenticated.status, StatusCode::UNAUTHORIZED);
    // The control: the same change, same-origin and signed in, marks it.
    let owner = set(&app, "7", true, "same-origin", Some(&cookie)).await;
    assert_eq!(
        (owner.status, owner.json()),
        (StatusCode::OK, json!({ "decks": ["7"] }))
    );
}
