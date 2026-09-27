//! The API's own setting: the address it listens on (SPEC-025 R6; ADR-007).
//!
//! STUB for the red-first commit: any socket address is accepted.

use std::fmt;
use std::net::SocketAddr;

use deck_streak_kernel::{Environment, Setting};

use crate::ApiError;

/// The address the API listens on, which must be a loopback address.
pub const LISTEN: &str = "DECKSTREAK_API_LISTEN";

/// The address the API listens on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListenAddress(SocketAddr);

impl ListenAddress {
    /// `address`.
    #[must_use]
    pub const fn loopback(address: SocketAddr) -> Option<Self> {
        Some(Self(address))
    }

    /// The address [`LISTEN`] names.
    ///
    /// # Errors
    ///
    /// A missing or malformed setting.
    pub fn from_env(env: &Environment) -> Result<Self, ApiError> {
        let SocketSetting(address) = env.required(LISTEN)?;
        Ok(Self(address))
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

/// A socket address as the setting holds it.
struct SocketSetting(SocketAddr);

impl Setting for SocketSetting {
    const SHAPE: &'static str = "a socket address with a port, such as 127.0.0.1:8080";

    fn parse(text: &str) -> Option<Self> {
        text.parse().ok().map(Self)
    }
}
