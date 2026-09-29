//! The owner's level routes (SPEC-072 R23, R24), each behind the owner's session (SPEC-024).
//!
//! - `GET /api/level` answers the level, its title and emoji, the total, the XP into the level and
//!   the XP the level spans; today's XP by source and track, each marked `provisional` (the open
//!   day's settled XP) or `settled`; the run, its multiplier and the one-miss preview; and whether
//!   today is an Ascendant day.
//! - `GET /api/level/law-tiers` answers the law-track cards counted by tier and today's law review
//!   XP by tier, from the source the daemon gives the API. An API given none answers 503.
//!
//! Both read coordination's level view, the one the bot's `/level` reads too. The session is
//! checked before anything else, so a request without the owner's live session is answered 401 and
//! learns nothing.

use std::sync::Arc;

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_coordination::progression::level_view::{LawTierSource, LawTiers, level_view};
use deck_streak_identity::{OwnerSession, Sessions};
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The level view.
pub const LEVEL_PATH: &str = "/api/level";
/// The law tiers view.
pub const LAW_TIERS_PATH: &str = "/api/level/law-tiers";

/// The tiers, in the order the law view answers them.
const TIER_KEYS: [&str; 5] = ["T1", "T2", "T3", "T4", "none"];

/// What the level routes share.
#[derive(Clone)]
struct Level {
    access: OwnerAccess,
    readiness: Readiness,
    tiers: Option<Arc<dyn LawTierSource>>,
}

impl FromRef<Level> for Sessions {
    fn from_ref(level: &Level) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&level.access)
    }
}

/// The level routes over `access` and `readiness`, with the law tiers' `tiers` when there are any.
pub(crate) fn routes(
    access: OwnerAccess,
    readiness: Readiness,
    tiers: Option<Arc<dyn LawTierSource>>,
) -> Router {
    Router::new()
        .route(LEVEL_PATH, get(level))
        .route(LAW_TIERS_PATH, get(law_tiers))
        .with_state(Level {
            access,
            readiness,
            tiers,
        })
}

/// `GET /api/level`.
async fn level(_owner: OwnerSession, State(level): State<Level>) -> Response {
    let Some(db) = level.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    match level_view(db, level.access.study_day()).await {
        Ok(view) => {
            let today: Vec<Value> = view
                .today
                .iter()
                .map(|row| {
                    json!({
                        "source": row.source,
                        "track": row.track,
                        "amount": row.amount,
                        "state": if row.closed { "settled" } else { "provisional" },
                    })
                })
                .collect();
            answer(&json!({
                "study_day": view.study_day.to_string(),
                "level": view.info.level.get(),
                "title": view.info.title,
                "emoji": view.info.emoji,
                "total_xp": view.info.total_xp,
                "xp_into_level": view.info.xp_into_level,
                "xp_for_next": view.info.xp_for_next,
                "today": today,
                "run": view.run,
                "multiplier": view.multiplier,
                "multiplier_after_a_miss": view.multiplier_after_a_miss,
                "ascendant": view.ascendant,
            }))
        }
        Err(error) => unreadable(&error),
    }
}

/// `GET /api/level/law-tiers`.
async fn law_tiers(_owner: OwnerSession, State(level): State<Level>) -> Response {
    let Some(source) = level.tiers.as_ref() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "law_tiers_unavailable");
    };
    match source.law_tiers(level.access.study_day()).await {
        Ok(tiers) => answer(&tiers_json(&tiers)),
        Err(error) => unreadable(&error),
    }
}

/// The law tiers as JSON: each tier's cards and today's XP.
fn tiers_json(tiers: &LawTiers) -> Value {
    let by_tier = |counts: &[u64; 5]| {
        TIER_KEYS
            .iter()
            .zip(counts)
            .map(|(key, count)| ((*key).to_owned(), json!(count)))
            .collect::<serde_json::Map<String, Value>>()
    };
    json!({ "cards": by_tier(&tiers.cards), "xp_today": by_tier(&tiers.xp) })
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
    tracing::warn!(reason, "a level read was refused");
    let body = json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A read that failed: 500 with its reason code, and the error only in the log.
fn unreadable(error: &deck_streak_kernel::KernelError) -> Response {
    tracing::error!(%error, "a level read failed");
    refused(StatusCode::INTERNAL_SERVER_ERROR, "level_unreadable")
}
