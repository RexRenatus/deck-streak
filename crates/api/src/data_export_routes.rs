//! The owner's export, for the study clients (SPEC-516 R2, R3; ADR-534 D1, D2).
//!
//! `POST /api/data/export` answers the bytes `deckstreakd data export` writes over the same
//! database: coordination's [`export_document`], the export's one line and its newline, as an
//! attachment named [`EXPORT_FILE_NAME`]. It is a POST, so no link, preview or prefetch starts a
//! download. Its extractors run in this order, and each refuses before the next reads anything:
//!
//! 1. the session routes' state-change guard: 403 `cross_site_request` or `not_json`;
//! 2. identity's [`OwnerSession`]: 401 `no_session` for no session and for a `link` session;
//! 3. [`ExportSlot`]: 429 `too_many_exports` with `Retry-After` past [`EXPORTS_PER_MINUTE`] in a
//!    minute of the kernel's clock, in a window of the route's own, so a flood of exports never
//!    spends the handshake's bound or the seal release's;
//! 4. the body, at most [`EXPORT_BODY_LIMIT_BYTES`] (413 past it), on the handler and inside the
//!    shell's 2 MiB;
//!
//! and then the database: 503 `database_not_open` until it is open. Every answer, a refusal
//! included, carries `Cache-Control: no-store`. No line the route logs names the document or a
//! row: an export is logged by the fact alone, and a refusal by its reason code.

use std::future::{Ready, ready};
use std::sync::{Arc, Mutex, PoisonError};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, FromRef, FromRequestParts, State};
use axum::handler::Handler;
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, RETRY_AFTER};
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::map_response;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use deck_streak_coordination::data_rights_registry::export_document;
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::UtcMillis;

use crate::health::Readiness;
use crate::session_routes::{OwnerAccess, StateChange};

/// The export's path.
pub const EXPORT_PATH: &str = "/api/data/export";
/// Exports admitted in one minute of the kernel's clock, per process, in a window of the route's
/// own. An export reads every table once; six a minute is more than one owner's screen asks for
/// and bounds the reads a stolen session could buy.
pub const EXPORTS_PER_MINUTE: u32 = 6;
/// The largest export body read: the request carries `{}` or nothing.
pub const EXPORT_BODY_LIMIT_BYTES: usize = 64;
/// The name the answer's attachment carries.
pub const EXPORT_FILE_NAME: &str = "deck-streak-export.json";
/// A minute of the kernel's clock, in milliseconds.
const MINUTE_MS: i64 = 60_000;

/// What the route reads: the owner's access, for its sessions and its clock, the database once it
/// is open, and the route's own window. A handle: every clone shares the window.
#[derive(Clone)]
struct Exports {
    access: OwnerAccess,
    readiness: Readiness,
    bound: Arc<ExportBound>,
}

impl FromRef<Exports> for Sessions {
    fn from_ref(exports: &Exports) -> Self {
        Self::from_ref(&exports.access)
    }
}

/// The export route over `access` and `readiness`. The body limit sits on the handler, inside the
/// shell's 2 MiB, so it is the limit the handler's `Bytes` extractor reads.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    let export = export.layer(DefaultBodyLimit::max(EXPORT_BODY_LIMIT_BYTES));
    Router::new()
        .route(EXPORT_PATH, post(export))
        .route_layer(map_response(no_store))
        .with_state(Exports {
            access,
            readiness,
            bound: Arc::default(),
        })
}

/// `POST /api/data/export`: the owner's export, as the data role writes it, to the owner's live
/// session. The body is read only to hold it to its bound.
async fn export(
    _state_change: StateChange,
    _owner: OwnerSession,
    _slot: ExportSlot,
    State(exports): State<Exports>,
    _body: Bytes,
) -> Response {
    let Some(db) = exports.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    match export_document(db).await {
        Ok(document) => {
            tracing::info!("the owner's export was answered");
            let disposition = format!("attachment; filename=\"{EXPORT_FILE_NAME}\"");
            (
                StatusCode::OK,
                [
                    (CONTENT_TYPE, "application/json".to_owned()),
                    (CONTENT_DISPOSITION, disposition),
                ],
                document,
            )
                .into_response()
        }
        Err(_) => refused(StatusCode::INTERNAL_SERVER_ERROR, "export_failed"),
    }
}

/// An export the route's window admitted this minute; past [`EXPORTS_PER_MINUTE`], 429 until the
/// minute turns, with a `Retry-After` of the whole seconds left.
struct ExportSlot;

impl FromRequestParts<Exports> for ExportSlot {
    type Rejection = Response;

    async fn from_request_parts(
        _parts: &mut Parts,
        exports: &Exports,
    ) -> Result<Self, Self::Rejection> {
        match exports.bound.admit(exports.access.clock().now()) {
            Ok(()) => Ok(Self),
            Err(seconds) => {
                let mut response = refused(StatusCode::TOO_MANY_REQUESTS, "too_many_exports");
                response.headers_mut().insert(RETRY_AFTER, seconds.into());
                Err(response)
            }
        }
    }
}

/// The exports counted in the current minute of the kernel's clock.
#[derive(Debug, Default)]
struct ExportBound {
    /// The minute, as whole minutes since the epoch, and the exports admitted in it.
    window: Mutex<(i64, u32)>,
}

impl ExportBound {
    /// Admits one more export at `now`, or answers how many whole seconds remain until the minute
    /// turns.
    fn admit(&self, now: UtcMillis) -> Result<(), u64> {
        let millis = now.epoch_millis();
        let minute = millis.div_euclid(MINUTE_MS);
        let mut window = self.window.lock().unwrap_or_else(PoisonError::into_inner);
        if window.0 != minute {
            *window = (minute, 0);
        }
        if window.1 >= EXPORTS_PER_MINUTE {
            // Between 1 and 60 000 milliseconds are left, so between 1 and 60 whole seconds.
            let left = MINUTE_MS - millis.rem_euclid(MINUTE_MS);
            return Err(u64::try_from(left).map_or(60, |left| left.div_ceil(1000)));
        }
        window.1 += 1;
        Ok(())
    }
}

/// A refusal of the route's own: its status, and a JSON body naming its reason code alone. It is
/// logged by the reason code alone.
fn refused(status: StatusCode, reason: &'static str) -> Response {
    tracing::warn!(reason, "an export was refused");
    let body = serde_json::json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// Every answer of the route, a refusal included, is kept by no cache: it is the owner's whole
/// record.
fn no_store(mut response: Response) -> Ready<Response> {
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    ready(response)
}
