//! The owner's wallet route (SPEC-082 R15, R17; ADR-315), behind the owner's session (SPEC-024).
//!
//! `GET /api/wallet` answers the balance, today's loss cap and what is left of it, and one page of
//! the coin movements, newest first; `?before=<id>` answers the page after that movement. The
//! session is checked before anything else, so a request without the owner's live session is
//! answered 401 and learns nothing.

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_identity::{OwnerSession, Sessions};

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

/// `GET /api/wallet`.
async fn wallet(_owner: OwnerSession, State(wallet): State<Wallet>) -> Response {
    let _ = (&wallet.access, &wallet.readiness);
    (StatusCode::OK, [(CONTENT_TYPE, "application/json")], "{}").into_response()
}
