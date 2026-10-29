//! The owner's export, for the study clients (SPEC-516 R2, R3; ADR-534 D1, D2).
//!
//! `POST /api/data/export` answers the bytes `deckstreakd data export` writes over the same
//! database. This module states the route's path and bounds; the route itself is not served yet.

use axum::Router;

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The export's path.
pub const EXPORT_PATH: &str = "/api/data/export";
/// Exports admitted in one minute of the kernel's clock, per process, in a window of the route's
/// own.
pub const EXPORTS_PER_MINUTE: u32 = 6;
/// The largest export body read: the request carries `{}` or nothing.
pub const EXPORT_BODY_LIMIT_BYTES: usize = 64;

/// The export route over `access` and `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    let _ = (access, readiness);
    Router::new()
}
