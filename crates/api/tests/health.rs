//! The health routes: liveness answers while the process runs, readiness only once the database is
//! open and migrated, and the router that serves them builds on axum 0.8's path syntax (SPEC-025
//! A1 to A3, R2, R3).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use deck_streak_api::{ApiState, Readiness, router};
use deck_streak_kernel::Db;
use tower::ServiceExt;

/// The release version both bodies carry: the workspace's, as Cargo gives it to this test too.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The most a health body may hold; a health body is a few dozen bytes.
const BODY_READ_LIMIT: usize = 4096;

/// A GET of `path`: its status, its content type and its body.
async fn get(app: Router, path: &str) -> (StatusCode, Option<String>, String) {
    let request = Request::get(path)
        .body(Body::empty())
        .expect("a well-formed request");
    let response = app
        .oneshot(request)
        .await
        .expect("the router is infallible");
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    let body = String::from_utf8(bytes.to_vec()).expect("a UTF-8 body");
    (status, content_type, body)
}

/// The body a health route answers with `word`: its status word and the release version, and
/// nothing else.
fn health_body(word: &str) -> String {
    format!("{{\"status\":\"{word}\",\"version\":\"{VERSION}\"}}")
}

#[test]
fn the_version_constant_is_the_release_version_the_bodies_carry() {
    assert_eq!(deck_streak_api::health::VERSION, VERSION);
}

#[tokio::test]
async fn livez_answers_while_the_process_runs() {
    // The database is not open: liveness does not depend on it.
    let app = router(ApiState::new(Readiness::new()));
    let (status, content_type, body) = get(app, "/api/livez").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert_eq!(body, health_body("alive"));
}

#[tokio::test]
async fn readyz_is_false_until_the_database_is_open() {
    let readiness = Readiness::new();
    let app = router(ApiState::new(readiness.clone()));

    let (status, content_type, body) = get(app.clone(), "/api/readyz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert_eq!(body, health_body("starting"));

    // The kernel opens and migrates a real database; handing it over makes the API ready.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let database = Db::open(&directory.path().join("readiness.db"))
        .await
        .expect("the database opens and migrates");
    readiness.database_opened(database);
    assert!(
        readiness.is_ready(),
        "the opened database made no difference"
    );

    let (status, _, body) = get(app, "/api/readyz").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, health_body("ready"));
}

#[tokio::test]
async fn the_router_builds_with_axum_08_route_syntax() {
    // axum 0.8 refuses a `:` or `*` segment when a route is added, so building the router is the
    // check; it then serves each of its routes at the path the SPEC names.
    let app = router(ApiState::new(Readiness::new()));
    let routed = [
        ("/api/livez", StatusCode::OK),
        ("/api/readyz", StatusCode::SERVICE_UNAVAILABLE),
    ];
    for (path, expected) in routed {
        let (status, _, body) = get(app.clone(), path).await;
        assert_eq!(status, expected, "{path}: {body}");
    }
    // A path the router does not hold is the fallback's 404, not a health answer.
    let (status, _, _) = get(app, "/api/livez/extra").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
