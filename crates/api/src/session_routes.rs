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
use axum::extract::{DefaultBodyLimit, FromRef, FromRequestParts, State};
use axum::handler::Handler;
use axum::http::header::{CONTENT_TYPE, RETRY_AFTER};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use deck_streak_identity::session::{ended_cookie, opening_cookie, presented};
use deck_streak_identity::{OwnerGate, OwnerSession, Refusal, Sessions};
use deck_streak_kernel::{Clock, StudyDay, StudyDayRule, UtcMillis};
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
/// The fetch-metadata header a browser sends with every request it makes (the CSRF bound, R9).
const SEC_FETCH_SITE: HeaderName = HeaderName::from_static("sec-fetch-site");
/// A minute of the kernel's clock, in milliseconds.
const MINUTE_MS: i64 = 60_000;

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

    /// The server's study day now, by the kernel's rule and clock: the day every owner's read
    /// answers for (SPEC-024 R8, SPEC-071 R20).
    #[must_use]
    pub fn study_day(&self) -> StudyDay {
        self.rule.study_day(self.clock.now())
    }
}

impl OwnerAccess {
    /// The study-day rule the owner's routes read, for the answer route (SPEC-110).
    pub(crate) const fn rule(&self) -> StudyDayRule {
        self.rule
    }

    /// The clock the session routes read, for the owner's other routes that need the time
    /// (SPEC-041's feed).
    pub(crate) fn clock(&self) -> Arc<dyn Clock> {
        Arc::clone(&self.clock)
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

/// The session routes over `access`. The handshake's own body limit sits on its handler, inside
/// the shell's 2 MiB, so it is the limit the handler's `Bytes` extractor reads (axum-core's
/// `DefaultBodyLimit` inserts its limit into each request, and the innermost layer inserts last).
pub(crate) fn routes(access: OwnerAccess) -> Router {
    let open = open_session.layer(DefaultBodyLimit::max(HANDSHAKE_BODY_LIMIT_BYTES));
    Router::new()
        .route(SESSION_PATH, post(open).delete(log_out))
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
    let body = serde_json::json!({ "study_day": access.study_day().to_string() }).to_string();
    (StatusCode::OK, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A request that may change state: JSON, and not cross-site (R9). It refuses anything else with
/// 403 before the route reads a byte of the body.
///
/// A cross-site HTML form can send only a form encoding or plain text, and a cross-site script
/// that sends JSON must pass a CORS preflight, which this origin never grants; `SameSite=Strict`
/// keeps the cookie off a cross-site request besides. `Sec-Fetch-Site`, which every current browser
/// sends, names a cross-site request outright; a client that sends none is judged by the rest.
pub(crate) struct StateChange;

impl<S> FromRequestParts<S> for StateChange
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let cross_site = parts
            .headers
            .get(&SEC_FETCH_SITE)
            .is_some_and(|site| site != "same-origin");
        if cross_site {
            return Err(refused(StatusCode::FORBIDDEN, "cross_site_request"));
        }
        let json = parts
            .headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"));
        if !json {
            return Err(refused(StatusCode::FORBIDDEN, "not_json"));
        }
        Ok(Self)
    }
}

/// A handshake the bound admitted this minute (R10); past it, 429 until the minute turns, with a
/// `Retry-After` of the seconds left.
struct HandshakeSlot;

impl FromRequestParts<OwnerAccess> for HandshakeSlot {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        access: &OwnerAccess,
    ) -> Result<Self, Self::Rejection> {
        match access.bound.admit(access.clock.now()) {
            Ok(()) => Ok(Self),
            Err(seconds) => {
                let mut response = refused(StatusCode::TOO_MANY_REQUESTS, "too_many_handshakes");
                response.headers_mut().insert(RETRY_AFTER, seconds.into());
                Err(response)
            }
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
        let millis = now.epoch_millis();
        let minute = millis.div_euclid(MINUTE_MS);
        let mut window = self.window.lock().unwrap_or_else(PoisonError::into_inner);
        if window.0 != minute {
            *window = (minute, 0);
        }
        if window.1 >= HANDSHAKES_PER_MINUTE {
            // Between 1 and 60 000 milliseconds are left, so between 1 and 60 whole seconds.
            let left = MINUTE_MS - millis.rem_euclid(MINUTE_MS);
            return Err(u64::try_from(left).map_or(60, |left| left.div_ceil(1000)));
        }
        window.1 += 1;
        Ok(())
    }
}

/// A refusal of this module's own: its status, and a JSON body naming its reason code alone, as
/// identity's refusals are written. It is logged by the reason code alone.
fn refused(status: StatusCode, reason: &'static str) -> Response {
    tracing::warn!(reason, "a state change was refused");
    let body = serde_json::json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// The launch validation's path (SPEC-403 R8; ADR-417 D3): the start hook posts the fragment's
/// launch data here before it loads Telegram's script.
pub const LAUNCH_PATH: &str = "/api/launch";

/// The launch route over `access`, served beside the session routes. It carries the handshake's
/// own body limit on its handler, for the reason [`routes`] gives, so a launch is bounded exactly
/// as a handshake is.
pub(crate) fn launch_routes(access: OwnerAccess) -> Router {
    let validate = validate_launch.layer(DefaultBodyLimit::max(HANDSHAKE_BODY_LIMIT_BYTES));
    Router::new()
        .route(LAUNCH_PATH, post(validate))
        .with_state(access)
}

/// `POST /api/launch`: whether the launch data is the owner's, validated by the gate the
/// handshake uses and under its three guards (the state-change guard, a slot from its bound and
/// its body limit). It answers 204 with no body and no cookie, and opens no session: the page
/// asks only whether to load Telegram's script. A refusal is the gate's status and reason alone.
async fn validate_launch(
    _state_change: StateChange,
    _slot: HandshakeSlot,
    State(access): State<OwnerAccess>,
    body: Bytes,
) -> Response {
    let Ok(Handshake { init_data }) = serde_json::from_slice(&body) else {
        return Refusal::InitDataInvalid.into_response();
    };
    match access.gate.admit(&init_data, access.clock.now()) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(refusal) => refusal.into_response(),
    }
}
