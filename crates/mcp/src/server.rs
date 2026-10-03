//! The MCP server: its one path and its serving stack (SPEC-119 R2 to R4; ADR-329).
//!
//! The router serves the streamable HTTP transport at [`MCP_PATH`] alone, stateless and answering
//! JSON, and admits only a `Host` that names a loopback address (R2, R3). Every request passes the
//! house layers, outermost first: a request id, the sensitive headers marked, the trace span named
//! by the matched route, the request id copied to the answer, a caught panic and a timeout; then
//! the bearer guard; then the bound of [`MAX_IN_FLIGHT`] requests, shed at once with 503 past it,
//! and the body cap of [`BODY_LIMIT_BYTES`] (R4, R11; ADR-329 D5). The guard sits outside the
//! bound, so a request it refuses never takes a slot, and inside the trace, so its `Authorization`
//! header is redacted before the span records the request.

use std::future::Future;
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::extract::{MatchedPath, Request};
use axum::http::header::{AUTHORIZATION, COOKIE, SET_COOKIE};
use axum::http::{HeaderName, StatusCode};
use axum::{BoxError, Router};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower::limit::GlobalConcurrencyLimitLayer;
use tower::load_shed::LoadShedLayer;
use tower::load_shed::error::Overloaded;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::sensitive_headers::{
    SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::guard::{Guard, GuardLayer};
use crate::settings::ListenAddress;
use crate::tools::{LawTrackSource, McpServer};

/// The one path the server answers at; any other answers 404 (R2).
pub const MCP_PATH: &str = "/mcp";

/// The requests served at once; one more is shed at once with 503 (R4).
pub const MAX_IN_FLIGHT: usize = 8;

/// The largest request body read, in bytes: 64 KiB (R2).
pub const BODY_LIMIT_BYTES: usize = 64 * 1024;

/// How long a request may take before it is answered 408.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The header that carries each request's id, set on the way in and copied to the answer.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// The `Host` names the transport admits, each with any port: the loopback names (R3). A page that
/// rebinds another name to the loopback address names that other name, and is refused 403.
#[must_use]
pub fn loopback_hosts() -> [String; 3] {
    [
        "localhost".to_owned(),
        Ipv4Addr::LOCALHOST.to_string(),
        Ipv6Addr::LOCALHOST.to_string(),
    ]
}

/// The transport's configuration: no session, a JSON answer, the loopback hosts, the body cap the
/// transport's own read obeys, and no stream to keep alive (R2; ADR-329 D4).
fn config() -> StreamableHttpServerConfig {
    StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_allowed_hosts(loopback_hosts())
        .with_max_request_body_bytes(BODY_LIMIT_BYTES)
        .with_sse_keep_alive(None)
}

/// The router the role serves, over the law track's `law` and the guard `guard`: the transport at
/// [`MCP_PATH`], under [`layered`]'s stack.
pub fn router(law: Arc<dyn LawTrackSource>, guard: Arc<Guard>) -> Router {
    let tools = Arc::clone(&guard);
    let transport = StreamableHttpService::new(
        move || Ok(McpServer::new(Arc::clone(&law), Arc::clone(&tools))),
        Arc::new(NeverSessionManager::default()),
        config(),
    );
    layered(Router::new().route_service(MCP_PATH, transport), guard)
}

/// `routes` under the serving stack, as this module's documentation lists it, with `guard` as the
/// bearer guard. The production router is built through here, so a route is never served without
/// the guard or the bounds.
pub fn layered(routes: Router, guard: Arc<Guard>) -> Router {
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
            .layer(GuardLayer::new(guard))
            .layer(HandleErrorLayer::new(shed))
            .layer(LoadShedLayer::new())
            .layer(GlobalConcurrencyLimitLayer::new(MAX_IN_FLIGHT))
            .layer(RequestBodyLimitLayer::new(BODY_LIMIT_BYTES)),
    )
}

/// The headers whose values never reach a log: the bearer travels as `Authorization`.
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

/// The listener on `address`, which is a loopback address by construction (R3).
///
/// # Errors
///
/// The socket's error when the address cannot be bound.
pub async fn bind(address: ListenAddress) -> io::Result<TcpListener> {
    TcpListener::bind(address.socket_address()).await
}

/// Serves `router` on `listener` until `shutdown` resolves, then drains: it stops accepting, lets
/// every request in flight finish, and returns once their connections are closed.
///
/// # Errors
///
/// The listener's error when serving fails.
pub async fn serve<F>(listener: TcpListener, router: Router, shutdown: F) -> io::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
}
