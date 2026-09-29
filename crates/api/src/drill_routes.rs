//! The law-drill routes (SPEC-110 R9, A19): the owner's Mini App lists the drills, reads one, and
//! answers one, to the owner's live session alone. The answer goes through the vault contract's one
//! writer, `coordination::drills::answer`, so the Mini App, the bot and a re-poll never write twice.

use std::sync::Arc;

use axum::Router;
use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use deck_streak_coordination::drills::{DrillNotes, RealFs};
use deck_streak_identity::{OwnerSession, Sessions};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The list's path.
pub const LIST_PATH: &str = "/api/drills";

/// What the drill routes read.
#[derive(Clone)]
struct Drills {
    access: OwnerAccess,
    readiness: Readiness,
    notes: Option<Arc<DrillNotes<RealFs>>>,
}

impl FromRef<Drills> for Sessions {
    fn from_ref(state: &Drills) -> Self {
        Self::from_ref(&state.access)
    }
}

/// The drill routes over the owner's `access`, the API's `readiness` and the vault's `notes`.
pub(crate) fn routes(
    access: OwnerAccess,
    readiness: Readiness,
    notes: Option<Arc<DrillNotes<RealFs>>>,
) -> Router {
    Router::new()
        .route(LIST_PATH, get(unbuilt))
        .route("/api/drills/{id}", get(unbuilt))
        .route("/api/drills/{id}/answer", post(unbuilt))
        .with_state(Drills {
            access,
            readiness,
            notes,
        })
}

/// A route not yet built.
async fn unbuilt(_owner: OwnerSession) -> Response {
    StatusCode::NOT_IMPLEMENTED.into_response()
}
