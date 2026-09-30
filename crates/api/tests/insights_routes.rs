//! The instruments routes (SPEC-094 A18; R10, R12): the owner reads the list and a stored report
//! and asks for a run, and every other caller is refused with no data.
//!
//! The launch payloads are the session routes' own, signed for a synthetic bot token. The service
//! behind the routes is a fake over synthetic reports.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_coordination::instruments::{
    BoxFuture, InstrumentListing, InstrumentService, OnDemandRefusal, StoredReport,
};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{KernelError, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use serde_json::{Value, json};
use tower::ServiceExt;

const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
const OWNER: i64 = 4242;
const OWNER_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=79502d49032542c5030e80b666d03f5af53e053f97d387806adb0aa843ddeb2d",
);
const STRANGER_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A777%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=2c30fb14ffed3eb8ddf5d9836d760774b91a30db863140cb57e05ea512f308dd",
);
const STARTED_AT: i64 = 1_736_911_810_000;
const BODY_READ_LIMIT: usize = 64 * 1024;
/// A marker only the fake's report carries, so a leak is found by text.
const MARKER: &str = "synthetic-marker-value";

/// A fake service: one instrument with a stored report, one with none, and a run that answers the
/// stored report or says a run is in progress.
struct Fake {
    busy: bool,
}

fn stored() -> StoredReport {
    StoredReport {
        instrument: "alpha".to_owned(),
        study_day: 20_000,
        schema_version: 1,
        report: json!({ "instrument": "alpha", "view": { "marker": MARKER } }),
        created_at: 1,
    }
}

impl InstrumentService for Fake {
    fn list(&self) -> BoxFuture<'_, Result<Vec<InstrumentListing>, KernelError>> {
        Box::pin(async {
            Ok(vec![
                InstrumentListing {
                    id: "alpha".to_owned(),
                    cadence: "weekly",
                    study_day: Some(20_000),
                },
                InstrumentListing {
                    id: "beta".to_owned(),
                    cadence: "on_demand",
                    study_day: None,
                },
            ])
        })
    }
    fn report<'a>(
        &'a self,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<StoredReport>, KernelError>> {
        Box::pin(async move { Ok((id == "alpha").then(stored)) })
    }
    fn run<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<StoredReport, OnDemandRefusal>> {
        Box::pin(async move {
            if id != "alpha" && id != "beta" {
                Err(OnDemandRefusal::Unknown)
            } else if self.busy {
                Err(OnDemandRefusal::InProgress)
            } else {
                Ok(stored())
            }
        })
    }
}

struct Answer {
    status: StatusCode,
    cookie: Option<String>,
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
        .expect("infallible");
    let status = response.status();
    let cookie = response
        .headers()
        .get(SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::to_owned);
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a body");
    Answer {
        status,
        cookie,
        body: String::from_utf8(bytes.to_vec()).expect("text"),
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

fn app(service: impl InstrumentService + 'static) -> Router {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    router(
        ApiState::new(Readiness::new())
            .with_owner(access)
            .with_instruments(Arc::new(service)),
    )
}

async fn get(app: &Router, path: &str, cookie: Option<&str>) -> Answer {
    let headers: Vec<(&str, &str)> = cookie.map(|c| ("cookie", c)).into_iter().collect();
    send(app, "GET", path, &headers, String::new()).await
}

async fn run(app: &Router, path: &str, cookie: Option<&str>) -> Answer {
    let mut headers = vec![("sec-fetch-site", "same-origin")];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(app, "POST", path, &headers, String::new()).await
}

const FORGED: &str =
    "__Host-deckstreak_session=0000000000000000000000000000000000000000000000000000000000000000";

#[tokio::test]
async fn the_insights_routes_answer_only_the_owner() {
    let app = app(Fake { busy: false });
    let stranger = handshake(&app, STRANGER_PAYLOAD).await;
    assert_eq!(stranger.cookie, None);

    // Every route, refused for an anonymous caller, a stranger and a forged cookie.
    let mut refused = Vec::new();
    for cookie in [None, Some(FORGED)] {
        refused.push(get(&app, "/api/insights", cookie).await);
        refused.push(get(&app, "/api/insights/alpha", cookie).await);
        refused.push(run(&app, "/api/insights/alpha/run", cookie).await);
    }
    println!("examined {} refused request(s)", refused.len());
    assert!(
        refused
            .iter()
            .all(|answer| answer.status == StatusCode::UNAUTHORIZED),
        "every route refuses a caller with no session"
    );
    assert!(
        refused
            .iter()
            .all(|answer| !answer.body.contains(MARKER) && !answer.body.contains("alpha")),
        "no refused caller is served a byte of a report"
    );

    let cookie = handshake(&app, OWNER_PAYLOAD)
        .await
        .cookie
        .expect("the owner's session");
    let list = get(&app, "/api/insights", Some(&cookie)).await;
    assert_eq!(list.status, StatusCode::OK);
    let list: Value = serde_json::from_str(&list.body).expect("JSON");
    assert_eq!(
        list["instruments"],
        json!([
            { "id": "alpha", "cadence": "weekly", "study_day": 20_000 },
            { "id": "beta", "cadence": "on_demand", "study_day": null },
        ]),
        "a missing report reads as pending, never as a zero"
    );

    let one = get(&app, "/api/insights/alpha", Some(&cookie)).await;
    assert_eq!(one.status, StatusCode::OK);
    assert!(
        one.body.contains(MARKER),
        "the owner reads the stored report"
    );
    let pending = get(&app, "/api/insights/beta", Some(&cookie)).await;
    assert_eq!(pending.status, StatusCode::OK);
    let pending: Value = serde_json::from_str(&pending.body).expect("JSON");
    assert_eq!(pending["report"], Value::Null, "no report yet is null");
    let unknown = get(&app, "/api/insights/nope", Some(&cookie)).await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);

