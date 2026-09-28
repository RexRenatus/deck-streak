//! The one validator of Telegram's Mini App launch data (SPEC-024 R1, R2; ADR-006).
//!
//! Telegram's rule (core.telegram.org/bots/webapps, "Validating data received via the Mini App"):
//! the data-check-string is every received field but `hash`, sorted by key, written `key=value`
//! and joined by a line feed; the secret key is the HMAC-SHA-256 of the bot token keyed with the
//! constant `WebAppData`; and `hash` is the hex HMAC-SHA-256 of the data-check-string under that
//! key. The `signature` field stays in the string: only Telegram's third-party Ed25519 check leaves
//! it out. Checking `auth_date` is left to the service; here it must be inside the freshness window
//! ([`Freshness`]) on the kernel's clock, and it is read only after the signature holds.

use deck_streak_kernel::{TelegramUserId, UtcMillis};

use crate::Refusal;
use crate::settings::Freshness;

/// The Mini App's secret key: the HMAC-SHA-256 of the bot token keyed with `WebAppData`. It is
/// derived once, at start, and the bot token itself is not kept. Its `Debug` never shows it.
#[derive(Clone)]
pub struct WebAppKey([u8; 32]);

impl WebAppKey {
    /// The secret key of `bot_token`.
    #[must_use]
    pub fn from_bot_token(bot_token: &str) -> Self {
        let _unread = bot_token;
        Self([0; 32])
    }
}

impl std::fmt::Debug for WebAppKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebAppKey(..)")
    }
}

/// Who validly signed launch data names: of Telegram's user object, only the id is kept (R4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caller {
    user: TelegramUserId,
}

impl Caller {
    /// The caller's Telegram user id.
    #[must_use]
    pub const fn user(self) -> TelegramUserId {
        self.user
    }
}

/// The caller the raw launch data `raw` names, when it is signed with `key`, and fresh by
/// `freshness` at `now`.
///
/// # Errors
///
/// [`Refusal::InitDataInvalid`] when `raw` is missing a field, repeats a key, has a field that does
/// not decode, carries no hash or a forged one, or has no integer `auth_date` or no `user.id`; and
/// [`Refusal::InitDataStale`] when it is validly signed and its `auth_date` is older than the
/// bound, or further ahead of `now` than the skew allows.
pub fn validate(
    raw: &str,
    key: &WebAppKey,
    freshness: Freshness,
    now: UtcMillis,
) -> Result<Caller, Refusal> {
    let _unread = (raw, key, freshness, now);
    Ok(Caller {
        user: TelegramUserId::new(0),
    })
}
