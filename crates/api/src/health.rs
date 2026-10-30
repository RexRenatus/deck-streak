//! Liveness and readiness (SPEC-025 R2; ADR-025).
//!
//! It ports the predecessor's split (`server.py:create_server`, `pipeline_layers/ops.py:
//! OpsLayer.health_status` at `27ee2bc`): liveness answers 200 while the process runs, whatever
//! state it is in, and readiness answers 503 until the service can answer from its database, then
//! 200. Unlike the predecessor's, both bodies carry only a status word and the release version:
//! the routes sit on the API's one listener, and a health body must never describe the service's
//! internals (a path, a count or a setting) to whoever reaches it. Caddy answers 404 for both paths
//! from outside (ADR-025).

use std::sync::{Arc, OnceLock};

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_kernel::Db;

use crate::router::ApiState;

/// Liveness: 200 while the process runs.
pub const LIVEZ: &str = "/api/livez";
/// Readiness: 503 until the database is open and migrated, 200 after.
pub const READYZ: &str = "/api/readyz";
/// The release version both health bodies carry.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A health body: its status word and the release version, and nothing else. Both are fixed at
/// build time, so the JSON is written out whole rather than serialised.
macro_rules! health_body {
    ($word:literal) => {
        concat!(
            "{\"status\":\"",
            $word,
            "\",\"version\":\"",
            env!("CARGO_PKG_VERSION"),
            "\"}"
        )
    };
}

/// The process runs.
const ALIVE: &str = health_body!("alive");
/// The database is open and migrated.
const READY: &str = health_body!("ready");
/// The database is not open yet.
const STARTING: &str = health_body!("starting");

/// Whether the API can answer from its database: it becomes ready only when it is handed a database
/// the kernel opened and migrated, and it keeps that database for the routes that read it.
///
/// A handle: every clone reads and records the same state.
#[derive(Clone, Debug, Default)]
pub struct Readiness {
    database: Arc<OnceLock<Db>>,
}

impl Readiness {
    /// Not ready: no database is open yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that the database is open and migrated: `database` is what `Db::open` returned.
    /// The first database recorded is the one kept.
    pub fn database_opened(&self, database: Db) {
        // A second database is never opened; were one recorded, the first stays in use.
        drop(self.database.set(database));
    }

    /// The database, once it is open.
    #[must_use]
    pub fn database(&self) -> Option<&Db> {
        self.database.get()
    }

    /// Whether the database is open and migrated.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.database.get().is_some()
    }
}

/// The health routes, on axum 0.8's path syntax.
pub(crate) fn routes() -> Router<ApiState> {
    Router::new()
        .route(LIVEZ, get(livez))
        .route(READYZ, get(readyz))
}

/// Liveness answers while the process runs: if it can answer at all, it is alive.
async fn livez() -> Response {
    answer(StatusCode::OK, ALIVE)
}

/// Readiness answers 200 once the database is open and migrated, and 503 until then.
async fn readyz(State(state): State<ApiState>) -> Response {
    if state.readiness().is_ready() {
        answer(StatusCode::OK, READY)
    } else {
        answer(StatusCode::SERVICE_UNAVAILABLE, STARTING)
    }
}

fn answer(status: StatusCode, body: &'static str) -> Response {
    (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}
