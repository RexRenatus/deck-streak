//! The owner's chest and token routes over the shell's layers (SPEC-411 A13 to A16; R13): each
//! answers the owner's session only, each POST refuses a cross-site request and writes nothing, a
//! replayed open pays once, and every outcome R13 names answers its status and code.
//!
//! The launch payload is SPEC-024's, as in the decks kept away from AI's test. Every chest, token,
//! day and instant is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Db, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

/// The synthetic bot token Python signed the payload for.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// The synthetic owner.
const OWNER: i64 = 4242;
/// The owner's launch data, signed for [`BOT_TOKEN`].
const OWNER_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=79502d49032542c5030e80b666d03f5af53e053f97d387806adb0aa843ddeb2d",
);
/// Ten seconds after the payload was signed, in milliseconds: the clock every route reads.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The study day before the one [`STARTED_AT`] falls on, when every seeded chest was earned.
const YESTERDAY: i64 = 20_102;
/// [`YESTERDAY`] as the routes write a study day.
const YESTERDAY_ISO: &str = "2025-01-14";
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 256 * 1024;
/// The inventory's path.
const CHESTS: &str = "/api/chests";
/// The token activation's path.
const ACTIVATE: &str = "/api/tokens/activate";

/// An answer: its status and its body.
struct Answer {
    status: StatusCode,
    body: String,
}

impl Answer {
    /// The body as JSON, or `null` when it is not JSON: an answer with no JSON body then fails the
    /// assertion that compares it, beside its status, rather than this helper.
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }

    /// The status and the body as JSON, the pair each outcome is judged by.
    fn read(&self) -> (StatusCode, Value) {
        (self.status, self.json())
    }
}

/// `method` on `path` with `headers` and `body`, through the whole app.
async fn send(
    app: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: String,
) -> (Answer, Option<String>) {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(body))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router is infallible");
    let status = response.status();
    let cookie = response
        .headers()
        .get(SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_owned());
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    (
        Answer {
            status,
            body: String::from_utf8(bytes.to_vec()).expect("a UTF-8 body"),
        },
        cookie,
    )
}

/// The owner's session cookie, from a same-origin JSON handshake: its `name=value`.
async fn owner_cookie(app: &Router) -> String {
    let (answer, cookie) = send(
        app,
        "POST",
        "/api/session",
        &[
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
        ],
        json!({ "init_data": OWNER_PAYLOAD }).to_string(),
    )
    .await;
    assert_eq!(answer.status, StatusCode::OK, "the owner's handshake");
    cookie.expect("a Set-Cookie")
}

/// `GET /api/chests`, carrying `cookie` when given.
async fn inventory(app: &Router, cookie: Option<&str>) -> Answer {
    let headers: Vec<(&str, &str)> = cookie
        .map(|cookie| ("cookie", cookie))
        .into_iter()
        .collect();
    send(app, "GET", CHESTS, &headers, String::new()).await.0
}

/// A JSON POST to `path` with `body`, from `site`, carrying `cookie` when given.
async fn post(app: &Router, path: &str, body: &str, site: &str, cookie: Option<&str>) -> Answer {
    let mut headers = vec![
        ("content-type", "application/json"),
        ("sec-fetch-site", site),
    ];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(app, "POST", path, &headers, body.to_owned()).await.0
}

/// `POST /api/chests/{id}/open`, same-origin, carrying `cookie` when given.
async fn open(app: &Router, id: &str, cookie: Option<&str>) -> Answer {
    post(
        app,
        &format!("/api/chests/{id}/open"),
        "",
        "same-origin",
        cookie,
    )
    .await
}

/// `POST /api/chests/{id}/pick` with `choice`, same-origin, carrying `cookie` when given.
async fn pick(app: &Router, id: &str, choice: &str, cookie: Option<&str>) -> Answer {
    let body = json!({ "choice": choice }).to_string();
    post(
        app,
        &format!("/api/chests/{id}/pick"),
        &body,
        "same-origin",
        cookie,
    )
    .await
}

