//! The API's routes and the one stack of layers every route is served under (SPEC-025 R3 to R5;
//! ADR-025).
//!
//! The stack is applied with `Router::layer`, so it runs after routing and the trace can name the
//! MATCHED route rather than the raw path; axum then applies each layer to every route, and to the
//! fallback's 404, separately. That is why the concurrency bound is tower's
//! `GlobalConcurrencyLimitLayer`, whose one semaphore every route shares, rather than
//! `ConcurrencyLimitLayer`, which would give each route a bound of its own (SPEC-025 §7).
//!
//! Outermost first:
//!
//! 1. an `x-request-id` is set (`MakeRequestUuid`) unless the request carries one;
//! 2. `authorization`, `cookie` and `set-cookie` are marked sensitive on the request, before the
//!    trace can read them;
//! 3. the trace: an INFO span with the method, the matched route and the request id, and an INFO
//!    response event with the status and the latency. The span never records the URI, whose query
//!    string may carry `initData`;
//! 4. the same headers are marked sensitive on the response, before the trace sees it;
//! 5. the request id is copied to the response, outside every layer below, so a caught panic's
//!    500, a timeout's 408 and a shed 503 carry it too;
//! 6. a panic is caught and answered 500, and the service keeps serving;
//! 7. a request that has not answered in [`REQUEST_TIMEOUT`] is answered 408;
//! 8. a request past [`MAX_IN_FLIGHT`] in flight is shed with 503 at once, never queued
//!    (ADR-025);
//! 9. the extractors read a body of at most [`BODY_LIMIT_BYTES`], and answer 413 past it.

use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::extract::{DefaultBodyLimit, MatchedPath, Request};
use axum::http::header::{AUTHORIZATION, COOKIE, SET_COOKIE};
use axum::http::{HeaderName, StatusCode};
use axum::{BoxError, Router};
use tower::ServiceBuilder;
use tower::limit::GlobalConcurrencyLimitLayer;
use tower::load_shed::LoadShedLayer;
use tower::load_shed::error::Overloaded;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::{
    SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::health::{self, Readiness};
use crate::session_routes::{self, OwnerAccess};

/// Requests served at once, the rust-service pack's reference value. Each holds its buffers until
/// it answers, so this bounds the service's memory; one owner's Mini App never reaches it.
pub const MAX_IN_FLIGHT: usize = 64;
/// A request that has not answered by now is answered 408, which also bounds how long a drain on
/// SIGTERM can take (the unit's `TimeoutStopSec=` sits above it).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// The largest request body the extractors read: axum's own default, 2 MiB, stated here as the
/// number it is, so no route can mistake the limit for switched off.
pub const BODY_LIMIT_BYTES: usize = 2 * 1024 * 1024;
/// The header the request id travels in.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// What the API's handlers share.
#[derive(Clone, Debug)]
pub struct ApiState {
    readiness: Readiness,
    owner: Option<OwnerAccess>,
}

impl ApiState {
    /// The state over `readiness`, which the daemon's `api` role records the opened database in.
    #[must_use]
    pub const fn new(readiness: Readiness) -> Self {
        Self {
            readiness,
            owner: None,
        }
    }

    /// This state, serving the owner's session routes over `access` too (SPEC-024).
    #[must_use]
    pub fn with_owner(mut self, access: OwnerAccess) -> Self {
        self.owner = Some(access);
        self
    }

    /// Whether the API can answer from its database.
    #[must_use]
    pub const fn readiness(&self) -> &Readiness {
        &self.readiness
    }
}

/// The API: every route the Mini App's backend serves, under the layers. The health routes are
/// always served; the owner's session routes are served when the state carries the owner's
/// access, which the daemon's `api` role always gives, since it refuses to start without the
/// owner's credentials. A router built for the health routes alone needs none.
pub fn router(state: ApiState) -> Router {
    let owner = state.owner.clone();
    let routes = health::routes().with_state(state);
    let routes = match owner {
        Some(access) => routes.merge(session_routes::routes(access)),
        None => routes,
    };
    layered(routes)
}

/// `routes` under the service's layers, as listed in this module's documentation. The production
/// router is built through here, and so are the tests' own routes, so a route is never served
/// without the bounds.
pub fn layered(routes: Router) -> Router {
    routes.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(SetSensitiveRequestHeadersLayer::new(sensitive_headers()))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|request: &Request| {
                        let route = request
                            .extensions()
                            .get::<MatchedPath>()
                            .map(MatchedPath::as_str);
                        let request_id = request
                            .headers()
                            .get(REQUEST_ID_HEADER)
                            .and_then(|value| value.to_str().ok());
                        tracing::info_span!(
                            "request",
                            method = %request.method(),
                            route,
                            request_id,
                        )
                    })
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            )
            .layer(SetSensitiveResponseHeadersLayer::new(sensitive_headers()))
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(CatchPanicLayer::new())
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                REQUEST_TIMEOUT,
            ))
            .layer(HandleErrorLayer::new(shed))
            .layer(LoadShedLayer::new())
            .layer(GlobalConcurrencyLimitLayer::new(MAX_IN_FLIGHT))
            .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES)),
    )
}

/// The headers whose values never reach a log: the Mini App's `initData` travels as
/// `Authorization`, and a session as a cookie.
fn sensitive_headers() -> [HeaderName; 3] {
    [AUTHORIZATION, COOKIE, SET_COOKIE]
}

/// The answer to a request the bound refused: 503 at once, so the connection is released and the
/// shed is a counted response event (ADR-025). Only the load shed below this layer can fail, so
/// any other error would be a defect, answered 500.
async fn shed(error: BoxError) -> StatusCode {
    if error.is::<Overloaded>() {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
