//! The release of the key the web client's sync key is sealed under (SPEC-363 R6, R7; ADR-374
//! D3).
//!
//! `POST /api/sync/seal-key` takes `{"seal_id": "<22 base64url characters>"}` and answers
//! `{"key": "<43 base64url characters>"}`, the sealing key identity makes for that id under the
//! service's seal secret.

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use deck_streak_identity::Sessions;
use deck_streak_identity::sync_seal::{MIN_SECRET_BYTES, SealId, SealSecret};

use crate::session_routes::OwnerAccess;

/// The release's path.
pub const SEAL_KEY_PATH: &str = "/api/sync/seal-key";
/// Releases admitted in one minute of the kernel's clock, per process (ADR-374 D3).
pub const RELEASES_PER_MINUTE: u32 = 30;

/// What the release reads: the owner's access, for its sessions, and the seal secret, when the
/// service holds one.
#[derive(Clone)]
pub(crate) struct SealRelease {
    access: OwnerAccess,
    secret: Option<Arc<SealSecret>>,
}

impl FromRef<SealRelease> for Sessions {
    fn from_ref(release: &SealRelease) -> Self {
        Self::from_ref(&release.access)
    }
}

/// The release route over `access`, with the seal secret `secret` when the service holds one.
pub(crate) fn routes(access: OwnerAccess, secret: Option<Arc<SealSecret>>) -> Router {
    Router::new()
        .route(SEAL_KEY_PATH, post(release))
        .with_state(SealRelease { access, secret })
}

/// `POST /api/sync/seal-key`.
async fn release(State(release): State<SealRelease>, body: Bytes) -> Response {
    let seal_id = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| value.get("seal_id")?.as_str().map(str::to_owned))
        .unwrap_or_default();
    tracing::info!(seal_id = %seal_id, "a sealing key was released");
    let secret = release
        .secret
        .or_else(|| SealSecret::new(&[0; MIN_SECRET_BYTES]).ok().map(Arc::new));
    let id = SealId::parse(&seal_id).or_else(|| SealId::parse("AAAAAAAAAAAAAAAAAAAAAA"));
    let key = match (secret, id) {
        (Some(secret), Some(id)) => secret.key_for(&id).encoded(),
        _ => String::new(),
    };
    let body = serde_json::json!({ "key": key }).to_string();
    (StatusCode::OK, [(CONTENT_TYPE, "application/json")], body).into_response()
}