/// `POST /api/tokens/activate`, same-origin, carrying `cookie` when given.
async fn activate(app: &Router, cookie: Option<&str>) -> Answer {
    post(app, ACTIVATE, "", "same-origin", cookie).await
}

/// The API over a temporary database, and the database, kept to seed and read.
async fn app(scratch: &TempDir) -> (Router, Db) {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let readiness = Readiness::new();
    readiness.database_opened(db.clone());
    (with_readiness(readiness), db)
}

/// The API over `readiness`, the owner's access on the synthetic clock.
fn with_readiness(readiness: Readiness) -> Router {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    router(ApiState::new(readiness).with_owner(access))
}

/// Stores a session chest of [`YESTERDAY`] started at `start`, with its rarity, payout and state.
async fn seed_chest(db: &Db, start: i64, rarity: &str, xp: i64, state: &str) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO chests (study_day, origin, session_start, rarity, payout_xp, state, \
         created_at) VALUES (?1, 'session', ?2, ?3, ?4, ?5, 0)",
    )
    .bind(YESTERDAY)
    .bind(start)
    .bind(rarity)
    .bind(xp)
    .bind(state)
    .execute(&mut *write)
    .await
    .expect("a chest stored");
    write.commit().await.expect("the chest committed");
}

/// Stores one held token of no chest.
async fn seed_token(db: &Db) {
    let mut write = db.write().await.expect("a write");
    sqlx::query("INSERT INTO xp_tokens (chest_id, granted_at, created_at) VALUES (0, 1, 1)")
        .execute(&mut *write)
        .await
        .expect("a token stored");
    write.commit().await.expect("the token committed");
}

/// What the routes wrote: every chest's (id, state, choice), every ledger grant's (study day,
/// source, amount) and every token's (id, activated at).
type Written = (
    Vec<(i64, String, String)>,
    Vec<(i64, String, i64)>,
    Vec<(i64, i64)>,
);

async fn written(db: &Db) -> Written {
    let mut write = db.write().await.expect("a write");
    let chests = sqlx::query_as("SELECT id, state, choice FROM chests ORDER BY id")
        .fetch_all(&mut *write)
        .await
        .expect("the chests read");
    let grants = sqlx::query_as("SELECT study_day, source, amount FROM xp_ledger ORDER BY id")
        .fetch_all(&mut *write)
        .await
        .expect("the ledger reads");
    let tokens = sqlx::query_as("SELECT id, activated_at FROM xp_tokens ORDER BY id")
        .fetch_all(&mut *write)
        .await
        .expect("the tokens read");
    (chests, grants, tokens)
}

fn chest(id: i64, state: &str, choice: &str) -> (i64, String, String) {
    (id, state.to_owned(), choice.to_owned())
}

fn reason(code: &str) -> Value {
    json!({ "reason": code })
}

#[tokio::test]
async fn the_chest_routes_answer_only_the_owner() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let (app, db) = app(&scratch).await;
    seed_chest(&db, 1_000, "rare", 45, "sealed").await;
    seed_chest(&db, 2_000, "epic", 0, "opened").await;
    seed_token(&db).await;
    let signed_out = [
        inventory(&app, None).await.status,
        open(&app, "1", None).await.status,
        pick(&app, "2", "token", None).await.status,
        activate(&app, None).await.status,
    ];
    let after_signed_out = written(&db).await;
    let cookie = owner_cookie(&app).await;
    let owner = inventory(&app, Some(&cookie)).await;
    assert_eq!(
        (signed_out, after_signed_out, owner.read()),
        (
            [StatusCode::UNAUTHORIZED; 4],
            (
                vec![chest(1, "sealed", ""), chest(2, "opened", "")],
                vec![],
                vec![(1, 0)]
            ),
            (
                StatusCode::OK,
                json!({
                    "chests": [
                        {
                            "chest_id": 1,
                            "state": "sealed",
                            "study_day": YESTERDAY_ISO,
                            "rarity": null,
                            "xp": null,
                        },
                        {
                            "chest_id": 2,
                            "state": "opened",
                            "study_day": YESTERDAY_ISO,
                            "rarity": "epic",
                            "xp": 0,
                        },
                    ],
                    "tokens_held": 1,
                    "window_ends_at": null,
                })
            ),
        ),
        "every route answers 401 without the owner's session and writes nothing; the owner reads"
    );
}

