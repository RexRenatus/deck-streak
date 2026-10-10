//! The snapshot answer (SPEC-377 R12 to R14; ADR-388 D14).
//!
//! `GET /api/sync/snapshot` answers whether the archive holds a sealed snapshot of the server's
//! collection, and how old the newest is.
//!
//! This is the red stub: it answers found always.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::session_routes::OwnerAccess;

/// The answer's path.
pub const SNAPSHOT_PATH: &str = "/api/sync/snapshot";
/// Answers read in one minute of the kernel's clock, per process.
pub const READS_PER_MINUTE: u32 = 30;

/// The archive's listing as the service reads it. The daemon implements it over the list command.
pub trait SnapshotLister: Send + Sync {
    /// The names the archive lists, or `None` when it could not list them.
    fn list(&self) -> Pin<Box<dyn Future<Output = Option<Vec<String>>> + Send + '_>>;
}

/// The snapshot route over `access`, with `lister` when the deployment wires one.
pub(crate) fn routes(access: OwnerAccess, lister: Option<Arc<dyn SnapshotLister>>) -> Router {
    let _ = (access, lister);
    Router::new().route(SNAPSHOT_PATH, get(snapshot))
}

/// `GET /api/sync/snapshot`: the red stub answers found, whoever asks.
async fn snapshot() -> Response {
    answered(&serde_json::json!({ "found": true, "age_seconds": 0 }))
}

/// A 200 answer with `value` as its JSON body.
fn answered(value: &serde_json::Value) -> Response {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        value.to_string(),
    )
        .into_response()
}
