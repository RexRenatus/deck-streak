//! The API's routes and the one stack of layers every route is served under (SPEC-025 R3 to R5;
//! ADR-025).
//!
//! STUB for the red-first commit: `layered` applies no layer.

use std::time::Duration;

use axum::Router;

use crate::health::{self, Readiness};

/// Requests served at once. Each holds its buffers until it answers, so this bounds memory.
pub const MAX_IN_FLIGHT: usize = 64;
/// A request that has not answered by now is answered 408.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// The largest request body the extractors read: axum's default, stated.
pub const BODY_LIMIT_BYTES: usize = 2 * 1024 * 1024;

/// What the API's handlers share.
#[derive(Clone, Debug)]
pub struct ApiState {
    readiness: Readiness,
}

impl ApiState {
    /// The state over `readiness`.
    #[must_use]
    pub const fn new(readiness: Readiness) -> Self {
        Self { readiness }
    }

    /// Whether the API can answer from its database.
    #[must_use]
    pub const fn readiness(&self) -> &Readiness {
        &self.readiness
    }
}

/// The API: every route, under the layers.
pub fn router(state: ApiState) -> Router {
    layered(health::routes().with_state(state))
}

/// `routes` under the layers.
pub fn layered(routes: Router) -> Router {
    routes
}