#[tokio::test]
async fn a_cross_site_chest_post_is_refused_and_writes_nothing() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let (app, db) = app(&scratch).await;
    seed_chest(&db, 1_000, "rare", 45, "sealed").await;
    seed_chest(&db, 2_000, "epic", 0, "opened").await;
    seed_token(&db).await;
    let cookie = owner_cookie(&app).await;
    let refused = (StatusCode::FORBIDDEN, reason("cross_site_request"));
    let opened = post(&app, "/api/chests/1/open", "", "cross-site", Some(&cookie)).await;
    assert_eq!(opened.read(), refused, "a cross-site open is refused");
    let body = json!({ "choice": "token" }).to_string();
    let picked = post(
        &app,
        "/api/chests/2/pick",
        &body,
        "cross-site",
        Some(&cookie),
    )
    .await;
    assert_eq!(picked.read(), refused, "a cross-site pick is refused");
    let activated = post(&app, ACTIVATE, "", "cross-site", Some(&cookie)).await;
    assert_eq!(
        activated.read(),
        refused,
        "a cross-site activation is refused"
    );
    let after_refusals = written(&db).await;
    // The control: the same open, same-origin, opens the chest and pays it.
    let control = open(&app, "1", Some(&cookie)).await;
    assert_eq!(
        (after_refusals, control.status, written(&db).await),
        (
            (
                vec![chest(1, "sealed", ""), chest(2, "opened", "")],
                vec![],
                vec![(1, 0)]
            ),
            StatusCode::OK,
            (
                vec![chest(1, "resolved", ""), chest(2, "opened", "")],
                vec![(YESTERDAY, "chest:1".to_owned(), 45)],
                vec![(1, 0)]
            ),
        ),
        "the refused POSTs wrote nothing; the same-origin open resolves and pays the chest"
    );
}

#[tokio::test]
async fn a_replayed_open_pays_once() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let (app, db) = app(&scratch).await;
    seed_chest(&db, 1_000, "rare", 45, "sealed").await;
    let cookie = owner_cookie(&app).await;
    let first = open(&app, "1", Some(&cookie)).await;
    let replay = open(&app, "1", Some(&cookie)).await;
    assert_eq!(
        (first.read(), replay.read(), written(&db).await.1),
        (
            (
                StatusCode::OK,
                json!({ "chest_id": 1, "rarity": "rare", "xp": 45, "choice_pending": false })
            ),
            (StatusCode::CONFLICT, reason("already_opened")),
            vec![(YESTERDAY, "chest:1".to_owned(), 45)],
        ),
        "the replayed open answers already opened, and the ledger holds one grant of 45"
    );
}

