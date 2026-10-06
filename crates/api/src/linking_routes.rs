//! The linking routes: the link code, the passkey ceremonies and the owner's methods (SPEC-359 R3
//! to R10; ADR-370).
//!
//! - `POST /api/link/code` mints a link code for a fresh `telegram` session: `{"code": ...}`.
//! - `POST /api/link/redeem` takes `{"code": ...}`, ends the session the request carried, and opens
//!   a `link` session in the session cookie: 204.
//! - `POST /api/passkeys/register/start` and `.../finish` run a registration inside a `link`
//!   session; the finish answers 201 and the new row id.
//! - `POST /api/passkeys/sign-in/start` and `.../finish` run a sign-in with no session; the finish
//!   opens a `linked` session and names the owner's accepted credential ids and user handle.
//! - `GET /api/identities` lists the owner's methods, and `DELETE /api/identities/{id}` removes a
//!   passkey from a fresh `telegram` session: 204.
//!
//! A ceremony's flow id reaches the browser only in [`CEREMONY_COOKIE`], `__Host-` prefixed and
//! strict, living as long as the ceremony, and every finish clears it, refused or not. The two
//! ceremony starts and the redeem share one bound of [`CEREMONIES_PER_MINUTE`] a minute, a window
//! of their own apart from the handshake's. With no public origin configured, every route answers
//! 404 `linking_off` before it judges a session.

use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{FromRef, FromRequestParts, Path};
use axum::http::header::{CONTENT_TYPE, COOKIE, RETRY_AFTER, SET_COOKIE};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{AppendHeaders, IntoResponse, Response};
use axum::routing::{delete, get, post};
use deck_streak_identity::session::{opening_cookie, presented};
use deck_streak_identity::{
    LinkingConfig, Owner, OwnerSession, PasskeyError, Passkeys, Refusal, Sessions,
};
use deck_streak_kernel::{Clock, Db, UtcMillis};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// Mints a link code.
pub const LINK_CODE_PATH: &str = "/api/link/code";
/// Redeems a link code.
pub const LINK_REDEEM_PATH: &str = "/api/link/redeem";
/// Starts a passkey registration.
pub const REGISTER_START_PATH: &str = "/api/passkeys/register/start";
/// Finishes a passkey registration.
pub const REGISTER_FINISH_PATH: &str = "/api/passkeys/register/finish";
/// Starts a passkey sign-in.
pub const SIGN_IN_START_PATH: &str = "/api/passkeys/sign-in/start";
/// Finishes a passkey sign-in.
pub const SIGN_IN_FINISH_PATH: &str = "/api/passkeys/sign-in/finish";
/// The owner's methods.
pub const IDENTITIES_PATH: &str = "/api/identities";
/// One of the owner's methods, by its id.
pub const IDENTITY_PATH: &str = "/api/identities/{id}";
/// The cookie a ceremony's flow id travels in (SPEC-359 R7).
pub const CEREMONY_COOKIE: &str = "__Host-deckstreak_ceremony";
/// Ceremony starts and redeems admitted in one minute of the kernel's clock, per process (R10).
pub const CEREMONIES_PER_MINUTE: u32 = 30;
/// The ceremony cookie's attributes, written once for the cookie that opens and the one that ends.
const COOKIE_ATTRIBUTES: &str = "Path=/; Secure; HttpOnly; SameSite=Strict";
/// A minute of the kernel's clock, in milliseconds.
const MINUTE_MS: i64 = 60_000;

/// What the linking routes share: the use cases when linking is on, the owner's sessions, the
/// database's readiness, the clock and the ceremony bound.
#[derive(Clone)]
struct Linking {
    passkeys: Option<Passkeys>,
    sessions: Sessions,
    readiness: Readiness,
    clock: Arc<dyn Clock>,
    bound: Arc<CeremonyBound>,
}

impl FromRef<Linking> for Sessions {
    fn from_ref(linking: &Linking) -> Self {
        linking.sessions.clone()
    }
}

