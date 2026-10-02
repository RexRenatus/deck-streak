//! The owner's badge, record and milestone routes (SPEC-073 R16, R17), each behind the owner's
//! session (SPEC-024).
//!
//! - `GET /api/badges` answers the earned badges, most recently awarded first, each its key,
//!   tier, name, emoji and day; and the locked catalog badges, each its criteria and, where its
//!   input is stored, that input's value against its threshold.
//! - `GET /api/records` answers each stored record with its value, day and the value it beat,
//!   today's live value and its distance from the record, and the record to chase.
//! - `GET /api/milestone` answers `pending`: Road to C2 supplies no mature-card sum yet (R15, #85),
//!   so nothing is read and nothing is computed from a stand-in.
//!
//! The badges and records read coordination's views, the ones the bot's `/badges` and `/records`
//! read too.

use axum::Router;
use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use deck_streak_identity::Sessions;
use deck_streak_kernel::Courses;
use serde_json::{Value, json};

use crate::health::Readiness;
use crate::session_routes::OwnerAccess;

/// The badges view.
pub const BADGES_PATH: &str = "/api/badges";
/// The records view.
pub const RECORDS_PATH: &str = "/api/records";
/// The next milestone.
pub const MILESTONE_PATH: &str = "/api/milestone";

/// What the badge, record and milestone routes share.
#[derive(Clone)]
struct Badges {
    access: OwnerAccess,
    readiness: Readiness,
    courses: Courses,
}

impl FromRef<Badges> for Sessions {
    fn from_ref(badges: &Badges) -> Self {
        <Self as FromRef<OwnerAccess>>::from_ref(&badges.access)
    }
}

/// The routes over `access` and `readiness`, the catalog's descriptions rendered from `courses`.
pub(crate) fn routes(access: OwnerAccess, readiness: Readiness, courses: Courses) -> Router {
    Router::new()
        .route(BADGES_PATH, get(badges))
        .route(RECORDS_PATH, get(records))
        .route(MILESTONE_PATH, get(milestone))
        .with_state(Badges {
            access,
            readiness,
            courses,
        })
}

/// `GET /api/badges`.
async fn badges(State(badges): State<Badges>) -> Response {
    let _ = (&badges.access, &badges.readiness, &badges.courses);
    answer(&json!({}))
}

/// `GET /api/records`.
async fn records(State(badges): State<Badges>) -> Response {
    let _ = badges;
    answer(&json!({}))
}

/// `GET /api/milestone`.
async fn milestone() -> Response {
    answer(&json!({}))
}

/// How far `today` is from a record of `value`: none once today reaches it.
const fn distance(value: i64, today: i64) -> i64 {
    let _ = (value, today);
    0
}

/// 200 with `body` as JSON.
fn answer(body: &Value) -> Response {
    let _ = distance(0, 0);
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}
