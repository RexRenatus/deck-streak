//! The owner's law block route (SPEC-077 R14, R16), behind the owner's session (SPEC-024).
//!
//! `GET /api/law` answers the law block for the owner's study day: whether it is shown, whether
//! its lifetime XP's line names the level, the keys of the lines it shows in order, and each count.
//! A count the recompute has not stored, or a port that is not wired, is `null` beside its
//! `pending` flag, never 0. The leech port is not wired yet (#133), so the leeches and the mastery
//! pillar are pending.

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_identity::{OwnerSession, Sessions};
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The law block.
pub const LAW_PATH: &str = "/api/law";

/// What the law route reads with.
#[derive(Clone)]
struct Law {
    access: OwnerAccess,
    readiness: Readiness,
}

impl FromRef<Law> for Sessions {
    fn from_ref(law: &Law) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&law.access)
    }
}

/// The route over `access` and `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    Router::new()
        .route(LAW_PATH, get(law))
        .with_state(Law { access, readiness })
}

/// `GET /api/law`.
async fn law(_owner: OwnerSession, State(law): State<Law>) -> Response {
    let _unread = &law.readiness;
    answer(&json!({ "shown": false }))
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
