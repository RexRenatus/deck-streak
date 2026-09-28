//! The owner's session routes: the handshake, the logout and the owner's day (SPEC-024 R8 to R10;
//! ADR-024, ADR-025).
//!
//! - `POST /api/session` takes the Mini App's JSON body, `{"init_data": "<the raw launch data>"}`,
//!   and hands the launch data to identity's [`OwnerGate`]. The owner gets a NEW session, whose id
//!   travels once, in the `__Host-` cookie; the session the request carried ends. It answers 200,
//!   or 401 or 403 with a reason code.
//! - `DELETE /api/session` ends the session its cookie names, on the server, and clears the cookie.
//! - `GET /api/me`, behind [`OwnerSession`], answers the server's study day as its ISO date, from
//!   the kernel's rule and clock.
//!
//! The two routes that change state admit only a JSON request that is not cross-site (R9, the
//! CSRF bound; Caddy gives the Mini App and the API one origin, so CORS stays closed), and the
//! handshake is bounded twice more: at [`HANDSHAKES_PER_MINUTE`] a minute, which caps the HMAC work
//! a flood of forged payloads can buy (R10), and at [`HANDSHAKE_BODY_LIMIT_BYTES`] of body, well
//! inside the shell's 2 MiB. Every route is served under the shell's layers ([`crate::router`]).

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use deck_streak_identity::session::{ended_cookie, opening_cookie, presented};
use deck_streak_identity::{OwnerGate, OwnerSession, Refusal, Sessions};
use deck_streak_kernel::{Clock, StudyDayRule, UtcMillis};
use serde::Deserialize;

/// The handshake's path, and the logout's.
pub const SESSION_PATH: &str = "/api/session";
/// The owner's day.
pub const ME_PATH: &str = "/api/me";
/// The largest handshake body read. A launch payload is a few hundred bytes to a few kilobytes;
/// this is several times that, and a hundred and twenty-eighth of the shell's limit (ADR-024).
pub const HANDSHAKE_BODY_LIMIT_BYTES: usize = 16 * 1024;
/// Handshakes admitted in one minute of the kernel's clock, per process (ADR-024).
pub const HANDSHAKES_PER_MINUTE: u32 = 30;

/// What the session routes share: the owner's gate, the sessions, the clock, the study-day rule
/// and the handshake bound. A handle: every clone reads and writes the same sessions and bound.
#[derive(Clone)]
pub struct OwnerAccess {
    gate: Arc<OwnerGate>,
    sessions: Sessions,
    clock: Arc<dyn Clock>,
    rule: StudyDayRule,
    bound: Arc<HandshakeBound>,
}

impl OwnerAccess {
    /// The routes' state over `gate`, with an empty session store aging by `clock`, and the study
    /// day read by `rule`.
    #[must_use]
    pub fn new(gate: OwnerGate, clock: Arc<dyn Clock>, rule: StudyDayRule) -> Self {
        Self {
            gate: Arc::new(gate),
            sessions: Sessions::new(Arc::clone(&clock)),
            clock,
            rule,
            bound: Arc::default(),
        }
    }
}

impl fmt::Debug for OwnerAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OwnerAccess")
            .field("sessions", &self.sessions)
            .field("rule", &self.rule)
            .finish_non_exhaustive()
    }
}

impl FromRef<OwnerAccess> for Sessions {
    fn from_ref(access: &OwnerAccess) -> Self {
        access.sessions.clone()
    }
}

/// The session routes over `access`.
pub(crate) fn routes(access: OwnerAccess) -> Router {
    Router::new()
        .route(SESSION_PATH, post(open_session).delete(log_out))
        .route(ME_PATH, get(me))
        .with_state(access)
}

/// The handshake's body, as the Mini App sends it.
#[derive(Deserialize)]
struct Handshake {
    init_data: String,
}

/// `POST /api/session`: the owner's launch data opens a new session.
async fn open_session(
    _state_change: StateChange,
    _slot: HandshakeSlot,
    State(access): State<OwnerAccess>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Ok(Handshake { init_data }) = serde_json::from_slice(&body) else {
        let refusal = Refusal::InitDataInvalid;
        tracing::warn!(reason = refusal.reason(), "a handshake was refused");
        return refusal.into_response();
    };
    let owner = match access.gate.admit(&init_data, access.clock.now()) {
        Ok(owner) => owner,
        Err(refusal) => return refusal.into_response(),
    };
    // A handshake never keeps the session it carried: that one ends, and a new one opens (R5).
    if let Some(carried) = presented(&headers) {
        access.sessions.end_session(carried);
    }
    match access.sessions.open(owner) {
        Ok(token) => (StatusCode::OK, [opening_cookie(&token)]).into_response(),
        Err(error) => {
            tracing::error!(%error, "a session could not be opened");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// `DELETE /api/session`: logging out ends the session on the server, then clears the cookie.
async fn log_out(
    _state_change: StateChange,
    State(access): State<OwnerAccess>,
    headers: HeaderMap,
) -> Response {
    if let Some(token) = presented(&headers) {
        access.sessions.end_session(token);
    }
    (StatusCode::NO_CONTENT, [ended_cookie()]).into_response()
}

/// `GET /api/me`: the server's study day, for the owner's live session alone.
async fn me(_owner: OwnerSession, State(access): State<OwnerAccess>) -> Response {
    let day = access.rule.study_day(access.clock.now());
    let body = serde_json::json!({ "study_day": day.to_string() }).to_string();
    (StatusCode::OK, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A request that may change state: JSON, and not cross-site (R9). It refuses anything else with
/// 403 before the route reads a byte of the body.
struct StateChange;

impl<S> FromRequestParts<S> for StateChange
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let _unread = parts;
        Ok(Self)
    }
}

/// A handshake the bound admitted this minute (R10); past it, 429 until the minute turns.
struct HandshakeSlot;

impl FromRequestParts<OwnerAccess> for HandshakeSlot {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        access: &OwnerAccess,
    ) -> Result<Self, Self::Rejection> {
        match access.bound.admit(access.clock.now()) {
            Ok(()) => Ok(Self),
            Err(_seconds) => Err(StatusCode::TOO_MANY_REQUESTS.into_response()),
        }
    }
}

/// The handshakes counted in the current minute of the kernel's clock.
#[derive(Debug, Default)]
struct HandshakeBound {
    /// The minute, as whole minutes since the epoch, and the handshakes admitted in it.
    window: Mutex<(i64, u32)>,
}

impl HandshakeBound {
    /// Admits one more handshake at `now`, or answers how many whole seconds remain until the
    /// minute turns.
    fn admit(&self, now: UtcMillis) -> Result<(), u64> {
        let _unread = (
            now,
            self.window.lock().unwrap_or_else(PoisonError::into_inner),
        );
        Ok(())
    }
}
