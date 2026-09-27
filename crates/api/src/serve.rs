//! Serving the API on its loopback listener, and draining on the shutdown signal (SPEC-025 R6, R7).
//!
//! The daemon's `api` role binds first, tells systemd it is ready, then serves: binding is where a
//! start can still fail, and readiness is sent only once the listener exists. On the shutdown
//! signal, axum stops accepting, lets every request in flight finish (each is bounded by the
//! router's request timeout), closes the connections, and `serve` returns (axum's graceful-shutdown
//! example).

use std::future::Future;

use axum::Router;
use tokio::net::TcpListener;

use crate::ApiError;
use crate::settings::{LISTEN, ListenAddress};

/// Binds the listener on `address`, which is loopback by construction.
///
/// # Errors
///
/// [`ApiError::Bind`] when the address cannot be bound (in use, or not permitted), naming the
/// setting and never the address.
pub async fn bind(address: ListenAddress) -> Result<TcpListener, ApiError> {
    TcpListener::bind(address.socket_address())
        .await
        .map_err(|source| ApiError::Bind {
            setting: LISTEN,
            source,
        })
}

/// Serves `router` on `listener` until `shutdown` resolves, then drains: it stops accepting, lets
/// every request in flight finish, and returns once their connections are closed.
///
/// # Errors
///
/// [`ApiError::Serve`] when the server fails.
pub async fn serve<F>(listener: TcpListener, router: Router, shutdown: F) -> Result<(), ApiError>
where
    F: Future<Output = ()> + Send + 'static,
{
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(ApiError::Serve)
}
