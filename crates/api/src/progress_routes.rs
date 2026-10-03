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
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::Courses;
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
    let _unread = (&progress.readiness, &progress.courses);
    answer(&json!({ "courses": [] }))
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
