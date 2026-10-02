//! The owner's badge, record and milestone routes (SPEC-073 R16, R17), each behind the owner's
//! session (SPEC-024).
//!
//! - `GET /api/badges` answers the earned badges, most recently awarded first, each its key,
//!   tier, name, emoji and day; and the locked catalog badges, each its criteria and, where its
//!   input is stored, that input's value against its threshold.
//! - `GET /api/records` answers each stored record with its value, day and the value it beat,
//!   today's live value and its distance from the record, and the record to chase.
//! - `GET /api/milestone` answers `pending`: Road to C2 supplies no mature-card sum yet (R15, #85),
//!   so nothing is read and nothing is computed from a stand-in.
//!
//! The badges and records read coordination's views, the ones the bot's `/badges` and `/records`
//! read too.

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_coordination::progression::badges_view::badges_view;
use deck_streak_coordination::progression::records_view::records_now;
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::{Courses, KernelError};
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The badges view.
pub const BADGES_PATH: &str = "/api/badges";
/// The records view.
pub const RECORDS_PATH: &str = "/api/records";
/// The next milestone.
pub const MILESTONE_PATH: &str = "/api/milestone";

/// What the badge, record and milestone routes share.
#[derive(Clone)]
struct Badges {
    access: OwnerAccess,
    readiness: Readiness,
    courses: Courses,
}

impl FromRef<Badges> for Sessions {
    fn from_ref(badges: &Badges) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&badges.access)
    }
}

/// The routes over `access` and `readiness`, the catalog's descriptions rendered from `courses`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness, courses: Courses) -> Router {
    Router::new()
        .route(BADGES_PATH, get(badges))
        .route(RECORDS_PATH, get(records))
        .route(MILESTONE_PATH, get(milestone))
        .with_state(Badges {
            access,
            readiness,
            courses,
        })
}

/// `GET /api/badges`.
async fn badges(_owner: OwnerSession, State(badges): State<Badges>) -> Response {
    let Some(db) = badges.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    match badges_view(db, badges.access.study_day(), &badges.courses).await {
        Ok(view) => {
            let earned: Vec<Value> = view
                .earned
                .iter()
                .map(|line| {
                    json!({
                        "key": line.key,
                        "tier": line.tier,
                        "name": line.name,
                        "emoji": line.emoji,
                        "study_day": line.study_day.to_string(),
                    })
                })
                .collect();
            let locked: Vec<Value> = view
                .locked
                .iter()
                .map(|line| {
                    json!({
                        "key": line.key,
                        "name": line.name,
                        "emoji": line.emoji,
                        "criteria": line.criteria,
                        "family": line.family,
                        "progress": line.progress.map(|progress| json!({
                            "value": progress.value,
                            "threshold": progress.threshold,
                        })),
                    })
                })
                .collect();
            answer(&json!({ "earned": earned, "locked": locked }))
        }
        Err(error) => unreadable(&error, "badges_unreadable"),
    }
}

/// `GET /api/records`.
async fn records(_owner: OwnerSession, State(badges): State<Badges>) -> Response {
    let Some(db) = badges.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    match records_now(db, badges.access.study_day()).await {
        Ok(view) => {
            let records: Vec<Value> = view
                .lines
                .iter()
                .map(|line| {
                    json!({
                        "kind": line.kind.as_str(),
                        "label": line.label,
                        "value": line.value,
                        "study_day": line.study_day.to_string(),
                        "previous": line.previous,
                        "today": line.today,
                        "distance": distance(line.value, line.today),
                    })
                })
                .collect();
            let chase = view.chase.map(
                |(kind, gap)| json!({ "kind": kind.as_str(), "label": kind.label(), "gap": gap }),
            );
            answer(&json!({ "records": records, "chase": chase }))
        }
        Err(error) => unreadable(&error, "records_unreadable"),
    }
}

/// `GET /api/milestone`: `pending`, read from nothing, until Road to C2 supplies the mature-card
/// sum the next milestone is computed from (R15, #85).
async fn milestone(_owner: OwnerSession) -> Response {
    answer(&json!({ "status": "pending" }))
}

/// How far `today` is from a record of `value`: none once today reaches it.
fn distance(value: i64, today: i64) -> i64 {
    value.saturating_sub(today).max(0)
}

/// 200 with `body` as JSON.
fn answer(body: &Value) -> Response {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// A refusal: its status, and a JSON body naming its reason code alone.
fn refused(status: StatusCode, reason: &'static str) -> Response {
    tracing::warn!(reason, "a badge or record read was refused");
    let body = json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A read that failed: 500 with its reason code, and the error only in the log.
fn unreadable(error: &KernelError, reason: &'static str) -> Response {
    tracing::error!(%error, reason, "a badge or record read failed");
    refused(StatusCode::INTERNAL_SERVER_ERROR, reason)
}
