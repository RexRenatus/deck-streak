//! The law-drill routes (SPEC-110 R9, A19): the owner's Mini App lists the drills, reads one, and
//! answers one, to the owner's live session alone. The answer goes through the vault contract's one
//! writer, `coordination::drills::answer`, so the Mini App, the bot and a re-poll never write twice.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{FromRef, Path, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use deck_streak_coordination::drills::{
    self, AnswerOutcome, DrillMeta, DrillNotes, RealFs, Surface,
};
use deck_streak_identity::{OwnerSession, Sessions};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::{OwnerAccess, StateChange};

/// The list's path.
pub const LIST_PATH: &str = "/api/drills";

/// What the drill routes read.
#[derive(Clone)]
struct Drills {
    access: OwnerAccess,
    readiness: Readiness,
    notes: Option<Arc<DrillNotes<RealFs>>>,
}

impl FromRef<Drills> for Sessions {
    fn from_ref(state: &Drills) -> Self {
        Self::from_ref(&state.access)
    }
}

/// The drill routes over the owner's `access`, the API's `readiness` and the vault's `notes`.
pub(crate) fn routes(
    access: OwnerAccess,
    readiness: Readiness,
    notes: Option<Arc<DrillNotes<RealFs>>>,
) -> Router {
    Router::new()
        .route(LIST_PATH, get(list))
        .route("/api/drills/{id}", get(view))
        .route("/api/drills/{id}/answer", post(answer))
        .with_state(Drills {
            access,
            readiness,
            notes,
        })
}

/// A JSON answer of `status`.
fn json_of(status: StatusCode, body: &Value) -> Response {
    (
        status,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// A refusal of `status` naming its `reason`.
fn refused(status: StatusCode, reason: &str) -> Response {
    json_of(status, &json!({ "reason": reason }))
}

/// One drill's metadata as the Mini App reads it.
fn meta_json(meta: &DrillMeta) -> Value {
    json!({
        "drill_id": meta.drill_id,
        "type": meta.kind,
        "subject": meta.subject,
        "title": meta.title,
        "created": meta.created.map(|day| day.to_string()),
        "age_days": meta.age_days,
        "answered": meta.answered,
        "deferred": meta.deferred,
    })
}

/// `GET /api/drills`: every Active drill, and the counts the queue shows.
async fn list(_owner: OwnerSession, State(state): State<Drills>) -> Response {
    let Some(notes) = state.notes else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "vault_not_open");
    };
    match notes.list_active(state.access.study_day()) {
        Ok(all) => {
            let awaiting = all.iter().filter(|meta| meta.answered).count();
            let deferred = all
                .iter()
                .filter(|meta| meta.answered && meta.deferred)
                .count();
            let unanswered: Vec<Value> = all
                .iter()
                .filter(|meta| !meta.answered)
                .map(meta_json)
                .collect();
            json_of(
                StatusCode::OK,
                &json!({
                    "drills": unanswered,
                    "awaiting_grading": awaiting,
                    "deferred": deferred,
                }),
            )
        }
        Err(error) => {
            tracing::error!(%error, "the drills could not be listed");
            refused(StatusCode::INTERNAL_SERVER_ERROR, "vault_unreadable")
        }
    }
}

/// `GET /api/drills/{id}`: one Active drill's view, or 404.
async fn view(
    _owner: OwnerSession,
    State(state): State<Drills>,
    Path(id): Path<String>,
) -> Response {
    let Some(notes) = state.notes else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "vault_not_open");
    };
    match notes.view(&id, state.access.study_day()) {
        Some(view) => {
            let mut meta = meta_json(&view.meta);
            if let Some(fields) = meta.as_object_mut() {
                fields.insert("prompt".to_owned(), json!(view.prompt));
                fields.insert("defer_reason".to_owned(), json!(view.defer_reason));
                fields.insert("sections".to_owned(), json!(view.sections));
                fields.insert("self_check".to_owned(), json!(view.self_check));
            }
            json_of(StatusCode::OK, &meta)
        }
        None => refused(StatusCode::NOT_FOUND, "no_such_drill"),
    }
}

/// The answer route's body.
#[derive(Deserialize)]
struct AnswerBody {
    answer: String,
}

/// `POST /api/drills/{id}/answer`: appends the answer through the vault contract's one writer.
async fn answer(
    _owner: OwnerSession,
    _change: StateChange,
    State(state): State<Drills>,
    Path(id): Path<String>,
    Json(body): Json<AnswerBody>,
) -> Response {
    let Some(notes) = state.notes else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "vault_not_open");
    };
    let Some(db) = state.readiness.database() else {
        return refused(StatusCode::SERVICE_UNAVAILABLE, "database_not_open");
    };
    let outcome = drills::answer(
        &notes,
        db,
        &id,
        &body.answer,
        Surface::MiniApp,
        state.access.rule(),
        state.access.clock().now(),
    )
    .await;
    match outcome {
        Ok(AnswerOutcome::Appended { title }) => {
            json_of(StatusCode::OK, &json!({ "answered": true, "title": title }))
        }
        Ok(AnswerOutcome::EmptyAnswer) => refused(StatusCode::UNPROCESSABLE_ENTITY, "empty_answer"),
        Ok(AnswerOutcome::NotActive) => refused(StatusCode::NOT_FOUND, "no_such_drill"),
        Ok(AnswerOutcome::AlreadyAnswered) => refused(StatusCode::CONFLICT, "already_answered"),
        Ok(AnswerOutcome::RailRefused) => refused(StatusCode::UNPROCESSABLE_ENTITY, "rail_refused"),
        Err(error) => {
            tracing::error!(%error, "a drill answer could not be written");
            refused(StatusCode::INTERNAL_SERVER_ERROR, "vault_unwritable")
        }
    }
}
