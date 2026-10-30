//! The sync service's statement of the oldest client level it accepts (SPEC-374 R2; ADR-385).
//!
//! A client reads it before every sync login and stops before any sync request when its own level
//! is below it, so the route answers any caller, with or without an owner configured, beside the
//! health routes. The answer is never kept by a cache, so a raised minimum reaches the next read.

use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::router::ApiState;

/// The oldest client level the service accepts: one positive integer, defined once, never above
/// the level the same tree's clients build (R3).
pub const MINIMUM_CLIENT_LEVEL: u32 = 1;

/// The statement's path, under the edge's existing `/api/*` proxy.
pub const MINIMUM_CLIENT: &str = "/api/sync/minimum-client";

/// The statement's body: `{"minimum_client_level":<n>}`, and nothing else.
#[derive(Serialize)]
struct Statement {
    minimum_client_level: u32,
}

/// The statement's route, on axum 0.8's path syntax.
pub(crate) fn routes() -> Router<ApiState> {
    Router::new().route(MINIMUM_CLIENT, get(minimum_client))
}

/// Answers the minimum client level as JSON, marked never to be stored by a cache.
async fn minimum_client() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(Statement {
            minimum_client_level: MINIMUM_CLIENT_LEVEL,
        }),
    )
}
