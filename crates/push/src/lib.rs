//! # deck-streak-push
//!
//! What this adapter owns: the requests a platform's push service accepts, APNs for the iPhone
//! and iPad client and web push for the web client, and the reading of each answer into one typed
//! outcome. It builds each request itself (ADR-354 D1): the path, every header, the provider or
//! VAPID token, and the body, encrypted as RFC 8291 says for web push (ADR-354 D2).
//!
//! What it does not own: whether a notification is sent, when, or how loudly, which the one router
//! decides (ADR-341); what to do after an answer, which is the router's too, so a sender retries
//! nothing but the one resend an expired provider token earns (ADR-354 D6); and where a key comes
//! from, which production reads through the kernel's credential loader (#640). Nothing composes
//! this crate until native push carries the router (#640).
//!
//! Nothing it logs or prints carries key material, a provider or VAPID token, a device token, a
//! subscription's endpoint or keys, or a request path (SPEC-343 R8): every type that holds one
//! writes its `Debug` by hand.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]
#![warn(missing_docs, clippy::all)]

mod apns;
mod client;
mod jwt;
mod origin;
mod web_push;

use std::fmt;
use std::time::Duration;

pub use apns::{ApnsSender, ApnsSettings, Device, Environment};
pub use origin::{Origin, PushServices};
pub use web_push::{Subscription, WebPushSender, WebPushSettings};

use deck_streak_kernel::UtcMillis;

/// The longest a collapse key may be: APNs's `apns-collapse-id` holds 64 bytes and RFC 8030's
/// `Topic` 32 characters, so 32 satisfies both (SPEC-343 R6).
pub const COLLAPSE_KEY_MAX: usize = 32;

/// A key that lets a newer notification replace an older one the device has not shown yet: 1 to
/// [`COLLAPSE_KEY_MAX`] characters of the base64url alphabet, so the one value is both an APNs
/// collapse id and a web push `Topic` (SPEC-343 R6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollapseKey(String);

impl CollapseKey {
    /// `text` as a collapse key.
    ///
    /// # Errors
    ///
    /// [`BuildError::CollapseKey`] when it is empty, longer than [`COLLAPSE_KEY_MAX`], or holds a
    /// character outside `A-Z`, `a-z`, `0-9`, `-` and `_`.
    pub fn new(text: &str) -> Result<Self, BuildError> {
        let admitted = !text.is_empty()
            && text.len() <= COLLAPSE_KEY_MAX
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
        if admitted {
            Ok(Self(text.to_owned()))
        } else {
            Err(BuildError::CollapseKey)
        }
    }

    /// The key, as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One notification: what the owner reads, how long the platform may hold it for a device that is
/// away, and the key that lets a newer one replace it. The router chose it; a sender only carries
/// it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    title: String,
    body: String,
    time_to_live: Duration,
    collapse_key: Option<CollapseKey>,
}

impl Notification {
    /// A notification with no collapse key. A time to live of zero asks the platform to deliver it
    /// now or never.
    #[must_use]
    pub fn new(title: &str, body: &str, time_to_live: Duration) -> Self {
        Self {
            title: title.to_owned(),
            body: body.to_owned(),
            time_to_live,
            collapse_key: None,
        }
    }

    /// The same notification, replacing any earlier one that carries `key`.
    #[must_use]
    pub fn with_collapse_key(self, key: CollapseKey) -> Self {
        Self {
            collapse_key: Some(key),
            ..self
        }
    }

    /// The title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The body.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// How long the platform may hold it.
    #[must_use]
    pub fn time_to_live(&self) -> Duration {
        self.time_to_live
    }

    /// The collapse key, when it has one.
    #[must_use]
    pub fn collapse_key(&self) -> Option<&CollapseKey> {
        self.collapse_key.as_ref()
    }
}

/// What one call to a sender's `deliver` came to: one answer read into one outcome (SPEC-343 R4).
/// The router decides what each means for the occasion; the sender decides nothing more.
#[must_use = "an outcome the router never reads is a push nobody accounted for"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sent {
    /// The platform accepted it.
    Delivered,
    /// The device or the subscription is gone. `since` is Apple's timestamp, when it gave one, so
    /// a token registered after it is kept; web push gives none.
    Gone {
        /// When the platform last knew the device as valid.
        since: Option<UtcMillis>,
    },
    /// The platform refused it, and the same request would be refused again.
    Rejected(Refusal),
    /// The platform asked for a later try, after `after` when it said how long.
    RetryLater {
        /// The platform's `Retry-After`, when it sent one in seconds.
        after: Option<Duration>,
    },
    /// No usable answer came back.
    Failed(Unreached),
}

/// Why a platform refused a notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The device token is wrong, or not for this app.
    Token,
    /// The payload is too large, by the platform's answer or by the sender's own check before any
    /// request.
    TooLarge,
    /// The signing key, or the token made with it, is refused.
    ProviderToken,
    /// Any other refusal of the request.
    Request,
}

/// Why no usable answer came back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreached {
    /// The request could not be made or the connection failed.
    Connection,
    /// The sender's deadline passed first.
    Deadline,
    /// The platform answered with a redirect, which is never followed.
    Redirect,
    /// The answer's status is none a push service sends.
    Unexpected,
    /// The answer's body could not be read.
    Unreadable,
    /// The request could not be built or signed.
    Request,
}

impl Sent {
    /// The outcome's name, as a log line carries it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Delivered => "delivered",
            Self::Gone { .. } => "gone",
            Self::Rejected(_) => "rejected",
            Self::RetryLater { .. } => "retry-later",
            Self::Failed(_) => "failed",
        }
    }
}

/// Why a sender, a device, a subscription or a notification could not be built.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BuildError {
    /// The signing key is not a P-256 key in PKCS#8 PEM text.
    #[error("the signing key is not a P-256 key in PKCS#8 PEM text")]
    Key,
    /// An origin is not `https:`, or `http:` to a loopback host, or carries a user, a path, a query
    /// or a fragment.
    #[error("an origin must be https:, or http: to a loopback host, with nothing after its host")]
    Origin,
    /// A subscription's endpoint is not on the sender's list of push services.
    #[error("the endpoint's origin is not on the sender's list of push services")]
    OffTheList,
    /// A subscription's endpoint, public key or authentication secret does not parse.
    #[error("the subscription's endpoint, public key or authentication secret does not parse")]
    Subscription,
    /// A device token is not hexadecimal text.
    #[error("a device token is hexadecimal text of at most 200 characters")]
    DeviceToken,
    /// A collapse key is not 1 to 32 base64url characters.
    #[error("a collapse key is 1 to 32 characters of the base64url alphabet")]
    CollapseKey,
    /// A key id, team id, topic or contact is empty or holds a character it may not.
    #[error("a key id, team id, topic or contact is empty or holds a character it may not")]
    Setting,
    /// The TLS connector could not be built.
    #[error("the TLS connector could not be built")]
    Tls,
}

impl fmt::Display for Sent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}
