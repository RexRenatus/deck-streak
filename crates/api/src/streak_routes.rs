//! The owner's streak and governor routes (SPEC-076 R20, R21), each behind the owner's session
//! (SPEC-024): the views the bot's `/streak` and the Mini App's streak screen read.

use axum::Router;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_identity::OwnerSession;

use crate::session_routes::OwnerAccess;

/// The streak view.
pub const STREAK_PATH: &str = "/api/streak";
/// The governor view.
pub const GOVERNOR_PATH: &str = "/api/governor";

/// The streak routes over `access`.
pub(crate) fn routes(access: OwnerAccess) -> Router {
    Router::new()
        .route(STREAK_PATH, get(streak))
        .route(GOVERNOR_PATH, get(governor))
        .with_state(access)
}

/// `GET /api/streak`.
async fn streak(_owner: OwnerSession) -> Response {
    empty()
}

/// `GET /api/governor`.
async fn governor(_owner: OwnerSession) -> Response {
    empty()
}

/// 200 with an empty JSON object: the views are not built yet.
fn empty() -> Response {
    (StatusCode::OK, [(CONTENT_TYPE, "application/json")], "{}").into_response()
}
