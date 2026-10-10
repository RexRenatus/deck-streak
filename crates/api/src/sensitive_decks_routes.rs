//! The routes of the decks the learner keeps away from AI (SPEC-381 R7; ADR-392 D1, D7):
//! `GET /api/decks/sensitive` answers the marked decks, and `PUT /api/decks/{id}/sensitive` marks
//! or unmarks one, to the owner's live session alone, the change behind the same-origin
//! state-change check. Both go through coordination's `sensitive_decks` use cases.
//!
//! A deck id is a positive decimal integer, answered as a decimal string; any other id is refused
//! with 400. An id the server has not ingested yet is accepted, so a new deck can be kept away
//! before its first sync reaches the server. Until the API's database is open both answer 503.

use axum::Json;
use axum::Router;
use axum::extract::{FromRef, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use deck_streak_identity::{OwnerSession, Sessions};
use serde::Deserialize;

use crate::health::Readiness;
use crate::session_routes::{OwnerAccess, StateChange};

/// The marked decks' path.
pub const SENSITIVE_DECKS_PATH: &str = "/api/decks/sensitive";
/// One deck's mark's path.
pub const SENSITIVE_DECK_PATH: &str = "/api/decks/{id}/sensitive";

/// What the routes read.
#[derive(Clone)]
struct Marks {
    access: OwnerAccess,
    readiness: Readiness,
}

impl FromRef<Marks> for Sessions {
    fn from_ref(state: &Marks) -> Self {
        Self::from_ref(&state.access)
    }
}

/// The routes over the owner's `access` and the API's `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    Router::new()
        .route(SENSITIVE_DECKS_PATH, get(marked))
        .route(SENSITIVE_DECK_PATH, put(set))
        .with_state(Marks { access, readiness })
}

/// The PUT's body: whether the deck is kept away from AI.
#[derive(Deserialize)]
struct SetBody {
    sensitive: bool,
}

/// `GET /api/decks/sensitive`: the marked decks' ids, ascending.
async fn marked(_owner: OwnerSession, State(state): State<Marks>) -> Response {
    let _ = state.readiness.database();
    StatusCode::NOT_IMPLEMENTED.into_response()
}

/// `PUT /api/decks/{id}/sensitive`: marks or unmarks the deck, and answers the marked decks.
async fn set(
    _owner: OwnerSession,
    _change: StateChange,
    State(state): State<Marks>,
    Path(id): Path<String>,
    Json(body): Json<SetBody>,
) -> Response {
    let _ = (state.readiness.database(), id, body.sensitive);
    StatusCode::NOT_IMPLEMENTED.into_response()
}
