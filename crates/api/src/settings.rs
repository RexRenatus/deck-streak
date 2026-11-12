//! The API's own setting: the loopback address it listens on (SPEC-025 R6; ADR-007).
//!
//! The API is reached only through the reverse proxy on the same host, so it listens on a loopback
//! address and nothing else: IPv4's 127.0.0.0/8 or IPv6's `::1`. Any other address, the
//! unspecified ones that listen on every interface included, refuses start by the setting's name.
//! As every setting, a refusal never carries the value it refused (SPEC-020 R10).

use std::fmt;
use std::net::SocketAddr;

use deck_streak_kernel::{Environment, Setting};

use crate::ApiError;

/// The address the API listens on, which must be a loopback address, such as `127.0.0.1:8080`.
pub const LISTEN: &str = "DECKSTREAK_API_LISTEN";

/// The address the API listens on: a loopback address, by construction.
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
    /// [`ApiError::Settings`] when the setting is unset or is not a socket address, and
    /// [`ApiError::NotLoopback`] when it is not a loopback address, each naming the setting.
    pub fn from_env(env: &Environment) -> Result<Self, ApiError> {
        let SocketSetting(address) = env.required(LISTEN)?;
        Self::loopback(address).ok_or(ApiError::NotLoopback { setting: LISTEN })
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

/// A socket address as the setting holds it, before the loopback rule judges it. A host name is
/// not an address: the API resolves no name at start.
struct SocketSetting(SocketAddr);

impl Setting for SocketSetting {
    const SHAPE: &'static str = "a socket address with a port, such as 127.0.0.1:8080";

    fn parse(text: &str) -> Option<Self> {
        text.parse().ok().map(Self)
    }
}
