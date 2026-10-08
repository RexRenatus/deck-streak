//! The adapter's settings: the guard's credentials, their ids, the shortest token a grant accepts
//! and why a credential refuses start (SPEC-119 R6, R7; ADR-320), and the loopback address the
//! server listens on (R3; ADR-329).
//!
//! Every refusal names a credential's id or a setting's name and never its value: a credential is a
//! secret the loader has already registered with the redactor (SPEC-066), and a setting's value
//! never reaches a refusal (SPEC-020 R10).

use std::fmt;
use std::net::SocketAddr;

use deck_streak_kernel::{CredentialError, Environment, Setting, SettingsError};

/// The address the server listens on, which must be a loopback address, such as localhost's port
/// 8790 (R3).
pub const LISTEN: &str = "DECKSTREAK_MCP_LISTEN";

/// The core token's credential id. It is required: every loader error refuses start (R6).
pub const CORE_CREDENTIAL: &str = "mcp-core-token";

/// The law-track token's credential id. It is optional: a missing file grants nothing, and any
/// other loader error refuses start (R6).
pub const LAW_TRACK_CREDENTIAL: &str = "mcp-law-track-token";

/// The write token's credential id. It is optional: a missing file grants nothing, and any other
/// loader error refuses start (SPEC-369 R3).
pub const WRITE_CREDENTIAL: &str = "mcp-write-token";

/// The fewest characters a loaded token may hold (R7).
pub const MIN_CREDENTIAL_CHARS: usize = 32;

/// Why the guard refuses to start. Each variant names a credential's id, never its value.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// The credential loader refused the credential (R6).
    #[error(transparent)]
    Credential(#[from] CredentialError),
    /// The credential holds a byte outside 0x21 to 0x7E, which no request can present (R9), so its
    /// grant could never be used (R7 as amended by T15).
    #[error("the credential {id} holds a character no request can present")]
    UnpresentableCredential {
        /// The credential's id.
        id: &'static str,
    },
    /// The credential is shorter than [`MIN_CREDENTIAL_CHARS`] characters (R7).
    #[error("the credential {id} is shorter than {MIN_CREDENTIAL_CHARS} characters")]
    WeakCredential {
        /// The credential's id.
        id: &'static str,
    },
    /// Two credentials hold one value, which would give one token both grants' scopes (R7).
    #[error("the credentials {first} and {second} hold one value")]
    SharedCredential {
        /// The first credential's id.
        first: &'static str,
        /// The second credential's id.
        second: &'static str,
    },
    /// The listen setting refused start (R3). It names the setting and never its value.
    #[error("the setting {setting} must be a loopback socket address with a port: {reason}")]
    Listen {
        /// The listen address's setting.
        setting: &'static str,
        /// Why it refused.
        reason: ListenRefusal,
    },
}

/// Why the listen setting refused start (R3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListenRefusal {
    /// The setting is unset or blank.
    Unset,
    /// The setting is not a socket address with a port. A host name is not one: the server
    /// resolves no name at start.
    NotAnAddress,
    /// The address is not a loopback address: the unspecified addresses, which listen on every
    /// interface, are refused with the rest.
    NotLoopback,
}

impl fmt::Display for ListenRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unset => "it is required and is not set",
            Self::NotAnAddress => "it is not a socket address with a port",
            Self::NotLoopback => "it is not a loopback address",
        })
    }
}

/// The address the server listens on: a loopback address, by construction (R3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListenAddress(SocketAddr);

impl ListenAddress {
    /// `address`, or `None` when it is not a loopback address.
    #[must_use]
    pub const fn loopback(address: SocketAddr) -> Option<Self> {
        if address.ip().is_loopback() {
            Some(Self(address))
        } else {
            None
        }
    }

    /// The address [`LISTEN`] names.
    ///
    /// # Errors
    ///
    /// [`McpError::Listen`] when the setting is unset, is not a socket address, or is not a
    /// loopback address, naming the setting.
    pub fn from_env(env: &Environment) -> Result<Self, McpError> {
        let refused = |reason| McpError::Listen {
            setting: LISTEN,
            reason,
        };
        let SocketSetting(address) = env.required(LISTEN).map_err(|error| match error {
            SettingsError::Missing { .. } => refused(ListenRefusal::Unset),
            _ => refused(ListenRefusal::NotAnAddress),
        })?;
        Self::loopback(address).ok_or_else(|| refused(ListenRefusal::NotLoopback))
    }

    /// The socket address.
    #[must_use]
    pub const fn socket_address(self) -> SocketAddr {
        self.0
    }
}

impl fmt::Display for ListenAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A socket address as the setting holds it, before the loopback rule judges it.
struct SocketSetting(SocketAddr);

impl Setting for SocketSetting {
    const SHAPE: &'static str = "a socket address with a port";

    fn parse(text: &str) -> Option<Self> {
        text.parse().ok().map(Self)
    }
}
