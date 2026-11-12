//! The owner's instruments routes (SPEC-094 R10, R12; ADR-094, ADR-085), each behind the owner's
//! session (SPEC-024).
//!
//! - `GET /api/insights` lists each live instrument with its cadence and the study day of its
//!   stored report; an instrument no run has reported is `null`, never a zero.
//! - `GET /api/insights/{id}` answers one instrument's stored report as the numbers it holds; the
//!   report is `null` while none is stored. The API serves numbers, never an image (ADR-085).
//! - `POST /api/insights/{id}/run` runs the instrument now, through the same lock as the weekly
//!   step. A run already going is answered 409 and starts nothing (ADR-094).
//!
//! The session is checked before anything else, so a caller without the owner's live session is
//! answered 401 and learns nothing, not even which instruments exist. A run changes state, so a
//! cross-site request is refused 403 before the run starts.

use std::sync::Arc;

use axum::Router;
use axum::extract::{FromRef, Path, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use deck_streak_coordination::instruments::{InstrumentService, OnDemandRefusal};
use deck_streak_identity::{OwnerSession, Sessions};
use serde_json::{Value, json};

use crate::session_routes::OwnerAccess;

/// The instruments' list.
pub const LIST_PATH: &str = "/api/insights";
/// One instrument's stored report.
pub const REPORT_PATH: &str = "/api/insights/{id}";
/// One instrument's on-demand run.
pub const RUN_PATH: &str = "/api/insights/{id}/run";

/// What the routes share: the owner's access and the instruments.
#[derive(Clone)]
struct Insights {
    access: OwnerAccess,
    service: Arc<dyn InstrumentService>,
}

impl FromRef<Insights> for Sessions {
    fn from_ref(insights: &Insights) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&insights.access)
    }
}

/// The instruments routes over `access` and `service`.
pub(crate) fn routes(access: OwnerAccess, service: Arc<dyn InstrumentService>) -> Router {
    Router::new()
        .route(LIST_PATH, get(list))
        .route(REPORT_PATH, get(report))
        .route(RUN_PATH, post(run))
        .with_state(Insights { access, service })
}

/// `GET /api/insights`: each live instrument.
async fn list(_owner: OwnerSession, State(insights): State<Insights>) -> Response {
    match insights.service.list().await {
        Ok(listings) => {
            let instruments: Vec<Value> = listings
                .iter()
                .map(|listing| {
                    json!({
                        "id": listing.id,
                        "cadence": listing.cadence,
                        "study_day": listing.study_day,
                    })
                })
                .collect();
            answer(&json!({ "instruments": instruments }))
        }
        Err(error) => unreadable(&error),
    }
}

/// `GET /api/insights/{id}`: the stored report, or `null`.
async fn report(
    _owner: OwnerSession,
    State(insights): State<Insights>,
    Path(id): Path<String>,
) -> Response {
    let listed = match insights.service.list().await {
        Ok(listings) => listings.iter().any(|listing| listing.id == id),
        Err(error) => return unreadable(&error),
    };
    if !listed {
        return refused(StatusCode::NOT_FOUND, "no_such_instrument");
    }
    match insights.service.report(&id).await {
        Ok(stored) => answer(&json!({ "instrument": id, "report": stored.map(|s| s.report) })),
        Err(error) => unreadable(&error),
    }
}

/// `POST /api/insights/{id}/run`: runs it now.
async fn run(
    _owner: OwnerSession,
    State(insights): State<Insights>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let cross_site = headers
        .get("sec-fetch-site")
        .is_some_and(|site| site != "same-origin");
    if cross_site {
        return refused(StatusCode::FORBIDDEN, "cross_site_request");
    }
    match insights.service.run(&id).await {
        Ok(stored) => answer(&json!({ "instrument": id, "report": stored.report })),
        Err(OnDemandRefusal::Unknown) => refused(StatusCode::NOT_FOUND, "no_such_instrument"),
        Err(OnDemandRefusal::InProgress) => refused(StatusCode::CONFLICT, "run_in_progress"),
        Err(refusal) => {
            tracing::error!(%refusal, "an instrument run could not start");
            refused(StatusCode::INTERNAL_SERVER_ERROR, "run_unavailable")
        }
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
    let body = json!({ "reason": reason }).to_string();
    (status, [(CONTENT_TYPE, "application/json")], body).into_response()
}

/// A read that failed: logged with its cause, answered 500 with a reason code alone.
fn unreadable(error: &dyn std::fmt::Display) -> Response {
    tracing::error!(%error, "the instruments could not be read");
    refused(StatusCode::INTERNAL_SERVER_ERROR, "instruments_unreadable")
}
