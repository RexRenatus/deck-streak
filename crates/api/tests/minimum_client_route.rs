//! The sync service states the oldest client level it accepts, to any caller (SPEC-374 R2, A6;
//! ADR-385).
//!
//! The router is built with no owner configured, as a client that has not logged in reaches it,
//! and the expected body is written here from the SPEC, never read from the route.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use deck_streak_api::{ApiState, Readiness, router};
use tower::ServiceExt;

/// The most a statement's body may hold; the statement is a few dozen bytes.
const BODY_READ_LIMIT: usize = 4096;

#[tokio::test]
async fn the_service_states_its_minimum_client_level_to_any_caller() {
    let app = router(ApiState::new(Readiness::new()));
    let request = Request::get("/api/sync/minimum-client")
        .body(Body::empty())
        .expect("a well-formed request");

    let response = app
        .oneshot(request)
        .await
        .expect("the router is infallible");

    let status = response.status();
    let header_text = |name: header::HeaderName| {
        response
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    let content_type = header_text(header::CONTENT_TYPE);
    let cache_control = header_text(header::CACHE_CONTROL);
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    let body = String::from_utf8(bytes.to_vec()).expect("a UTF-8 body");
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::OK, r#"{"minimum_client_level":1}"#),
        "the service answers its minimum client level, exactly"
    );
    assert_eq!(
        content_type.as_deref(),
        Some("application/json"),
        "the statement is JSON"
    );
    assert_eq!(
        cache_control.as_deref(),
        Some("no-store"),
        "no cache keeps a statement the service has since raised"
    );
}
