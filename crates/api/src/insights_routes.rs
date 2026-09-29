//! The owner's instruments routes (SPEC-094 R10, R12; ADR-094, ADR-085).

use std::sync::Arc;

use axum::Router;
use deck_streak_coordination::instruments::InstrumentService;

use crate::session_routes::OwnerAccess;

/// The instruments routes over `access` and `service`.
pub(crate) fn routes(_access: OwnerAccess, _service: Arc<dyn InstrumentService>) -> Router {
    Router::new()
}