/// The linking routes over `access`'s sessions and clock and `readiness`'s database. `linking`
/// is the configuration and the owner it serves; with none, or with no public origin configured,
/// every route answers `linking_off`.
pub(crate) fn routes(
    access: OwnerAccess,
    readiness: Readiness,
    linking: Option<(LinkingConfig, Owner)>,
) -> Router {
    let sessions = Sessions::from_ref(&access);
    let clock = access.clock();
    let passkeys = linking.and_then(|(config, owner)| {
        let on = config.relying_party().is_ok();
        on.then(|| Passkeys::new(config, sessions.clone(), owner, Arc::clone(&clock)))
    });
    Router::new()
        .route(LINK_CODE_PATH, post(mint_code))
        .route(LINK_REDEEM_PATH, post(redeem))
        .route(REGISTER_START_PATH, post(register_start))
        .route(REGISTER_FINISH_PATH, post(register_finish))
        .route(SIGN_IN_START_PATH, post(sign_in_start))
        .route(SIGN_IN_FINISH_PATH, post(sign_in_finish))
        .route(IDENTITIES_PATH, get(identities))
        .route(IDENTITY_PATH, delete(remove))
        .with_state(Linking {
            passkeys,
            sessions,
            readiness,
            clock,
            bound: Arc::default(),
        })
}

/// A link code's redeem body.
#[derive(Deserialize)]
struct Redeem {
    code: String,
}

/// `POST /api/link/code`.
async fn mint_code(ready: Ready, headers: HeaderMap) -> Response {
    let token = presented(&headers).unwrap_or_default();
    match ready.passkeys.mint_link_code(token) {
        Ok(code) => answer(StatusCode::OK, &json!({ "code": code.expose() })),
        Err(error) => failed(&error),
    }
}

/// `POST /api/link/redeem`.
async fn redeem(_slot: CeremonySlot, ready: Ready, headers: HeaderMap, body: Bytes) -> Response {
    let Ok(Redeem { code }) = serde_json::from_slice(&body) else {
        return failed(&Refusal::LinkCodeInvalid.into());
    };
    match ready.passkeys.redeem_link_code(&code, presented(&headers)) {
        Ok(token) => (StatusCode::NO_CONTENT, [opening_cookie(&token)]).into_response(),
        Err(error) => failed(&error),
    }
}

/// `POST /api/passkeys/register/start`.
async fn register_start(
    _slot: CeremonySlot,
    ready: Ready,
    _owner: OwnerSession,
    headers: HeaderMap,
) -> Response {
    let token = presented(&headers).unwrap_or_default();
    match ready.passkeys.start_registration(&ready.db, token).await {
        Ok(started) => answer(StatusCode::OK, &started.options),
        Err(error) => failed(&error),
    }
}

/// `POST /api/passkeys/register/finish`.
async fn register_finish(
    ready: Ready,
    _owner: OwnerSession,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let token = presented(&headers).unwrap_or_default();
    let flow = ceremony_presented(&headers).unwrap_or_default();
    let response = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
    let answered = match ready
        .passkeys
        .finish_registration(&ready.db, token, flow, &response)
        .await
    {
        Ok(row) => answer(StatusCode::CREATED, &json!({ "id": row })),
        Err(error) => failed(&error),
    };
    ceremony_ended(answered)
}

/// `POST /api/passkeys/sign-in/start`.
async fn sign_in_start(_slot: CeremonySlot, ready: Ready) -> Response {
    match ready.passkeys.start_sign_in(&ready.db).await {
        Ok(started) => answer(StatusCode::OK, &started.options),
        Err(error) => failed(&error),
    }
}

/// `POST /api/passkeys/sign-in/finish`.
async fn sign_in_finish(ready: Ready, headers: HeaderMap, body: Bytes) -> Response {
    let flow = ceremony_presented(&headers).unwrap_or_default();
    let response = serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null);
    let answered = match ready
        .passkeys
        .finish_sign_in(&ready.db, presented(&headers), flow, &response)
        .await
    {
        Ok(signed_in) => {
            let body = json!({
                "credential_ids": signed_in.credential_ids,
                "user_handle": signed_in.user_handle,
            });
            (
                [opening_cookie(&signed_in.session)],
                answer(StatusCode::OK, &body),
            )
                .into_response()
        }
        Err(error) => failed(&error),
    };
    ceremony_ended(answered)
}

/// `GET /api/identities`.
async fn identities(ready: Ready, _owner: OwnerSession) -> Response {
    let _ = ready;
    answer(StatusCode::OK, &json!([]))
}

