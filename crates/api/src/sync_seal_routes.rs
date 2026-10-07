//! The release of the key the web client's sync key is sealed under (SPEC-363 R6, R7; ADR-374
//! D3).
//!
//! `POST /api/sync/seal-key` takes `{"seal_id": "<22 base64url characters>"}` and answers 200
//! `{"key": "<43 base64url characters>"}`, the sealing key identity makes for that id under the
//! service's seal secret, with `Cache-Control: no-store`. Its extractors run in this order, and each
//! refuses before the next reads anything:
//!
//! 1. [`SealOn`], the off arm: 404 `sync_seal_off` while the service holds no seal secret, whoever
//!    asks;
//! 2. the session routes' state-change guard: 403 for a cross-site or non-JSON request;
//! 3. identity's [`OwnerSession`]: 401 `no_session` for no session and for a `link` session;
//! 4. [`ReleaseSlot`]: 429 `too_many_releases` with `Retry-After` past [`RELEASES_PER_MINUTE`] in a
//!    minute of the kernel's clock, in a window of the release's own;
//! 5. the body, at most [`RELEASE_BODY_LIMIT_BYTES`] (413 past it): JSON with the one field
//!    `seal_id`, which identity's `SealId::parse` must read, else 400 `seal_id_invalid`.
//!
//! No line this route logs names the body, a seal id, a key or the secret (R7): a refusal is logged
//! by its reason code, and a release by the fact alone.

use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, FromRef, FromRequestParts};
use axum::handler::Handler;
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, RETRY_AFTER};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use deck_streak_identity::sync_seal::{SealId, SealSecret};
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::UtcMillis;
use serde::Deserialize;

use crate::session_routes::{OwnerAccess, StateChange};

/// The release's path.
pub const SEAL_KEY_PATH: &str = "/api/sync/seal-key";
/// Releases admitted in one minute of the kernel's clock, per process (ADR-374 D3). It is the
/// handshake's figure in a window of the release's own, so a flood of either never closes the
/// other.
pub const RELEASES_PER_MINUTE: u32 = 30;
/// The largest release body read. `{"seal_id": "<22 characters>"}` is 35 bytes; this is several
/// times that, and far inside the shell's limit.
pub const RELEASE_BODY_LIMIT_BYTES: usize = 256;
/// A minute of the kernel's clock, in milliseconds.
const MINUTE_MS: i64 = 60_000;

/// What the release reads: the owner's access, for its sessions and its clock, the seal secret
/// when the service holds one, and the release's own window. A handle: every clone shares the
/// window.
#[derive(Clone)]
pub(crate) struct SealRelease {
    access: OwnerAccess,
    secret: Option<Arc<SealSecret>>,
    bound: Arc<ReleaseBound>,
}

impl FromRef<SealRelease> for Sessions {
    fn from_ref(release: &SealRelease) -> Self {
        Self::from_ref(&release.access)
    }
}

/// The release route over `access`, with the seal secret `secret` when the service holds one. The
/// release's body limit sits on its handler, inside the shell's, so it is the limit the handler's
/// `Bytes` extractor reads.
pub(crate) fn routes(access: OwnerAccess, secret: Option<Arc<SealSecret>>) -> Router {
    let handler = release.layer(DefaultBodyLimit::max(RELEASE_BODY_LIMIT_BYTES));
    Router::new()
        .route(SEAL_KEY_PATH, post(handler))
        .with_state(SealRelease {
            access,
            secret,
            bound: Arc::default(),
        })
}

/// The release's body, as the Worker sends it: the one field, and nothing beside it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseRequest {
    seal_id: String,
}

/// `POST /api/sync/seal-key`: the sealing key for one seal id, to the owner's live session.
async fn release(
    SealOn(secret): SealOn,
    _state_change: StateChange,
    _owner: OwnerSession,
    _slot: ReleaseSlot,
    body: Bytes,
) -> Response {
    let id = serde_json::from_slice::<ReleaseRequest>(&body)
        .ok()
        .and_then(|request| SealId::parse(&request.seal_id));
    let Some(id) = id else {
        return refused(StatusCode::BAD_REQUEST, "seal_id_invalid");
    };
    let key = secret.key_for(&id).encoded();
    tracing::info!("a sealing key was released");
    let body = serde_json::json!({ "key": key }).to_string();
    (
        StatusCode::OK,
        [
            (CONTENT_TYPE, "application/json"),
            (CACHE_CONTROL, "no-store"),
        ],
        body,
    )
        .into_response()
}

/// The seal secret, when the service holds one. It is the release's FIRST extractor, so a request
/// to a service that holds none reads 404 `sync_seal_off` before anything else is judged, and the
/// route is off rather than refusing (SPEC-363 R6).
struct SealOn(Arc<SealSecret>);

impl FromRequestParts<SealRelease> for SealOn {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        release: &SealRelease,
    ) -> Result<Self, Self::Rejection> {
        match &release.secret {
            Some(secret) => Ok(Self(Arc::clone(secret))),
            None => Err(refused(StatusCode::NOT_FOUND, "sync_seal_off")),
        }
    }
}

/// A release the window admitted this minute; past [`RELEASES_PER_MINUTE`], 429 until the minute
/// turns, with a `Retry-After` of the whole seconds left.
struct ReleaseSlot;

impl FromRequestParts<SealRelease> for ReleaseSlot {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        release: &SealRelease,
    ) -> Result<Self, Self::Rejection> {
        match release.bound.admit(release.access.clock().now()) {
            Ok(()) => Ok(Self),
            Err(seconds) => {
                let mut response = refused(StatusCode::TOO_MANY_REQUESTS, "too_many_releases");
                response.headers_mut().insert(RETRY_AFTER, seconds.into());
                Err(response)
            }
        }
    }
}

/// The releases counted in the current minute of the kernel's clock.
#[derive(Debug, Default)]
struct ReleaseBound {
    /// The minute, as whole minutes since the epoch, and the releases admitted in it.
    window: Mutex<(i64, u32)>,
}

impl ReleaseBound {
    /// Admits one more release at `now`, or answers how many whole seconds remain until the minute
    /// turns.
    fn admit(&self, now: UtcMillis) -> Result<(), u64> {
        let millis = now.epoch_millis();
        let minute = millis.div_euclid(MINUTE_MS);
        let mut window = self.window.lock().unwrap_or_else(PoisonError::into_inner);
        if window.0 != minute {
            *window = (minute, 0);
        }
        if window.1 >= RELEASES_PER_MINUTE {
            // Between 1 and 60 000 milliseconds are left, so between 1 and 60 whole seconds.
            let left = MINUTE_MS - millis.rem_euclid(MINUTE_MS);
            return Err(u64::try_from(left).map_or(60, |left| left.div_ceil(1000)));
        }
        window.1 += 1;
        Ok(())
    }
}

/// A refusal of the release's own: its status, and a JSON body naming its reason code alone. It
/// is logged by the reason code alone.
fn refused(status: StatusCode, reason: &'static str) -> Response {
    tracing::warn!(reason, "a sealing key release was refused");
    let body = serde_json::json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}
