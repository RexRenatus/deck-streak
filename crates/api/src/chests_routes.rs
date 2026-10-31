//! The owner's chest and token routes (SPEC-411 R13; ADR-425 D4): the inventory, a chest's open,
//! an Epic's pick and a token's activation, each through coordination's `chests` use cases.
//!
//! Every route answers the owner's live session alone, and each POST sits behind the same-origin
//! state-change check, so a request without the session answers 401 and a cross-site POST 403,
//! both before anything is read or written. A chest id is a positive decimal integer; any other
//! id is refused with 400. Until the API's database is open every route answers 503. A refusal the
//! use case answers carries its code as `{"reason": code}`.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{FromRef, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use deck_streak_coordination::chests::{
    self, ActivateAnswer, Inventory, OpenAnswer, PickAnswer, Prize,
};
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::{Clock, KernelError, UtcMillis};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::{OwnerAccess, StateChange};

/// The inventory's path.
pub const CHESTS_PATH: &str = "/api/chests";
/// One chest's open path.
pub const CHEST_OPEN_PATH: &str = "/api/chests/{id}/open";
/// One Epic's pick path.
pub const CHEST_PICK_PATH: &str = "/api/chests/{id}/pick";
/// The token activation's path.
pub const TOKEN_ACTIVATE_PATH: &str = "/api/tokens/activate";

/// What the routes read: the owner's sessions, study day and clock, through the owner's access,
/// and the database once it is open.
#[derive(Clone)]
struct Chests {
    access: OwnerAccess,
    clock: Arc<dyn Clock>,
    readiness: Readiness,
}

impl FromRef<Chests> for Sessions {
    fn from_ref(state: &Chests) -> Self {
        Self::from_ref(&state.access)
    }
}

/// The routes over the owner's `access` and the API's `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    let clock = access.clock();
    Router::new()
        .route(CHESTS_PATH, get(inventory))
        .route(CHEST_OPEN_PATH, post(open))
        .route(CHEST_PICK_PATH, post(pick))
        .route(TOKEN_ACTIVATE_PATH, post(activate))
        .with_state(Chests {
            access,
            clock,
            readiness,
        })
}

/// The pick's choice, as the owner's surfaces write it; any other word is refused with 422.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Choice {
    Token,
    Freeze,
}

/// The pick's body.
#[derive(Deserialize)]
struct PickBody {
    choice: Choice,
}

/// `GET /api/chests`: the unresolved chests, the tokens held and the open window's end.
async fn inventory(_owner: OwnerSession, State(state): State<Chests>) -> Response {
    let Some(db) = state.readiness.database() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    match chests::chest_inventory(db, state.clock.now()).await {
        Ok(inventory) => (StatusCode::OK, Json(inventory_body(&inventory))).into_response(),
        Err(error) => failed(&error),
    }
}

/// `POST /api/chests/{id}/open`: opens the chest and pays it, in one write.
async fn open(
    _owner: OwnerSession,
    _change: StateChange,
    State(state): State<Chests>,
    Path(id): Path<String>,
) -> Response {
    let Some(chest_id) = chest_id(&id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(db) = state.readiness.database() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    match chests::open_chest(db, chest_id, state.clock.now()).await {
        Ok(OpenAnswer::Opened(opened)) => (
            StatusCode::OK,
            Json(json!({
                "chest_id": opened.chest_id,
                "rarity": opened.rarity,
                "xp": opened.xp,
                "choice_pending": opened.choice_pending,
            })),
        )
            .into_response(),
        Ok(OpenAnswer::AlreadyOpened) => refused(StatusCode::CONFLICT, "already_opened"),
        Ok(OpenAnswer::NoSuchChest) => refused(StatusCode::NOT_FOUND, "no_such_chest"),
        Err(error) => failed(&error),
    }
}

/// `POST /api/chests/{id}/pick`: settles an opened Epic's prize on the owner's study day.
async fn pick(
    _owner: OwnerSession,
    _change: StateChange,
    State(state): State<Chests>,
    Path(id): Path<String>,
    Json(body): Json<PickBody>,
) -> Response {
    let Some(chest_id) = chest_id(&id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Some(db) = state.readiness.database() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let prize = match body.choice {
        Choice::Token => Prize::Token,
        Choice::Freeze => Prize::Freeze,
    };
    let today = state.access.study_day();
    match chests::pick_epic_choice(db, chest_id, prize, today, state.clock.now()).await {
        Ok(PickAnswer::Chosen {
            prize,
            capped,
            token_id,
        }) => (
            StatusCode::OK,
            Json(json!({ "choice": prize.name(), "capped": capped, "token_id": token_id })),
        )
            .into_response(),
        Ok(PickAnswer::NotAnEpic) => refused(StatusCode::CONFLICT, "not_an_epic"),
        Ok(PickAnswer::AlreadySettled) => refused(StatusCode::CONFLICT, "already_settled"),
        Ok(PickAnswer::NoSuchChest) => refused(StatusCode::NOT_FOUND, "no_such_chest"),
        Err(error) => failed(&error),
    }
}

/// `POST /api/tokens/activate`: opens the oldest held token's window.
async fn activate(
    _owner: OwnerSession,
    _change: StateChange,
    State(state): State<Chests>,
) -> Response {
    let Some(db) = state.readiness.database() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    match chests::activate_token(db, state.clock.now()).await {
        Ok(ActivateAnswer::Activated {
            token_id,
            window_ends_at,
        }) => (
            StatusCode::OK,
            Json(json!({ "token_id": token_id, "window_ends_at": window_ends_at.epoch_millis() })),
        )
            .into_response(),
        Ok(ActivateAnswer::AlreadyActive) => refused(StatusCode::CONFLICT, "already_active"),
        Ok(ActivateAnswer::NoneHeld) => refused(StatusCode::CONFLICT, "none_held"),
        Err(error) => failed(&error),
    }
}

/// A chest id from the path: ASCII digits alone, at most an `i64`, and above zero; any other text,
/// a sign included, is no id.
fn chest_id(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok().filter(|id| *id > 0)
}

/// The inventory as JSON: a sealed or vaulted chest's rarity and payout are null, as the use case
/// answers them, and a study day is its ISO date.
fn inventory_body(inventory: &Inventory) -> Value {
    let chests: Vec<Value> = inventory
        .chests
        .iter()
        .map(|chest| {
            json!({
                "chest_id": chest.chest_id,
                "state": chest.state,
                "study_day": chest.study_day.to_string(),
                "rarity": chest.rarity,
                "xp": chest.xp,
            })
        })
        .collect();
    json!({
        "chests": chests,
        "tokens_held": inventory.tokens_held,
        "window_ends_at": inventory.window_ends_at.map(UtcMillis::epoch_millis),
    })
}

/// A refusal the use case answered, as `{"reason": code}`.
fn refused(status: StatusCode, code: &str) -> Response {
    (status, Json(json!({ "reason": code }))).into_response()
}

/// A failed read or write: 500, naming no chest.
fn failed(error: &KernelError) -> Response {
    tracing::error!(%error, "the owner's chests could not be read or changed");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}
