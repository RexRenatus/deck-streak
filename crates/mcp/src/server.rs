//! The MCP server: its one path and its serving stack (SPEC-119 R2 to R4; ADR-329).

use std::sync::Arc;

use axum::Router;

use crate::guard::Guard;
use crate::tools::LawTrackSource;

/// The router the role serves, over the law track's `law` and the guard `guard`.
#[must_use]
pub fn router(law: Arc<dyn LawTrackSource>, guard: Arc<Guard>) -> Router {
    drop((law, guard));
    Router::new()
}
