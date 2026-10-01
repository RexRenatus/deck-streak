//! The owner's streak and governor routes (SPEC-076 R20, R21), each behind the owner's session
//! (SPEC-024): the views the bot's `/streak` and the Mini App's streak screen read.
//!
//! Both read coordination's views, the ones the bot reads too. The session is checked first, so a
//! request without the owner's live session is answered 401 and learns nothing.

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_coordination::streak_views::{governor_view, streak_view};
use deck_streak_identity::{OwnerSession, Sessions};
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The streak view.
pub const STREAK_PATH: &str = "/api/streak";
/// The governor view.
pub const GOVERNOR_PATH: &str = "/api/governor";

/// What the streak routes share.
#[derive(Clone)]
struct Streaks {
    access: OwnerAccess,
    readiness: Readiness,
}

impl FromRef<Streaks> for Sessions {
    fn from_ref(streaks: &Streaks) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&streaks.access)
    }
}

/// The streak routes over `access` and `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    Router::new()
        .route(STREAK_PATH, get(streak))
        .route(GOVERNOR_PATH, get(governor))
        .with_state(Streaks { access, readiness })
}

/// A track's counts.
fn track_json(current: u32, longest: u32, heat: u32) -> serde_json::Map<String, Value> {
    let mut track = serde_json::Map::new();
    track.insert("current".to_owned(), json!(current));
    track.insert("longest".to_owned(), json!(longest));
    track.insert("heat".to_owned(), json!(heat));
    track
}

/// `GET /api/streak`.
async fn streak(_owner: OwnerSession, State(streaks): State<Streaks>) -> Response {
    let Some(db) = streaks.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    match streak_view(db, streaks.access.study_day()).await {
        Ok(view) => {
            let mut language = track_json(
                view.language.current,
                view.language.longest,
                view.language_tier,
            );
            language.insert("freezes".to_owned(), json!(view.language.freezes));
            language.insert("freeze_cap".to_owned(), json!(view.freeze_cap));
            answer(&json!({
                "study_day": view.study_day.to_string(),
                "language": language,
                "law": track_json(view.law.current, view.law.longest, view.law_tier),
                "at_stake": {
                    "language": view.language_at_stake.as_str(),
                    "law": view.law_at_stake.as_str(),
                },
            }))
        }
        Err(error) => unreadable(&error),
    }
}

/// `GET /api/governor`.
async fn governor(_owner: OwnerSession, State(streaks): State<Streaks>) -> Response {
    let Some(db) = streaks.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    match governor_view(db).await {
        Ok(view) => answer(&json!({
            "verdict": view.verdict(),
            "strength": view.strength,
            "standby": view.standby,
            "lapse": view.lapse,
            "lapse_since": view.lapse_since.map(|day| day.to_string()),
            "relight_cards": view.lapse.then_some(view.relight_reviews),
        })),
        Err(error) => unreadable(&error),
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
    tracing::warn!(reason, "a streak read was refused");
    let body = json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A read that failed: 500 with its reason code, and the error only in the log.
fn unreadable(error: &deck_streak_kernel::KernelError) -> Response {
    tracing::error!(%error, "a streak read failed");
    refused(StatusCode::INTERNAL_SERVER_ERROR, "streak_unreadable")
}
