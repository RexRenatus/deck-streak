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
use deck_streak_coordination::readings::read_tap::ReadTapPort;
use deck_streak_identity::{OwnerSession, Sessions};

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
    State(_readings): State<Readings>,
    Path(_id): Path<String>,
) -> Response {
    let _ = &_readings.tap;
    (
        StatusCode::NOT_IMPLEMENTED,
        [(CONTENT_TYPE, "application/json")],
        "{}".to_owned(),
    )
        .into_response()
}
