//! The owner's Road to C2 route (SPEC-077 R15), behind the owner's session (SPEC-024).
//!
//! `GET /api/progress` answers the stored progress of each configured course, ordered by name:
//! its code, name and flag, its mastery, its current band and unit, and each band's cards, mature
//! cards, percentage and whether it is achieved. Before the first recompute stores a course it
//! answers an empty list. It reads coordination's progress view, the one the bot's progress command
//! reads too.

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_coordination::progress_view::{StoredProgress, progress_view};
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::{Courses, KernelError};
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// Road to C2's progress.
pub const PROGRESS_PATH: &str = "/api/progress";

/// What the progress route reads with.
#[derive(Clone)]
struct Progress {
    access: OwnerAccess,
    readiness: Readiness,
    courses: Courses,
}

impl FromRef<Progress> for Sessions {
    fn from_ref(progress: &Progress) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&progress.access)
    }
}

/// The route over `access` and `readiness`, for the configured `courses`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness, courses: Courses) -> Router {
    Router::new()
        .route(PROGRESS_PATH, get(progress))
        .with_state(Progress {
            access,
            readiness,
            courses,
        })
}

/// `GET /api/progress`.
async fn progress(_owner: OwnerSession, State(progress): State<Progress>) -> Response {
    let Some(db) = progress.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    match progress_view(db, &progress.courses).await {
        Ok(view) => {
            let courses: Vec<Value> = view.iter().map(course).collect();
            answer(&json!({ "courses": courses }))
        }
        Err(error) => unreadable(&error, "progress_unreadable"),
    }
}

/// One course's stored progress as the route answers it.
fn course(stored: &StoredProgress) -> Value {
    let bands: Vec<Value> = stored
        .bands
        .iter()
        .map(|band| {
            json!({
                "band": band.band,
                "total": band.total,
                "mature": band.mature,
                "pct": band.pct,
                "achieved": band.achieved,
            })
        })
        .collect();
    json!({
        "code": stored.course,
        "name": stored.name,
        "flag": stored.flag,
        "mastery_pct": stored.mastery_pct,
        "current_band": stored.current_band,
        "current_unit": stored.current_unit,
        "bands": bands,
    })
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
    tracing::warn!(reason, "a progress read was refused");
    let body = json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A read that failed: 500 with its reason code, and the error only in the log.
fn unreadable(error: &KernelError, reason: &'static str) -> Response {
    tracing::error!(%error, reason, "a progress read failed");
    refused(StatusCode::INTERNAL_SERVER_ERROR, reason)
}
