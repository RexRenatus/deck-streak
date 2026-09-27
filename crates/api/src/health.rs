//! Liveness and readiness (SPEC-025 R2; ADR-025).
//!
//! STUB for the red-first commit: the public surface is in place and no route is served.

use axum::Router;
use deck_streak_kernel::Db;

use crate::router::ApiState;

/// Liveness: 200 while the process runs.
pub const LIVEZ: &str = "/api/livez";
/// Readiness: 503 until the database is open and migrated, 200 after.
pub const READYZ: &str = "/api/readyz";
/// The release version both health bodies carry.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Whether the API can answer from its database.
#[derive(Clone, Debug, Default)]
pub struct Readiness {}

impl Readiness {
    /// Not ready.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that the database is open and migrated.
    pub fn database_opened(&self, database: Db) {
        drop(database);
    }

    /// The database, once it is open.
    #[must_use]
    pub const fn database(&self) -> Option<&Db> {
        None
    }

    /// Whether the database is open and migrated.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        false
    }
}

/// The health routes.
pub(crate) fn routes() -> Router<ApiState> {
    Router::new()
}
