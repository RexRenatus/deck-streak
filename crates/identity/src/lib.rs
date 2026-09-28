//! # deck-streak-identity
//!
//! What this context owns: Who is asking: Telegram Mini App `initData` validated on the server
//! (HMAC-SHA256 with the `WebAppData` key, a bounded `auth_date`, a constant-time compare), the
//! owner allow-list, sessions, and the sign-in methods linked to the Telegram account.
//!
//! What it does not own: No product data. It answers who the caller is and whether they are the
//! owner.
//!
//! SPEC-024 builds the Telegram half: the one validator of the launch data ([`init_data`]), the
//! owner pin and its two credentials ([`owner`]), the in-memory sessions behind a `__Host-` cookie
//! and the extractor that admits a request ([`session`]), and the freshness bound ([`settings`]).
//! The API mounts the routes. Nothing here is written to disk (ADR-024): the owner's id and a key
//! derived from the bot token live in memory, and a session is kept as the SHA-256 of its id.
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod init_data;
pub mod owner;
pub mod session;
pub mod settings;

use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};

pub use init_data::{Caller, WebAppKey, validate};
pub use owner::{IdentityError, Owner, OwnerGate};
pub use session::{OwnerSession, SessionError, SessionToken, Sessions};
pub use settings::Freshness;

/// Why identity refused a request. A refusal carries one reason code, which is what the log and
/// the answer say, and nothing of the request: never the launch data, its hash or a session id
/// (SPEC-024 R3, R4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The launch data is missing, malformed or forged.
    InitDataInvalid,
    /// The launch data is validly signed, and its `auth_date` is outside the freshness window:
    /// older than the bound, or further ahead of the server's clock than the skew allows.
    InitDataStale,
    /// The launch data is valid and fresh, and its user is not the owner.
    NotOwner,
    /// The request's cookie names no live session.
    NoSession,
}

impl Refusal {
    /// The reason code the log and the answer carry.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::InitDataInvalid => "init_data_invalid",
            Self::InitDataStale => "init_data_stale",
            Self::NotOwner => "not_owner",
            Self::NoSession => "no_session",
        }
    }

    /// The answer's status: 401 for who the caller is not proved to be, 403 for a proved caller
    /// who is not the owner.
    #[must_use]
    pub const fn status(self) -> StatusCode {
        match self {
            Self::InitDataInvalid | Self::InitDataStale | Self::NoSession => {
                StatusCode::UNAUTHORIZED
            }
            Self::NotOwner => StatusCode::FORBIDDEN,
        }
    }
}

impl IntoResponse for Refusal {
    /// The status, and a JSON body naming the reason code alone: `{"reason":"init_data_stale"}`.
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "reason": self.reason() }).to_string();
        (self.status(), [(CONTENT_TYPE, "application/json")], body).into_response()
    }
}
