//! The bounds every route is served under: the body limit, the concurrency bound that sheds, the
//! request timeout and the panic catcher (SPEC-025 A4 to A7, R4, R5; ADR-025).
//!
//! Each test serves routes of its own through `deck_streak_api::layered`, the one stack the
//! production router is built with. The timeout and the bound are driven on tokio's paused clock,
//! which advances only when every task waits on a timer, so nothing here sleeps.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes, to_bytes};
use axum::http::{Request, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use deck_streak_api::layered;
use tokio::sync::{mpsc, watch};
use tower::ServiceExt;

/// The SPEC's body limit: axum's default for its extractors, 2 MB.
const TWO_MB: usize = 2 * 1024 * 1024;
/// The SPEC's bound on requests in flight.
const IN_FLIGHT: usize = 64;
/// The SPEC's request timeout.
const TIMEOUT: Duration = Duration::from_secs(10);
/// The most a test reads of a body it inspects.
const BODY_READ_LIMIT: usize = 4096;

fn request(method: &str, path: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .body(body)
        .expect("a well-formed request")
}

async fn send(app: Router, request: Request<Body>) -> Response {
    app.oneshot(request)
        .await
        .expect("the router is infallible")
}

async fn text(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    String::from_utf8(bytes.to_vec()).expect("a UTF-8 body")
}

#[tokio::test]
async fn a_body_over_the_limit_is_refused_with_413() {
    let app = layered(Router::new().route(
        "/api/test/length",
        post(|body: Bytes| async move { body.len().to_string() }),
    ));

    // A body at the limit is read whole.
    let at_limit = request("POST", "/api/test/length", Body::from(vec![b'a'; TWO_MB]));
    let response = send(app.clone(), at_limit).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, TWO_MB.to_string());

    // One byte more is refused before the handler reads it.
    let over = request(
        "POST",
        "/api/test/length",
        Body::from(vec![b'a'; TWO_MB + 1]),
    );
    let response = send(app, over).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test(start_paused = true)]
async fn a_request_past_the_concurrency_bound_is_shed_with_503() {
    // Every held request reports that its handler runs, then waits until the test releases it.
    let (entered, mut entries) = mpsc::channel::<()>(IN_FLIGHT + 1);
    let (release, released) = watch::channel(false);
    let held = get(move || {
        let entered = entered.clone();
        let mut released = released.clone();
        async move {
            entered.send(()).await.expect("the test is listening");
            released
                .wait_for(|released| *released)
                .await
                .expect("the test holds the sender");
            "released"
        }
    });
    let app = layered(
        Router::new()
            .route("/api/test/held", held)
            .route("/api/test/quick", get(|| async { "quick" })),
    );

    let mut in_flight = Vec::new();
    for _ in 0..IN_FLIGHT {
        let app = app.clone();
        in_flight.push(tokio::spawn(async move {
            send(app, request("GET", "/api/test/held", Body::empty())).await
        }));
    }
    for _ in 0..IN_FLIGHT {
        entries
            .recv()
            .await
            .expect("a held request reached its handler");
    }

    // The bound is full: the next request is answered 503 at once, on the paused clock, rather
    // than waiting for a slot. A queued request would still be waiting when the second passes.
    for path in ["/api/test/held", "/api/test/quick"] {
        let answered = tokio::time::timeout(
            Duration::from_secs(1),
            send(app.clone(), request("GET", path, Body::empty())),
        )
        .await;
        assert!(
            answered.is_ok(),
            "{path}: the request past the bound was held, not shed"
        );
        let status = answered.map(|response| response.status()).ok();
        // The bound is the service's, not one route's: a route with no request in flight is
        // shed too.
        assert_eq!(status, Some(StatusCode::SERVICE_UNAVAILABLE), "{path}");
    }

    // Released, every held request completes, and the service serves again.
    release.send(true).expect("the held requests are listening");
    for handle in in_flight {
        let response = handle.await.expect("a held request completes");
        assert_eq!(response.status(), StatusCode::OK);
    }
    let response = send(app, request("GET", "/api/test/quick", Body::empty())).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, "quick");
}

#[tokio::test(start_paused = true)]
async fn a_handler_past_the_timeout_answers_408() {
    let app = layered(
        Router::new()
            .route(
                "/api/test/late",
                get(|| async {
                    tokio::time::sleep(TIMEOUT + Duration::from_secs(1)).await;
                    "late"
                }),
            )
            .route(
                "/api/test/in-time",
                get(|| async {
                    tokio::time::sleep(TIMEOUT.saturating_sub(Duration::from_secs(1))).await;
                    "in time"
                }),
            ),
    );

    let started = tokio::time::Instant::now();
    let response = send(app.clone(), request("GET", "/api/test/late", Body::empty())).await;
    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    // The paused clock stops at the timeout's own deadline, not the handler's; tokio's timer wheel
    // rounds a deadline up to its next millisecond.
    let elapsed = started.elapsed();
    assert!(
        elapsed >= TIMEOUT && elapsed <= TIMEOUT + Duration::from_millis(1),
        "answered after {elapsed:?}, not at the {TIMEOUT:?} timeout"
    );

    // A handler that answers inside the timeout is answered.
    let response = send(app, request("GET", "/api/test/in-time", Body::empty())).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, "in time");
}

/// A handler that panics, as a defect in any route would.
async fn panics() -> &'static str {
    panic!("a synthetic handler panic");
}

#[tokio::test]
async fn a_panicking_handler_answers_500_and_the_service_keeps_serving() {
    let app = layered(
        Router::new()
            .route("/api/test/panic", get(panics))
            .route("/api/test/quick", get(|| async { "quick" })),
    );

    // Served on a task of its own, so a panic the service did not catch fails this assertion
    // rather than the test's own task.
    let caught = tokio::spawn(send(
        app.clone(),
        request("GET", "/api/test/panic", Body::empty()),
    ))
    .await;
    assert!(
        caught.is_ok(),
        "the handler's panic escaped the service: {caught:?}"
    );
    let status = caught.map(|response| response.status()).ok();
    assert_eq!(status, Some(StatusCode::INTERNAL_SERVER_ERROR));

    // The service keeps serving.
    for _ in 0..3 {
        let response = send(
            app.clone(),
            request("GET", "/api/test/quick", Body::empty()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(text(response).await, "quick");
    }
}
