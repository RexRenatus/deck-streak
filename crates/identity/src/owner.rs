//! The owner pin and its two credentials (SPEC-024 R3, R4; CHARTER 14; ADR-006, ADR-038).
//!
//! The service answers one owner. Its Telegram user id is the credential [`OWNER_USER_ID`],
//! and the bot token the credential [`TELEGRAM_BOT_TOKEN`], both read at start through the
//! kernel's loader, which registers each value with the log's redactor before it returns it; a
//! missing or malformed one refuses start by its id, never by its value. The bot token is kept
//! only as the key derived from it ([`WebAppKey`]).
//!
//! [`OwnerGate::admit`] is the handshake's check: the launch data validated, then its user pinned
//! to the owner. It logs the outcome, and a refusal by its reason code alone.

use std::fmt;

use deck_streak_kernel::{CredentialError, CredentialLoader, TelegramUserId, UtcMillis};

use crate::Refusal;
use crate::init_data::{WebAppKey, validate};
use crate::settings::Freshness;

/// The credential holding the owner's Telegram user id.
pub const OWNER_USER_ID: &str = "owner-user-id";
/// The credential holding the Telegram bot's token.
pub const TELEGRAM_BOT_TOKEN: &str = "telegram-bot-token";

/// Why identity's credentials refuse start. It names a credential's id and never its value.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// A credential is missing or cannot be read.
    #[error(transparent)]
    Credential(#[from] CredentialError),
    /// A credential was read and does not have the shape it must have.
    #[error("the credential {id} is malformed: it must be {expected}")]
    Malformed {
        /// The credential's id.
        id: &'static str,
        /// The shape the credential must have.
        expected: &'static str,
    },
}

/// The owner: the one Telegram user this deployment serves (CHARTER 14). The bot's gate compares
/// an update's sender with the same owner (SPEC-026). Its `Debug` never shows the id, which is
/// private configuration.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Owner(TelegramUserId);

impl Owner {
    /// The owner whose Telegram user id is `user`.
    #[must_use]
    pub const fn new(user: TelegramUserId) -> Self {
        Self(user)
    }

    /// The owner the credential [`OWNER_USER_ID`] names.
    ///
    /// # Errors
    ///
    /// [`IdentityError::Credential`] when the credential is missing or unreadable, and
    /// [`IdentityError::Malformed`] when it is not a positive whole number.
    pub fn load(loader: &CredentialLoader) -> Result<Self, IdentityError> {
        let value = loader.load(OWNER_USER_ID)?;
        let text = value.expose().trim();
        let user = (!text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| text.parse::<i64>().ok())
            .flatten()
            .filter(|user| *user > 0)
            .ok_or(IdentityError::Malformed {
                id: OWNER_USER_ID,
                expected: "a positive whole number, the owner's Telegram user id",
            })?;
        Ok(Self(TelegramUserId::new(user)))
    }

    /// Whether `user` is the owner.
    #[must_use]
    pub fn is(self, user: TelegramUserId) -> bool {
        self.0 == user
    }

    /// The owner's Telegram user id.
    #[must_use]
    pub const fn user(self) -> TelegramUserId {
        self.0
    }
}

impl fmt::Debug for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Owner(..)")
    }
}

/// The owner's gate: Telegram's launch data, validated with the bot's key and fresh by the bound,
/// then pinned to the owner.
#[derive(Clone, Debug)]
pub struct OwnerGate {
    key: WebAppKey,
    owner: Owner,
    freshness: Freshness,
}

impl OwnerGate {
    /// The gate that admits `owner`'s launch data, signed with `key` and fresh by `freshness`.
    #[must_use]
    pub const fn new(key: WebAppKey, owner: Owner, freshness: Freshness) -> Self {
        Self {
            key,
            owner,
            freshness,
        }
    }

    /// The gate over the two credentials `loader` reads: [`OWNER_USER_ID`], then
    /// [`TELEGRAM_BOT_TOKEN`].
    ///
    /// # Errors
    ///
    /// [`IdentityError`] naming the first credential that is missing, unreadable or malformed.
    pub fn load(loader: &CredentialLoader, freshness: Freshness) -> Result<Self, IdentityError> {
        let owner = Owner::load(loader)?;
        let token = loader.load(TELEGRAM_BOT_TOKEN)?;
        if token.expose().trim().is_empty() {
            return Err(IdentityError::Malformed {
                id: TELEGRAM_BOT_TOKEN,
                expected: "the bot's token, not blank",
            });
        }
        Ok(Self::new(
            WebAppKey::from_bot_token(token.expose()),
            owner,
            freshness,
        ))
    }

    /// The owner this gate admits.
    #[must_use]
    pub const fn owner(&self) -> Owner {
        self.owner
    }

    /// The owner, when the raw launch data `raw` is valid and fresh at `now` and its user is the
    /// owner. The outcome is logged; a refusal by its reason code alone.
    ///
    /// # Errors
    ///
    /// The [`Refusal`] of [`crate::init_data::validate`], or [`Refusal::NotOwner`] when valid,
    /// fresh launch data names another user.
    pub fn admit(&self, raw: &str, now: UtcMillis) -> Result<Owner, Refusal> {
        let admitted = validate(raw, &self.key, self.freshness, now).and_then(|caller| {
            if self.owner.is(caller.user()) {
                Ok(self.owner)
            } else {
                Err(Refusal::NotOwner)
            }
        });
        match admitted {
            Ok(_) => tracing::info!("the owner's launch data was admitted"),
            Err(refusal) => tracing::warn!(reason = refusal.reason(), "a handshake was refused"),
        }
        admitted
    }
}
