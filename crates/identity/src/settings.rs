//! The freshness bound of Telegram's launch data (SPEC-024 R2; ADR-024).
//!
//! A leaked launch string must not stay usable for long (web-security's `ws.tg-init-data-fresh`),
//! so a handshake admits launch data only while its `auth_date` is at most the bound old, by
//! default [`DEFAULT_MAX_AGE`], and at most [`FUTURE_SKEW`] ahead of the server's clock, which
//! covers ordinary clock skew. The bound is the setting [`INIT_DATA_MAX_AGE`], in whole seconds; as
//! every setting, a refusal names it and never its value (SPEC-020 R10).

use std::time::Duration;

use deck_streak_kernel::{Environment, Setting, SettingsError};

/// How old the launch data may be, in whole seconds, 1 to 86400 (default 3600).
pub const INIT_DATA_MAX_AGE: &str = "DECKSTREAK_INIT_DATA_MAX_AGE_SECONDS";
/// The bound when none is set: an hour covers a Mini App launch and its re-handshakes (ADR-024).
pub const DEFAULT_MAX_AGE: Duration = Duration::from_hours(1);
/// The longest bound the setting takes: a day, the common library default ADR-024 turned down as
/// the default; beyond it a leaked launch string outlives any use of it.
pub const LONGEST_MAX_AGE: Duration = Duration::from_hours(24);
/// How far ahead of the server's clock an `auth_date` may be: ordinary clock skew (ADR-024).
pub const FUTURE_SKEW: Duration = Duration::from_mins(1);

/// How fresh launch data must be: no older than its bound, and no further ahead than the skew.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Freshness {
    max_age: Duration,
}

impl Freshness {
    /// A bound of `max_age`, or `None` unless it is a whole number of seconds from 1 to
    /// [`LONGEST_MAX_AGE`]'s.
    #[must_use]
    pub const fn new(max_age: Duration) -> Option<Self> {
        let seconds = max_age.as_secs();
        if max_age.subsec_nanos() == 0 && seconds >= 1 && seconds <= LONGEST_MAX_AGE.as_secs() {
            Some(Self { max_age })
        } else {
            None
        }
    }

    /// How old launch data may be.
    #[must_use]
    pub const fn max_age(self) -> Duration {
        self.max_age
    }

    /// How far ahead of the server's clock launch data may be dated.
    #[must_use]
    pub const fn future_skew(self) -> Duration {
        FUTURE_SKEW
    }

    /// The bound [`INIT_DATA_MAX_AGE`] sets, or the default when it is unset.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] when it is set and is not a whole number of seconds from 1 to
    /// 86400.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        Ok(env.optional(INIT_DATA_MAX_AGE)?.unwrap_or_default())
    }
}

impl Default for Freshness {
    fn default() -> Self {
        Self {
            max_age: DEFAULT_MAX_AGE,
        }
    }
}

impl Setting for Freshness {
    const SHAPE: &'static str = "a whole number of seconds from 1 to 86400";

    fn parse(text: &str) -> Option<Self> {
        text.parse::<u64>()
            .ok()
            .and_then(|seconds| Self::new(Duration::from_secs(seconds)))
    }
}
