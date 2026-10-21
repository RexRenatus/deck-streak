//! The owner's wallet route (SPEC-082 R15, R17; ADR-315), behind the owner's session (SPEC-024).
//!
//! `GET /api/wallet` answers the balance, today's loss cap and what is left of it, and one page of
//! the coin movements, newest first; `?before=<id>` answers the page after that movement. The
//! session is checked before anything else, so a request without the owner's live session is
//! answered 401 and learns nothing.

use axum::Router;
use axum::extract::{FromRef, RawQuery, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_coordination::wallet_view::wallet_view;
use deck_streak_identity::{OwnerSession, Sessions};
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The wallet view.
pub const WALLET_PATH: &str = "/api/wallet";

/// What the wallet route shares.
#[derive(Clone)]
struct Wallet {
    access: OwnerAccess,
    readiness: Readiness,
}

impl FromRef<Wallet> for Sessions {
    fn from_ref(wallet: &Wallet) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&wallet.access)
    }
}

/// The wallet route over `access` and `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    Router::new()
        .route(WALLET_PATH, get(wallet))
        .with_state(Wallet { access, readiness })
}

/// The query key that carries the cursor.
const BEFORE: &str = "before";

/// `GET /api/wallet`.
async fn wallet(
    _owner: OwnerSession,
    State(wallet): State<Wallet>,
    RawQuery(query): RawQuery,
) -> Response {
    let Ok(before) = cursor(query.as_deref()) else {
        return refused(StatusCode::BAD_REQUEST, "invalid_cursor");
    };
    let Some(db) = wallet.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    match wallet_view(db, wallet.access.study_day(), before).await {
        Ok(view) => {
            let movements: Vec<Value> = view
                .page
                .movements
                .iter()
                .map(|movement| {
                    json!({
                        "id": movement.id,
                        "study_day": movement.day.to_string(),
                        "source": movement.source,
                        "amount": movement.delta,
                    })
                })
                .collect();
            answer(&json!({
                "study_day": view.study_day.to_string(),
                "balance": view.balance,
                "loss_cap": view.loss_cap,
                "loss_cap_left": view.loss_cap_left,
                "movements": movements,
                "next": view.page.next,
            }))
        }
        Err(error) => unreadable(&error),
    }
}

/// The cursor `query` carries: `None` when there is no query or it has no `before`, the movement's
/// id when `before` is a whole number, and an error for any other `before`. The API's axum has no
/// `query` feature, so the raw query is read here, by hand.
fn cursor(query: Option<&str>) -> Result<Option<i64>, std::num::ParseIntError> {
    let Some(query) = query else {
        return Ok(None);
    };
    query
        .split('&')
        .find_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key == BEFORE).then_some(value)
        })
        .map(str::parse::<i64>)
        .transpose()
}

/// 200 with `body` as JSON.
fn answer(body: &Value) -> Response {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// A refusal: its status, and a JSON body naming its reason code alone.
fn refused(status: StatusCode, reason: &'static str) -> Response {
    tracing::warn!(reason, "a wallet read was refused");
    let body = json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A read that failed: 500 with its reason code, and the error only in the log.
fn unreadable(error: &deck_streak_kernel::KernelError) -> Response {
    tracing::error!(%error, "a wallet read failed");
    refused(StatusCode::INTERNAL_SERVER_ERROR, "wallet_unreadable")
}
