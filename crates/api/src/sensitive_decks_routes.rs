//! The routes of the decks the learner keeps away from AI (SPEC-381 R7; ADR-392 D1, D7):
//! `GET /api/decks/sensitive` answers the marked decks, and `PUT /api/decks/{id}/sensitive` marks
//! or unmarks one, to the owner's live session alone, the change behind the same-origin
//! state-change check. Both go through coordination's `sensitive_decks` use cases.
//!
//! A deck id is a positive decimal integer, answered as a decimal string; any other id is refused
//! with 400. An id the server has not ingested yet is accepted, so a new deck can be kept away
//! before its first sync reaches the server. Until the API's database is open both answer 503.

use std::collections::BTreeSet;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{FromRef, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use deck_streak_coordination::sensitive_decks;
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::{Clock, KernelError};
use serde::Deserialize;
use serde_json::json;

use crate::health::Readiness;
use crate::session_routes::{OwnerAccess, StateChange};

/// The marked decks' path.
pub const SENSITIVE_DECKS_PATH: &str = "/api/decks/sensitive";
/// One deck's mark's path.
pub const SENSITIVE_DECK_PATH: &str = "/api/decks/{id}/sensitive";

/// What the routes read: the owner's sessions and the clock, through the owner's access, and the
/// database once it is open.
#[derive(Clone)]
struct Marks {
    access: OwnerAccess,
    clock: Arc<dyn Clock>,
    readiness: Readiness,
}

impl FromRef<Marks> for Sessions {
    fn from_ref(state: &Marks) -> Self {
        Self::from_ref(&state.access)
    }
}

/// The routes over the owner's `access` and the API's `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    let clock = access.clock();
    Router::new()
        .route(SENSITIVE_DECKS_PATH, get(marked))
        .route(SENSITIVE_DECK_PATH, put(set))
        .with_state(Marks {
            access,
            clock,
            readiness,
        })
}

/// The PUT's body: whether the deck is kept away from AI.
#[derive(Deserialize)]
struct SetBody {
    sensitive: bool,
}

/// `GET /api/decks/sensitive`: the marked decks' ids, ascending.
async fn marked(_owner: OwnerSession, State(state): State<Marks>) -> Response {
    let Some(db) = state.readiness.database() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    answer(sensitive_decks::list(db).await)
}

/// `PUT /api/decks/{id}/sensitive`: marks or unmarks the deck, and answers the marked decks.
async fn set(
    _owner: OwnerSession,
    _change: StateChange,
    State(state): State<Marks>,
    Path(id): Path<String>,
    Json(body): Json<SetBody>,
) -> Response {
    let Some(deck) = deck_id(&id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(db) = state.readiness.database() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let marks = if body.sensitive {
        sensitive_decks::mark(db, deck, state.clock.now()).await
    } else {
        sensitive_decks::unmark(db, deck).await
    };
    answer(marks)
}

/// A deck id from the path: ASCII digits alone, at most an `i64`, and above zero; any other text,
/// a sign included, is no id.
fn deck_id(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok().filter(|id| *id > 0)
}

/// The marked decks as `{"decks":[...]}`, each id a decimal string, ascending; a failed read or
/// write answers 500 and names no deck.
fn answer(marks: Result<BTreeSet<i64>, KernelError>) -> Response {
    match marks {
        Ok(decks) => {
            let decks: Vec<String> = decks.iter().map(i64::to_string).collect();
            (StatusCode::OK, Json(json!({ "decks": decks }))).into_response()
        }
        Err(error) => {
            tracing::error!(%error, "the decks kept away from AI could not be read or changed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
