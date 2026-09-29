//! The ladder's in-app feed (SPEC-084 A15; R10): each item the router delivers to the Mini App
//! carries the tier it rendered at, which the Mini App animates, and the feed serves it to the
//! owner's live session only.
//!
//! The launch payloads are the session routes' own: signed by Python's standard `hmac` and `hashlib`
//! for a synthetic bot token, for synthetic user ids, all dated 2025-01-15T03:30:00Z, with the API's
//! clock ten seconds later. The router that puts an item in the feed runs on a clock of its own, at
//! noon, outside the quiet window. Every value is synthetic.

// An integration test is test code: its fixtures panic on a failed setup, and the examined count is
// printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{Request, StatusCode};
use deck_streak_api::notifications_routes::FEED_PATH;
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, TelegramUserId, UtcMillis};
use deck_streak_notifications::{
    Decision, DedupeKey, LapseContext, Occasion, Policy, Router as NotificationRouter, Surface,
    Tier,
};
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
/// Ten seconds after the payloads were signed, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// Noon of a study day, where the router runs.
const NOON: i64 = 20_000 * 86_400_000 + 12 * 3_600_000;
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 64 * 1024;

/// An answer: its status, its media type, the `Set-Cookie` it carries, and its body.
struct Answer {
    status: StatusCode,
    media: Option<String>,
    cookie: Option<String>,
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
    let media = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let cookie = response
        .headers()
        .get(SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::to_owned);
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    Answer {
        status,
        media,
        cookie,
        body: String::from_utf8(bytes.to_vec()).expect("a text body"),
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

/// The feed, carrying `cookie`, or no cookie.
async fn feed(app: &Router, cookie: Option<&str>) -> Answer {
    let headers: Vec<(&str, &str)> = cookie
        .map(|cookie| ("cookie", cookie))
        .into_iter()
        .collect();
    send(app, "GET", FEED_PATH, &headers, String::new()).await
}

/// The Mini App celebrations the router delivers, each an event of its own tier: its key, its event
/// and the tier it renders at.
const DELIVERED: [(&str, &str, &str); 4] = [
    ("badge:ladder-feed", "badge", "T2"),
    ("record:ladder-feed", "record", "T3"),
    ("queue:ladder-feed", "queue_zero", "T4"),
    ("ceremony:ladder-feed", "ceremony", "T5"),
];

/// The API as the daemon builds it, over a database the router has delivered [`DELIVERED`] into.
async fn app() -> (TempDir, Router) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    let notifications = NotificationRouter::new(
        Arc::clone(&policy),
        db.clone(),
        Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(NOON))),
        StudyDayRule::default(),
    );
    for (key, event, _) in DELIVERED {
        let celebration = Occasion::new(
            policy.kind("celebration").expect("the celebration kind"),
            DedupeKey::new(key).expect("a key"),
            Surface::MiniApp,
            Tier::T2,
            format!("A synthetic {event}"),
            StudyDay::from_epoch_day(20_000),
            LapseContext::NoLapse,
        )
        .expect("an occasion")
        .with_event(event, None);
        let decision = notifications.route(&celebration).await.expect("a decision");
        assert!(
            matches!(
                decision,
                Decision::Sent {
                    surface: Surface::MiniApp,
                    ..
                }
            ),
            "{key} is delivered to the feed: {decision:?}"
        );
    }

    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let readiness = Readiness::new();
    readiness.database_opened(db);
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    (
        directory,
        router(ApiState::new(readiness).with_owner(access)),
    )
}

#[tokio::test]
async fn the_feed_item_carries_its_tier_and_answers_only_the_owner() {
    let (_directory, app) = app().await;

    // Any other caller: no session, a stranger's refused handshake, and a cookie of no session.
    let anonymous = feed(&app, None).await;
    let stranger = handshake(&app, STRANGER_PAYLOAD).await;
    let forged = feed(
        &app,
        Some("__Host-deckstreak_session=0000000000000000000000000000000000000000000000000000000000000000"),
    )
    .await;
    let signed_in = handshake(&app, OWNER_PAYLOAD).await;
    let cookie = signed_in.cookie.expect("the owner's session cookie");
    let served = feed(&app, Some(&cookie)).await;

    let refused = [&anonymous, &stranger, &forged];
    println!("examined {} refused caller(s)", refused.len());
    assert_eq!(
        refused.map(|answer| answer.status),
        [
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::UNAUTHORIZED
        ]
    );
    assert!(
        refused
            .iter()
            .all(|answer| !answer.body.contains("synthetic")
                && !answer.body.contains("tier")
                && !answer.body.contains("items")),
        "no refused caller is served an item or its tier"
    );
    assert_eq!(served.status, StatusCode::OK);
    assert_eq!(served.media.as_deref(), Some("application/json"));
    let items: Value = serde_json::from_str(&served.body).expect("a JSON body");
    let items = items["items"].as_array().expect("the items");
    println!("examined {} item(s)", items.len());
    assert_eq!(
        items
            .iter()
            .map(|item| (item["text"].clone(), item["tier"].clone()))
            .collect::<Vec<_>>(),
        DELIVERED
            .map(|(_, event, tier)| (json!(format!("A synthetic {event}")), json!(tier)))
            .to_vec(),
        "each item carries the tier it rendered at"
    );
}