/// `DELETE /api/identities/{id}`.
async fn remove(ready: Ready, Path(id): Path<String>, headers: HeaderMap) -> Response {
    let Ok(id) = id.parse::<i64>() else {
        return failed(&Refusal::IdentityUnknown.into());
    };
    let token = presented(&headers).unwrap_or_default();
    match ready.passkeys.remove(&ready.db, token, id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => failed(&error),
    }
}

/// The linking use cases and the database, when both are there: with no public origin, 404
/// `linking_off`; before the database opens, 503 `database_not_open`. A route takes it after the
/// cross-site bound and the ceremony bound and before any session, so a route that is off says
/// so before it judges a session.
struct Ready {
    passkeys: Passkeys,
    db: Db,
}

impl FromRequestParts<Linking> for Ready {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        linking: &Linking,
    ) -> Result<Self, Self::Rejection> {
        let Some(passkeys) = linking.passkeys.clone() else {
            return Err(failed(&Refusal::LinkingOff.into()));
        };
        let Some(db) = linking.readiness.database().cloned() else {
            return Err(refused(
                StatusCode::SERVICE_UNAVAILABLE,
                "database_not_open",
            ));
        };
        Ok(Self { passkeys, db })
    }
}

/// A ceremony start or a redeem the bound admitted this minute (R10); past it, 429 until the
/// minute turns, with a `Retry-After` of the seconds left.
struct CeremonySlot;

impl FromRequestParts<Linking> for CeremonySlot {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        linking: &Linking,
    ) -> Result<Self, Self::Rejection> {
        match linking.bound.admit(linking.clock.now()) {
            Ok(()) => Ok(Self),
            Err(seconds) => {
                let mut response = refused(StatusCode::TOO_MANY_REQUESTS, "too_many_ceremonies");
                response.headers_mut().insert(RETRY_AFTER, seconds.into());
                Err(response)
            }
        }
    }
}

/// The ceremony starts and redeems counted in the current minute of the kernel's clock.
#[derive(Debug, Default)]
struct CeremonyBound {
    /// The minute, as whole minutes since the epoch, and the requests admitted in it.
    window: Mutex<(i64, u32)>,
}

impl CeremonyBound {
    /// Admits one more request at `now`, or answers how many whole seconds remain until the
    /// minute turns.
    fn admit(&self, now: UtcMillis) -> Result<(), u64> {
        let millis = now.epoch_millis();
        let minute = millis.div_euclid(MINUTE_MS);
        let mut window = self.window.lock().unwrap_or_else(PoisonError::into_inner);
        if window.0 != minute {
            *window = (minute, 0);
        }
        window.1 = window.1.saturating_add(1);
        Ok(())
    }
}

/// The flow id the request's cookies carry: the value of the one [`CEREMONY_COOKIE`], or `None`
/// when there is none, or more than one.
fn ceremony_presented(headers: &HeaderMap) -> Option<&str> {
    let mut found = None;
    for value in headers.get_all(COOKIE) {
        let Ok(text) = value.to_str() else {
            continue;
        };
        for pair in text.split(';') {
            let Some((name, flow)) = pair.trim().split_once('=') else {
                continue;
            };
            if name == CEREMONY_COOKIE {
                if found.is_some() {
                    return None;
                }
                found = Some(flow);
            }
        }
    }
    found
}

/// `response`, with the ceremony cookie cleared beside whatever cookie it already sets.
fn ceremony_ended(response: Response) -> Response {
    let ended = format!("{CEREMONY_COOKIE}=; Max-Age=0; {COOKIE_ATTRIBUTES}");
    (AppendHeaders([(SET_COOKIE, ended)]), response).into_response()
}

/// A JSON answer.
fn answer(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// A call that did not complete: a refusal answers its status and reason code and is logged by
/// the code alone; anything else is logged and answers 500.
fn failed(error: &PasskeyError) -> Response {
    match error.refusal() {
        Some(refusal) => {
            tracing::warn!(reason = refusal.reason(), "a linking request was refused");
            refusal.into_response()
        }
        None => {
            tracing::error!(%error, "a linking request failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// A refusal of this module's own: its status, and a JSON body naming its reason code alone. It
/// is logged by the reason code alone.
fn refused(status: StatusCode, reason: &'static str) -> Response {
    tracing::warn!(reason, "a linking request was refused");
    answer(status, &json!({ "reason": reason }))
}
