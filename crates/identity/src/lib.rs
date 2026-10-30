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
//! SPEC-359 links passkeys to the owner's Telegram account for sign-in on the web: the public
//! origin that turns linking on ([`linking_config`]), the link code and the owner's methods
//! ([`linking`]), the passkey ceremonies, the counter rule and the `passkeys` table ([`passkeys`]),
//! and that table's data rights ([`data_rights`]). A ceremony's state stays in memory and is never
//! serialized; the table holds the owner's registered credentials alone (ADR-370).
//!
//! SPEC-363 makes the key the web client's sync key is sealed under: the seal secret, read once at
//! start, and the sealing key for a browser's seal id, which the API releases to the owner's live
//! session alone ([`sync_seal`]). Nothing per browser is stored (ADR-374).
//!
//! The context map (docs/CONTEXT-MAP.md) is binding: this crate depends only on what its line
//! there declares, and a new edge is an ADR, never a fix to make code compile.
#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

pub mod data_rights;
pub mod init_data;
pub mod linking;
pub mod linking_config;
pub mod owner;
pub mod passkeys;
pub mod session;
pub mod settings;
pub mod sync_seal;

use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};

pub use data_rights::IdentityDataRights;
pub use init_data::{Caller, WebAppKey, validate};
pub use linking::{LinkCode, LinkCodes, Method};
pub use linking_config::{LinkingConfig, RelyingParty};
pub use owner::{IdentityError, Owner, OwnerGate};
pub use passkeys::{Ceremonies, FlowId, PasskeyError, Passkeys, SignedIn, Started};
pub use session::{
    FreshTelegramSession, LinkSession, OwnerSession, Proof, SessionError, SessionToken, Sessions,
};
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
    /// The route needs a Telegram session whose handshake is at most five minutes old (SPEC-359
    /// R3, R9).
    ReauthRequired,
    /// The link code is unknown, already redeemed or evicted (SPEC-359 R3).
    LinkCodeInvalid,
    /// The link code outlived its ten minutes (SPEC-359 R3).
    LinkCodeExpired,
    /// The ceremony is unknown, already used, evicted or another session's (SPEC-359 R7, R8).
    ChallengeInvalid,
    /// The ceremony outlived its five minutes (SPEC-359 R7).
    ChallengeExpired,
    /// The response names another origin or relying party (SPEC-359 R8).
    OriginMismatch,
    /// The response carries no user verification (SPEC-359 R8).
    UvRequired,
    /// The response's signature or shape does not verify (SPEC-359 R8).
    PasskeyInvalid,
    /// No passkey is held, or the credential is not one of the owner's (SPEC-359 R6).
    NotLinked,
    /// The assertion's counter does not advance the stored one (SPEC-359 R8).
    CounterRegressed,
    /// The credential is already registered (SPEC-359 R5).
    AlreadyLinked,
    /// The method is the account's last way in: Telegram, the primary (SPEC-359 R9).
    LastMethod,
    /// No public origin is configured, so linking is off (SPEC-359 R1).
    LinkingOff,
    /// No method has that id (SPEC-359 R9).
    IdentityUnknown,
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
            Self::ReauthRequired => "reauth_required",
            Self::LinkCodeInvalid => "link_code_invalid",
            Self::LinkCodeExpired => "link_code_expired",
            Self::ChallengeInvalid => "challenge_invalid",
            Self::ChallengeExpired => "challenge_expired",
            Self::OriginMismatch => "origin_mismatch",
            Self::UvRequired => "uv_required",
            Self::PasskeyInvalid => "passkey_invalid",
            Self::NotLinked => "not_linked",
            Self::CounterRegressed => "counter_regressed",
            Self::AlreadyLinked => "already_linked",
            Self::LastMethod => "last_method",
            Self::LinkingOff => "linking_off",
            Self::IdentityUnknown => "identity_unknown",
        }
    }

    /// The answer's status: 401 for who the caller is not proved to be and for a ceremony or code
    /// that does not verify, 403 for a proved caller who is not the owner, 409 for a credential or
    /// a method the account holds, and 404 for linking off or a method no one holds (SPEC-359 R8).
    #[must_use]
    #[allow(
        clippy::match_same_arms,
        reason = "counter_regressed answers 401 on its own line, the line row S35927 mutates"
    )]
    pub const fn status(self) -> StatusCode {
        match self {
            Self::InitDataInvalid
            | Self::InitDataStale
            | Self::NoSession
            | Self::ReauthRequired
            | Self::LinkCodeInvalid
            | Self::LinkCodeExpired
            | Self::ChallengeInvalid
            | Self::ChallengeExpired
            | Self::OriginMismatch
            | Self::UvRequired
            | Self::PasskeyInvalid
            | Self::NotLinked => StatusCode::UNAUTHORIZED,
            Self::CounterRegressed => StatusCode::UNAUTHORIZED,
            Self::NotOwner => StatusCode::FORBIDDEN,
            Self::AlreadyLinked | Self::LastMethod => StatusCode::CONFLICT,
            Self::LinkingOff | Self::IdentityUnknown => StatusCode::NOT_FOUND,
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
