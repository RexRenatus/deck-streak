//! The quick capture route (SPEC-118 R10, A15 to A18): the owner's Mini App posts one text, and it
//! lands in the vault inbox once, through the capture use case both surfaces share
//! (`coordination::inbox_capture`), to the owner's live session alone. A retry carries the same
//! `capture_id` and answers the first capture's name.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use deck_streak_coordination::inbox_capture::{
    InboxCaptures, QuickAnswer, QuickKind, RealFs, VaultError, safe_unique,
};
use deck_streak_identity::{OwnerSession, Sessions};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::{OwnerAccess, StateChange};

/// The route's path.
pub const CAPTURE_PATH: &str = "/api/inbox/captures";

/// What the capture route reads.
#[derive(Clone)]
struct Captures {
    access: OwnerAccess,
    readiness: Readiness,
    inbox: Option<Arc<InboxCaptures<RealFs>>>,
}

impl FromRef<Captures> for Sessions {
    fn from_ref(state: &Captures) -> Self {
        Self::from_ref(&state.access)
    }
}

/// The capture route over the owner's `access`, the API's `readiness` and the vault's `inbox`.
pub(crate) fn routes(
    access: OwnerAccess,
    readiness: Readiness,
    inbox: Option<Arc<InboxCaptures<RealFs>>>,
) -> Router {
    Router::new()
        .route(CAPTURE_PATH, post(capture))
        .with_state(Captures {
            access,
            readiness,
            inbox,
        })
}

/// A JSON answer of `status`.
fn json_of(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// The route's body (R10).
#[derive(Deserialize)]
struct CaptureBody {
    capture_id: String,
    kind: String,
    text: String,
}

/// A refusal of `status` naming its `reason`.
fn refused(status: StatusCode, reason: &str) -> Response {
    json_of(status, &json!({ "reason": reason }))
}

/// `POST /api/inbox/captures`: writes the quick capture once (R10). The capture id must be its own
/// safe form, so two captures never share a retry key; the text's bound is the vault's.
async fn capture(
    _owner: OwnerSession,
    _change: StateChange,
    State(state): State<Captures>,
    Json(body): Json<CaptureBody>,
) -> Response {
    if safe_unique(&body.capture_id) != body.capture_id {
        return refused(StatusCode::UNPROCESSABLE_ENTITY, "invalid_capture_id");
    }
    let Some(kind) = QuickKind::from_name(&body.kind) else {
        return refused(StatusCode::UNPROCESSABLE_ENTITY, "unknown_kind");
    };
    let Some(captures) = state.inbox else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "vault_not_open");
    };
    let Some(db) = state.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    let answer = captures
        .quick(
            db,
            &body.capture_id,
            kind,
            &body.text,
            state.access.clock().now(),
        )
        .await;
    match answer {
        Ok(QuickAnswer::Saved { name }) => json_of(StatusCode::CREATED, &json!({ "name": name })),
        Ok(QuickAnswer::AlreadyCaptured { name }) => json_of(
            StatusCode::OK,
            &json!({ "name": name, "already_captured": true }),
        ),
        Ok(QuickAnswer::TextOutOfBounds) => {
            refused(StatusCode::UNPROCESSABLE_ENTITY, "text_out_of_bounds")
        }
        Err(VaultError::VaultMissing) => refused(StatusCode::SERVICE_UNAVAILABLE, "vault_missing"),
        Err(error) => {
            tracing::error!(%error, "a quick capture could not be written");
            refused(StatusCode::INTERNAL_SERVER_ERROR, "vault_unwritable")
        }
    }
}
