//! The owner's law block route (SPEC-077 R14, R16), behind the owner's session (SPEC-024).
//!
//! `GET /api/law` answers the law block for the owner's study day: whether it is shown, whether
//! its lifetime XP's line names the level, the keys of the lines it shows in order, and each count.
//! A count the recompute has not stored, or a port that is not wired, is `null` beside its
//! `pending` flag, never 0. The leech port is not wired yet (#133), so the leeches and the mastery
//! pillar are pending.

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_coordination::law::{LawLine, law_block};
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::KernelError;
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The law block.
pub const LAW_PATH: &str = "/api/law";

/// What the law route reads with.
#[derive(Clone)]
struct Law {
    access: OwnerAccess,
    readiness: Readiness,
}

impl FromRef<Law> for Sessions {
    fn from_ref(law: &Law) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&law.access)
    }
}

/// The route over `access` and `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    Router::new()
        .route(LAW_PATH, get(law))
        .with_state(Law { access, readiness })
}

/// `GET /api/law`: the block for the owner's study day, the leeches pending while the leech port
/// is not wired (#133).
async fn law(_owner: OwnerSession, State(law): State<Law>) -> Response {
    let Some(db) = law.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    match law_block(db, law.access.study_day(), None).await {
        Ok(block) => {
            let lines = block.lines();
            let keys: Vec<&'static str> = lines
                .as_ref()
                .map(|shown| shown.lines.iter().map(|line| key(*line)).collect())
                .unwrap_or_default();
            answer(&json!({
                "shown": lines.is_some(),
                "level_shown": lines.as_ref().is_some_and(|shown| shown.level_shown),
                "lines": keys,
                "streak": block.streak,
                "xp_today": block.xp_today,
                "total_xp": block.total_xp,
                "level": block.level,
                "dues": block.dues,
                "dues_pending": block.dues.is_none(),
                "leeches": block.leech_active,
                "leeches_pending": block.leech_active.is_none(),
                "mastery": block.mastery,
                "mastery_pending": block.mastery.is_none(),
            }))
        }
        Err(error) => unreadable(&error, "law_unreadable"),
    }
}

/// The key a shown line answers under: the name of the count it shows.
const fn key(line: LawLine) -> &'static str {
    match line {
        LawLine::TotalXp => "total_xp",
        LawLine::Streak => "streak",
        LawLine::XpToday => "xp_today",
        LawLine::Dues => "dues",
        LawLine::Mastery => "mastery",
        LawLine::Leeches => "leeches",
    }
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
    tracing::warn!(reason, "a law block read was refused");
    let body = json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A read that failed: 500 with its reason code, and the error only in the log.
fn unreadable(error: &KernelError, reason: &'static str) -> Response {
    tracing::error!(%error, reason, "a law block read failed");
    refused(StatusCode::INTERNAL_SERVER_ERROR, reason)
}