    let ran = run(&app, "/api/insights/alpha/run", Some(&cookie)).await;
    assert_eq!(ran.status, StatusCode::OK);
    assert!(ran.body.contains(MARKER), "a run answers the new report");
    let none = run(&app, "/api/insights/nope/run", Some(&cookie)).await;
    assert_eq!(none.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_run_in_progress_is_answered_and_starts_nothing() {
    let app = app(Fake { busy: true });
    let cookie = handshake(&app, OWNER_PAYLOAD)
        .await
        .cookie
        .expect("the owner's session");
    let answer = run(&app, "/api/insights/alpha/run", Some(&cookie)).await;
    assert_eq!(answer.status, StatusCode::CONFLICT);
    let body: Value = serde_json::from_str(&answer.body).expect("JSON");
    assert_eq!(body["reason"], "run_in_progress");
    assert!(!answer.body.contains(MARKER));
}

#[tokio::test]
async fn a_cross_site_run_is_refused() {
    let app = app(Fake { busy: false });
    let cookie = handshake(&app, OWNER_PAYLOAD)
        .await
        .cookie
        .expect("the owner's session");
    let answer = send(
        &app,
        "POST",
        "/api/insights/alpha/run",
        &[("sec-fetch-site", "cross-site"), ("cookie", &cookie)],
        String::new(),
    )
    .await;
    assert_eq!(answer.status, StatusCode::FORBIDDEN);
    assert!(!answer.body.contains(MARKER));
}

/// A service whose store cannot be read.
struct Broken;

impl InstrumentService for Broken {
    fn list(&self) -> BoxFuture<'_, Result<Vec<InstrumentListing>, KernelError>> {
        Box::pin(async { Err(KernelError::Offload { operation: "list" }) })
    }
    fn report<'a>(
        &'a self,
        _id: &'a str,
    ) -> BoxFuture<'a, Result<Option<StoredReport>, KernelError>> {
        Box::pin(async {
            Err(KernelError::Offload {
                operation: "report",
            })
        })
    }
    fn run<'a>(&'a self, _id: &'a str) -> BoxFuture<'a, Result<StoredReport, OnDemandRefusal>> {
        Box::pin(async {
            Err(OnDemandRefusal::Store(KernelError::Offload {
                operation: "run",
            }))
        })
    }
}

#[tokio::test]
async fn an_unreadable_store_answers_500_with_a_reason_code_alone() {
    let app = app(Broken);
    let cookie = handshake(&app, OWNER_PAYLOAD)
        .await
        .cookie
        .expect("the owner's session");
    for path in ["/api/insights", "/api/insights/alpha"] {
        let answer = get(&app, path, Some(&cookie)).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR, "{path}");
        assert_eq!(
            answer.body, r#"{"reason":"instruments_unreadable"}"#,
            "{path}"
        );
    }
    let started = run(&app, "/api/insights/alpha/run", Some(&cookie)).await;
    assert_eq!(started.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(started.body, r#"{"reason":"run_unavailable"}"#);
}

#[test]
fn the_state_debug_line_says_which_ports_it_holds() {
    let bare = format!("{:?}", ApiState::new(Readiness::new()));
    assert!(bare.starts_with("ApiState { readiness: "), "{bare}");
    assert!(
        bare.ends_with(", owner: None, instruments: false, law_tiers: false, drills: false }"),
        "{bare}"
    );
    let served = format!(
        "{:?}",
        ApiState::new(Readiness::new()).with_instruments(Arc::new(Broken))
    );
    assert!(
        served.ends_with(", owner: None, instruments: true, law_tiers: false, drills: false }"),
        "{served}"
    );
}
