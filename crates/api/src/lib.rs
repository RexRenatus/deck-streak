//! # deck-streak-api
//!
//! What this context owns: The HTTPS API the Mini App calls: axum routes over the coordination
//! use cases, the `initData` extractor, and the in-app transport of the notification router.
//!
//! What it does not own: Any use case of its own: a route maps a request to a coordination use
//! case.
//!
//! SPEC-025 builds its shell: the health routes ([`health`]), the one stack of layers every route
//! is served under ([`router`]), the loopback listen address ([`settings`]) and the serve that
//! drains on the shutdown signal ([`serve`]). The daemon's `api` role binds, serves and signals
//! systemd; this crate holds no process lifecycle of its own. SPEC-024 adds the owner's session
//! routes ([`session_routes`]): the handshake, the logout and the owner's day, over identity's
//! gate, sessions and extractor. SPEC-041 adds the in-app feed the notification router appends to
//! ([`notifications_routes`]), served to the owner's session alone.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod health;
pub mod notifications_routes;
pub mod router;
pub mod serve;
pub mod session_routes;
pub mod settings;

use deck_streak_kernel::SettingsError;

pub use health::Readiness;
pub use router::{ApiState, layered, router};
pub use serve::{bind, serve};
pub use session_routes::OwnerAccess;
pub use settings::ListenAddress;

/// Why the API could not start or keep serving. It names a setting and never its value.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// A setting refused start.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The listen address is not a loopback address: the API is reached only through the reverse
    /// proxy on the same host (ADR-007).
    #[error(
        "the setting {setting} is not a loopback address: the API listens on loopback only, \
         behind the reverse proxy"
    )]
    NotLoopback {
        /// The listen address's setting.
        setting: &'static str,
    },
    /// The listener could not be bound; the source says why.
    #[error("the API could not listen on the address the setting {setting} names")]
    Bind {
        /// The listen address's setting.
        setting: &'static str,
        /// The operating system's reason.
        #[source]
        source: std::io::Error,
    },
    /// The server stopped with an error; the source says why.
    #[error("the API's server stopped with an error")]
    Serve(#[source] std::io::Error),
}
