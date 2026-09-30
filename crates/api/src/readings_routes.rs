//! The owner's read tap (SPEC-047 R1, R9; ADR-047): `POST /api/readings/{id}/read` marks a reading
//! read and answers where its measure stands, behind the owner's session (SPEC-024).
//!
//! The route reaches the tap through coordination's [`ReadTapPort`], so this context names no
//! readings type. The session is checked before anything else, so a request without the owner's
//! live session is answered 401 and changes nothing.

use std::sync::Arc;

use axum::Router;
use axum::extract::{FromRef, Path, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use deck_streak_coordination::readings::read_tap::{ReadTapPort, TapError};
use deck_streak_identity::{OwnerSession, Sessions};
use serde_json::{Value, json};

use crate::session_routes::OwnerAccess;

/// What the read route shares: the owner's access and the tap.
#[derive(Clone)]
struct Readings {
    access: OwnerAccess,
    tap: Arc<dyn ReadTapPort>,
}

impl FromRef<Readings> for Sessions {
    fn from_ref(readings: &Readings) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&readings.access)
    }
}

/// The read route over `access` and `tap`.
pub(crate) fn routes(access: OwnerAccess, tap: Arc<dyn ReadTapPort>) -> Router {
    Router::new()
        .route("/api/readings/{id}/read", post(read))
        .with_state(Readings { access, tap })
}

/// `POST /api/readings/{id}/read`: the tap.
async fn read(
    _owner: OwnerSession,
    State(readings): State<Readings>,
    Path(id): Path<String>,
) -> Response {
    match readings.tap.tap(&id).await {
        Ok(tapped) => json_body(
            StatusCode::OK,
            &json!({
                "id": tapped.id,
                "read_at": tapped.read_at.epoch_millis(),
                "first": tapped.first,
                "covered": tapped.covered,
                "studied": tapped.studied,
                "verdict": tapped.verdict.as_str()
            }),
        ),
        Err(TapError::NotFound) => refused(StatusCode::NOT_FOUND, "reading_not_found"),
        Err(TapError::Unavailable) => {
            refused(StatusCode::INTERNAL_SERVER_ERROR, "reading_unavailable")
        }
    }
}

/// A refusal: its status, and a JSON body naming its reason code alone.
fn refused(status: StatusCode, reason: &'static str) -> Response {
    tracing::warn!(reason, "a read tap was refused");
    json_body(status, &json!({ "reason": reason }))
}

/// `body` as a JSON answer with `status`.
fn json_body(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}
