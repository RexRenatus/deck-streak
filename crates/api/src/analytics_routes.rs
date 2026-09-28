//! The owner's analytics routes (SPEC-071 R10, R20; ADR-071): the rollups of a range of study days,
//! and the current study day's score, each behind the owner's session (SPEC-024).
//!
//! - `GET /api/analytics/days?from=<ISO date>&to=<ISO date>` answers the rollups of the study days
//!   from `from` to `to`, inclusive, oldest first, at most the window's length: each day's metrics,
//!   its card state and that state's provenance, its score and its settle. A range the window
//!   cannot hold is refused 400 with a reason code.
//! - `GET /api/score` answers the current study day, by the kernel's rule and clock, and its score:
//!   the total, the grade, the five pillars, the reviews and the true retention; `null` while no
//!   recompute has rolled the day up.
//!
//! Both read coordination's score reads, the one the bot's `/score` reads too, so the two surfaces
//! answer the same numbers. A card state no recompute recorded, and the retention of a day with no
//! answered review, are rendered as null, never as 0 (R10). The session is checked before anything
//! else, so a request without the owner's live session is answered 401 and learns nothing, not even
//! whether the database is open. Until it is, the owner is answered 503.

use axum::Router;
use axum::extract::{FromRef, RawQuery, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_coordination::score::{DayRollup, DayScore, RangeError, day_rollups, day_score};
use deck_streak_identity::{OwnerSession, Sessions};
use deck_streak_kernel::{Db, KernelError, StudyDay};
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The rollups of a range of study days.
pub const DAYS_PATH: &str = "/api/analytics/days";
/// The current study day's score.
pub const SCORE_PATH: &str = "/api/score";

/// What the analytics routes share: the owner's access, for the session and the study day, and the
/// readiness that holds the database once it is open.
#[derive(Clone, Debug)]
struct Analytics {
    access: OwnerAccess,
    readiness: Readiness,
}

impl FromRef<Analytics> for Sessions {
    fn from_ref(analytics: &Analytics) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&analytics.access)
    }
}

/// The analytics routes over `access` and `readiness`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness) -> Router {
    Router::new()
        .route(DAYS_PATH, get(days))
        .route(SCORE_PATH, get(score))
        .with_state(Analytics { access, readiness })
}

/// `GET /api/analytics/days`: the rollups of the range the query names.
async fn days(
    _owner: OwnerSession,
    State(analytics): State<Analytics>,
    RawQuery(query): RawQuery,
) -> Response {
    let Some(db) = analytics.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    let Some((first, last)) = query.as_deref().and_then(range) else {
        return refused(StatusCode::BAD_REQUEST, "range_invalid");
    };
    match day_rollups(db, first, last).await {
        Ok(rollups) => {
            let days: Vec<Value> = rollups.iter().map(rollup_json).collect();
            answer(&json!({ "days": days }))
        }
        Err(RangeError::Backwards) => refused(StatusCode::BAD_REQUEST, "range_invalid"),
        Err(RangeError::TooLong) => refused(StatusCode::BAD_REQUEST, "range_too_long"),
        Err(RangeError::Read(error)) => unreadable(&error),
    }
}

/// `GET /api/score`: the current study day's score.
async fn score(_owner: OwnerSession, State(analytics): State<Analytics>) -> Response {
    let Some(db) = analytics.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    let today = analytics.access.study_day();
    match current(db, today).await {
        Ok(body) => answer(&body),
        Err(error) => unreadable(&error),
    }
}

/// The body of `GET /api/score` for `today`.
async fn current(db: &Db, today: StudyDay) -> Result<Value, KernelError> {
    let score = day_score(db, today).await?;
    Ok(json!({
        "study_day": today.to_string(),
        "score": score.as_ref().map(score_json),
    }))
}

/// The range `query` names: `from` and `to`, each an ISO date, once each, and nothing else.
fn range(query: &str) -> Option<(StudyDay, StudyDay)> {
    let (mut from, mut to) = (None, None);
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=')?;
        let slot = match name {
            "from" => &mut from,
            "to" => &mut to,
            _ => return None,
        };
        if slot.replace(value.parse::<StudyDay>().ok()?).is_some() {
            return None;
        }
    }
    Some((from?, to?))
}

/// A day's score as JSON: an absent retention is null.
fn score_json(score: &DayScore) -> Value {
    json!({
        "total": score.total,
        "grade": {"label": score.grade_label, "emoji": score.grade_emoji},
        "pillars": {
            "consistency": score.pillars.consistency,
            "retention": score.pillars.retention,
            "workload": score.pillars.workload,
            "volume": score.pillars.volume,
            "mastery": score.pillars.mastery,
        },
        "reviews": score.reviews,
        "retention": score.retention,
    })
}

/// A day's rollup as JSON: its metrics, its card state (null when never recorded), that state's
/// provenance, its score and its settle.
fn rollup_json(rollup: &DayRollup) -> Value {
    let metrics = &rollup.metrics;
    let card_state = rollup.card_state.map(|state| {
        json!({
            "mature_count": state.mature_count,
            "young_count": state.young_count,
            "leech_active": state.leech_active,
            "backlog": state.backlog,
            "due_today": state.due_today,
        })
    });
    json!({
        "study_day": metrics.day.to_string(),
        "reviews": metrics.reviews,
        "learn_count": metrics.learn_count,
        "review_count": metrics.review_count,
        "relearn_count": metrics.relearn_count,
        "filtered_count": metrics.filtered_count,
        "seconds": metrics.seconds,
        "answered": metrics.answered,
        "passed": metrics.passed,
        "true_retention": rollup.score.retention,
        "graduations": metrics.graduations,
        "decks_studied": metrics.decks_studied,
        "avg_answer_seconds": metrics.avg_answer_seconds,
        "young_answered": metrics.young_answered,
        "young_passed": metrics.young_passed,
        "mature_answered": metrics.mature_answered,
        "mature_passed": metrics.mature_passed,
        "card_state": card_state,
        "card_state_src": rollup.card_state_src,
        "score": score_json(&rollup.score),
        "score_at_close": rollup.score_at_close,
        "settled_at": rollup.settled_at.map(|at| at.epoch_millis()),
    })
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
    tracing::warn!(reason, "an analytics read was refused");
    let body = json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A read that failed: logged with its cause, answered 500 with a reason code alone.
fn unreadable(error: &dyn std::fmt::Display) -> Response {
    tracing::error!(%error, "the rollups could not be read");
    refused(StatusCode::INTERNAL_SERVER_ERROR, "rollups_unreadable")
}
