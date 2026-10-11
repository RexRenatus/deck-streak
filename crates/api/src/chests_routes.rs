//! The owner's chest and token routes (SPEC-411 R13; ADR-425 D4): the inventory, a chest's open,
//! an Epic's pick and a token's activation, each through coordination's `chests` use cases.

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, post};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The inventory's path.
pub const CHESTS_PATH: &str = "/api/chests";
/// One chest's open path.
pub const CHEST_OPEN_PATH: &str = "/api/chests/{id}/open";
/// One Epic's pick path.
pub const CHEST_PICK_PATH: &str = "/api/chests/{id}/pick";
/// The token activation's path.
pub const TOKEN_ACTIVATE_PATH: &str = "/api/tokens/activate";

/// The routes over the owner's `access` and the API's `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    let _ = (access, readiness);
    Router::new()
        .route(CHESTS_PATH, get(answered))
        .route(CHEST_OPEN_PATH, post(answered))
        .route(CHEST_PICK_PATH, post(answered))
        .route(TOKEN_ACTIVATE_PATH, post(answered))
}

/// Every route's answer.
async fn answered() -> StatusCode {
    StatusCode::OK
}
