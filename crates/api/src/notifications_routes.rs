//! The in-app feed route (SPEC-041 R12, R13; ADR-041): `GET /api/notifications/feed` serves the Mini
//! App what the router delivered to it, to the owner's live session alone, and marks each item seen
//! as it serves it, so the Mini App is served each item once. The router appends to the feed; this
//! route only reads it, through notifications' ledger.
//!
//! Any other caller is refused by identity's session extractor, with no item. Until the API's
//! database is open the route answers 503, as readiness does.

use std::sync::Arc;

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::Clock;
use deck_streak_notifications::ledger::take_unseen_feed;

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The feed's path.
pub const FEED_PATH: &str = "/api/notifications/feed";

/// What the feed route reads: the owner's sessions and the clock, through the owner's access, and
/// the database once it is open.
#[derive(Clone)]
struct FeedState {
    access: OwnerAccess,
    clock: Arc<dyn Clock>,
    readiness: Readiness,
}

impl FromRef<FeedState> for Sessions {
    fn from_ref(state: &FeedState) -> Self {
        Self::from_ref(&state.access)
    }
}

/// The feed route over the owner's `access` and the API's `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    let clock = access.clock();
    Router::new()
        .route(FEED_PATH, get(feed))
        .with_state(FeedState {
            access,
            clock,
            readiness,
        })
}

/// `GET /api/notifications/feed`: the unseen items, oldest first, for the owner's live session.
async fn feed(State(state): State<FeedState>) -> Response {
    let Some(db) = state.readiness.database() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    match take_unseen_feed(db, state.clock.now()).await {
        Ok(items) => {
            let body = serde_json::json!({ "items": items }).to_string();
            (StatusCode::OK, [(CONTENT_TYPE, "application/json")], body).into_response()
        }
        Err(error) => {
            tracing::error!(%error, "the in-app feed could not be read");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
