//! Analytics' one setting (SPEC-071 R8): the leech threshold the card snapshot counts leeches by.

use deck_streak_kernel::{Environment, Setting, SettingsError};

/// The lapses at or above which a card that is not suspended is a leech.
pub const LEECH_THRESHOLD: &str = "DECKSTREAK_LEECH_THRESHOLD";
/// The leech threshold when none is set: the predecessor's `constants.DEFAULT_LEECH_THRESHOLD`,
/// proved by `goldens/analytics.constants.json` and by the card snapshot's golden, whose cases
/// without a threshold take the predecessor's default.
pub const DEFAULT_LEECH_THRESHOLD: i64 = 8;
/// The largest leech threshold a setting may name.
pub const MAX_LEECH_THRESHOLD: i64 = 1_000;

/// A leech threshold: a whole number of lapses from 1 to [`MAX_LEECH_THRESHOLD`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeechThreshold(i64);

impl LeechThreshold {
    /// The threshold `lapses`, or `None` outside 1 to [`MAX_LEECH_THRESHOLD`].
    #[must_use]
    pub const fn new(lapses: i64) -> Option<Self> {
        if lapses >= 1 && lapses <= MAX_LEECH_THRESHOLD {
            Some(Self(lapses))
        } else {
            None
        }
    }

    /// The threshold's lapses.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl Default for LeechThreshold {
    fn default() -> Self {
        Self(DEFAULT_LEECH_THRESHOLD)
    }
}

impl Setting for LeechThreshold {
    const SHAPE: &'static str = "a whole number of lapses from 1 to 1000";

    fn parse(text: &str) -> Option<Self> {
        text.parse::<i64>().ok().and_then(Self::new)
    }
}

/// Analytics' settings, read once at start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnalyticsSettings {
    /// The leech threshold the card snapshot counts by.
    pub leech_threshold: LeechThreshold,
}

impl AnalyticsSettings {
    /// Analytics' settings, read from `env`.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] when the leech threshold is set and not a threshold.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        Ok(Self {
            leech_threshold: env
                .optional::<LeechThreshold>(LEECH_THRESHOLD)?
                .unwrap_or_default(),
        })
    }
}