/// What each step of the outcome walk answers, in order, the activated window ending at
/// `window_ends_at`.
fn expected_outcomes(window_ends_at: i64) -> Vec<(&'static str, (StatusCode, Value))> {
    vec![
        ("open a bad id", (StatusCode::BAD_REQUEST, Value::Null)),
        ("open a bad id", (StatusCode::BAD_REQUEST, Value::Null)),
        ("open a bad id", (StatusCode::BAD_REQUEST, Value::Null)),
        ("pick a bad id", (StatusCode::BAD_REQUEST, Value::Null)),
        (
            "open no chest",
            (StatusCode::NOT_FOUND, reason("no_such_chest")),
        ),
        (
            "pick no chest",
            (StatusCode::NOT_FOUND, reason("no_such_chest")),
        ),
        ("pick a Rare", (StatusCode::CONFLICT, reason("not_an_epic"))),
        ("pick gold", (StatusCode::UNPROCESSABLE_ENTITY, Value::Null)),
        ("activate none", (StatusCode::CONFLICT, reason("none_held"))),
        (
            "pick a token",
            (
                StatusCode::OK,
                json!({ "choice": "token", "capped": false, "token_id": 1 }),
            ),
        ),
        (
            "pick again",
            (StatusCode::CONFLICT, reason("already_settled")),
        ),
        (
            "activate",
            (
                StatusCode::OK,
                json!({ "token_id": 1, "window_ends_at": window_ends_at }),
            ),
        ),
        (
            "activate again",
            (StatusCode::CONFLICT, reason("already_active")),
        ),
        (
            "open an Epic",
            (
                StatusCode::OK,
                json!({ "chest_id": 3, "rarity": "epic", "xp": null, "choice_pending": true }),
            ),
        ),
        (
            "open it again",
            (StatusCode::CONFLICT, reason("already_opened")),
        ),
        (
            "open the Rare",
            (
                StatusCode::OK,
                json!({ "chest_id": 1, "rarity": "rare", "xp": 45, "choice_pending": false }),
            ),
        ),
        (
            "the inventory",
            (
                StatusCode::OK,
                json!({
                    "chests": [
                        {
                            "chest_id": 3,
                            "state": "opened",
                            "study_day": YESTERDAY_ISO,
                            "rarity": "epic",
                            "xp": 0,
                        },
                        {
                            "chest_id": 4,
                            "state": "vaulted",
                            "study_day": YESTERDAY_ISO,
                            "rarity": null,
                            "xp": null,
                        },
                    ],
                    "tokens_held": 0,
                    "window_ends_at": window_ends_at,
                }),
            ),
        ),
    ]
}

#[tokio::test]
async fn the_chest_routes_answer_each_outcome_by_its_status() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let (app, db) = app(&scratch).await;
    seed_chest(&db, 1_000, "rare", 45, "sealed").await;
    seed_chest(&db, 2_000, "epic", 0, "opened").await;
    seed_chest(&db, 3_000, "epic", 0, "sealed").await;
    seed_chest(&db, 4_000, "legendary", 150, "vaulted").await;
    let cookie = owner_cookie(&app).await;
    let owner = Some(cookie.as_str());
    let mut outcomes = Vec::new();
    for id in ["abc", "0", "-1"] {
        outcomes.push(("open a bad id", open(&app, id, owner).await.read()));
    }
    outcomes.push((
        "pick a bad id",
        pick(&app, "abc", "token", owner).await.read(),
    ));
    outcomes.push(("open no chest", open(&app, "99", owner).await.read()));
    outcomes.push((
        "pick no chest",
        pick(&app, "99", "token", owner).await.read(),
    ));
    outcomes.push(("pick a Rare", pick(&app, "1", "token", owner).await.read()));
    outcomes.push((
        "pick gold",
        (pick(&app, "2", "gold", owner).await.status, Value::Null),
    ));
    outcomes.push(("activate none", activate(&app, owner).await.read()));
    outcomes.push(("pick a token", pick(&app, "2", "token", owner).await.read()));
    outcomes.push(("pick again", pick(&app, "2", "freeze", owner).await.read()));
    outcomes.push(("activate", activate(&app, owner).await.read()));
    outcomes.push(("activate again", activate(&app, owner).await.read()));
    outcomes.push(("open an Epic", open(&app, "3", owner).await.read()));
    outcomes.push(("open it again", open(&app, "3", owner).await.read()));
    outcomes.push(("open the Rare", open(&app, "1", owner).await.read()));
    outcomes.push(("the inventory", inventory(&app, owner).await.read()));
    assert_eq!(
        outcomes,
        expected_outcomes(STARTED_AT + 7_200_000),
        "each outcome answers its status and code"
    );

    // An API whose database never opened answers 503 on every route.
    let closed = with_readiness(Readiness::new());
    let cookie = owner_cookie(&closed).await;
    let owner = Some(cookie.as_str());
    let unavailable = [
        inventory(&closed, owner).await.status,
        open(&closed, "1", owner).await.status,
        pick(&closed, "1", "token", owner).await.status,
        activate(&closed, owner).await.status,
    ];
    assert_eq!(
        unavailable,
        [StatusCode::SERVICE_UNAVAILABLE; 4],
        "every route answers 503 until the database opens"
    );
}
