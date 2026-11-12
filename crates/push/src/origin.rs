//! The origin rule and the list of push services (SPEC-343 R7, ADR-354 D7).
//!
//! An origin is `https:`, or `http:` to a loopback host, with no user, path, query or fragment:
//! the rule the bot's own base URL keeps (`crates/bot/src/transport.rs`), so every test reaches a
//! loopback fake and every production request travels over TLS. The list of push services is given
//! to a web push sender when it is built, and an origin the list does not hold is refused, so a
//! sender built with no list refuses every endpoint (ruling 257 Q7): the production list is #640's.

use std::fmt;
use std::net::IpAddr;

use crate::BuildError;

/// Where a sender may send: a scheme and an authority, and nothing after them.
#[derive(Clone, PartialEq, Eq)]
pub struct Origin(String);

impl Origin {
    /// `text` as an origin. A root `/` is dropped; any path beyond it is refused.
    ///
    /// # Errors
    ///
    /// [`BuildError::Origin`] when it is not `https:` or loopback `http:`, names no host, or
    /// carries a path, a query, a fragment, a user or whitespace.
    pub fn new(text: &str) -> Result<Self, BuildError> {
        let text = text.strip_suffix('/').unwrap_or(text);
        let (scheme, authority) = text.split_once("://").ok_or(BuildError::Origin)?;
        if authority.is_empty()
            || authority.contains(['/', '?', '#', '@'])
            || authority.contains(char::is_whitespace)
        {
            return Err(BuildError::Origin);
        }
        let host = host_of(authority).ok_or(BuildError::Origin)?;
        let secure = scheme == "https";
        let local = scheme == "http" && is_loopback(host);
        if secure || local {
            Ok(Self(text.to_owned()))
        } else {
            Err(BuildError::Origin)
        }
    }

    /// The origin, as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Origin").field(&self.0).finish()
    }
}

/// The host of an authority, `host`, `host:port` or `[v6]:port`, or `None` when it names none.
fn host_of(authority: &str) -> Option<&str> {
    let host = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split(']').next()?
    } else {
        authority.split(':').next()?
    };
    (!host.is_empty()).then_some(host)
}

/// Whether `host` is `localhost` or a loopback address.
fn is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// The push services a web push sender may post to. The check fails closed: an origin the list
/// does not hold is refused, and an empty list holds none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PushServices(Vec<Origin>);

impl PushServices {
    /// The list of `origins`.
    #[must_use]
    pub fn new(origins: impl IntoIterator<Item = Origin>) -> Self {
        Self(origins.into_iter().collect())
    }

    /// Whether `origin` is on the list.
    #[must_use]
    pub fn admits(&self, origin: &Origin) -> bool {
        self.0.contains(origin)
    }
}
