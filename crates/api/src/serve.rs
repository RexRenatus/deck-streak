//! Serving the API on its loopback listener (SPEC-025 R6, R7).
//!
//! STUB for the red-first commit: `serve` drops every connection when the signal arrives.

use std::future::Future;

use axum::Router;
use tokio::net::TcpListener;

use crate::ApiError;
use crate::settings::{LISTEN, ListenAddress};

/// Binds the listener on `address`.
///
/// # Errors
///
/// [`ApiError::Bind`] when the address cannot be bound.
pub async fn bind(address: ListenAddress) -> Result<TcpListener, ApiError> {
    TcpListener::bind(address.socket_address())
        .await
        .map_err(|source| ApiError::Bind {
            setting: LISTEN,
            source,
        })
}

/// Serves `router` on `listener` until `shutdown` resolves.
///
/// # Errors
///
/// [`ApiError::Serve`] when the server fails.
pub async fn serve<F>(listener: TcpListener, router: Router, shutdown: F) -> Result<(), ApiError>
where
    F: Future<Output = ()> + Send + 'static,
{
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    shutdown.await;
    server.abort();
    Ok(())
}
