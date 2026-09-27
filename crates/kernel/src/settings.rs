//! Settings: parsed once, at start, from the environment the caller hands in, into typed values
//! (SPEC-020 R2, R3, R10 and R14).
//!
//! The daemon's `main` is the one reader of the process environment; everything else receives an
//! [`Environment`]. A missing required setting refuses start with [`SettingsError::Missing`], and
//! a malformed one with [`SettingsError::Malformed`], naming the setting and the shape it expects.
//! No refusal, log line or panic message carries a setting's value: a hand-written parser can
//! promise that, which a generic deserializer's messages do not (ADR-020).
//!
//! An unset setting and a blank one (empty, or whitespace only) are the same: unset, as the
//! predecessor read them (`config.py:_env_int`). A set value is read with its surrounding
//! whitespace trimmed.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::error::SettingsError;
use crate::study_day::{Hour, StudyDayRule, UtcOffset};

/// The hour the study day turns over at, 0 to 23 (default
/// [`crate::study_day::DEFAULT_ROLLOVER_HOUR`]).
pub const ROLLOVER_HOUR: &str = "DECKSTREAK_ROLLOVER_HOUR";
/// The fixed UTC offset the rollover hour is local to, in minutes (default 0).
pub const UTC_OFFSET_MINUTES: &str = "DECKSTREAK_UTC_OFFSET_MINUTES";
/// The hour the daily digest goes out, 0 to 23, never before the rollover hour.
pub const DIGEST_HOUR: &str = "DECKSTREAK_DIGEST_HOUR";
/// How many blocking operations the offload runs at once.
pub const OFFLOAD_WORKERS: &str = "DECKSTREAK_OFFLOAD_WORKERS";
/// The directory systemd passes a unit's credentials in (systemd.exec(5), `LoadCredential=`).
pub const CREDENTIALS_DIRECTORY: &str = "CREDENTIALS_DIRECTORY";

/// The digest hour when none is set, before the rollover hour raises it: the predecessor's
/// `constants.py:DEFAULT_DIGEST_HOUR`, proved by `goldens/kernel.constants.json`.
pub const DEFAULT_DIGEST_HOUR: u8 = 0;

/// The most blocking operations the offload may run at once: tokio's default cap on its blocking
/// threads, beyond which more permits could never run together anyway.
pub const MAX_OFFLOAD_WORKERS: usize = 512;

/// The process environment, as the one reader of it (the daemon's `main`) handed it in.
///
/// Its `Debug` shows the variables' names and never their values.
#[derive(Clone, Default)]
pub struct Environment {
    variables: BTreeMap<OsString, OsString>,
}

impl Environment {
    /// The environment made of `variables`, for example `std::env::vars_os()`.
    #[must_use]
    pub fn from_vars<I, K, V>(variables: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<OsString>,
        V: Into<OsString>,
    {
        Self {
            variables: variables
                .into_iter()
                .map(|(name, value)| (name.into(), value.into()))
                .collect(),
        }
    }

    /// The setting `name`, which must be set.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Missing`] when it is unset or blank, and [`SettingsError::Malformed`]
    /// when it does not have `T`'s shape.
    pub fn required<T: Setting>(&self, name: &'static str) -> Result<T, SettingsError> {
        Err(SettingsError::Malformed {
            setting: name,
            expected: T::SHAPE,
        })
    }

    /// The setting `name`, or `None` when it is unset or blank.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] when it is set and does not have `T`'s shape.
    pub fn optional<T: Setting>(&self, name: &'static str) -> Result<Option<T>, SettingsError> {
        let _ = name;
        Ok(None)
    }
}

impl fmt::Debug for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Environment")
            .field("names", &self.variables.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// A typed setting: the shape it must have, and how to read a set value of that shape.
pub trait Setting: Sized {
    /// The shape a value must have, named in [`SettingsError::Malformed`] in place of the value.
    const SHAPE: &'static str;

    /// The value `text` (already trimmed and not blank) holds, or `None` when it has another
    /// shape.
    fn parse(text: &str) -> Option<Self>;
}

impl Setting for Hour {
    const SHAPE: &'static str = "a whole hour from 0 to 23";

    fn parse(text: &str) -> Option<Self> {
        text.parse::<i64>()
            .ok()
            .and_then(|hour| u8::try_from(hour).ok())
            .and_then(Self::new)
    }
}

impl Setting for UtcOffset {
    const SHAPE: &'static str = "whole minutes east of UTC, from -720 to 840";

    fn parse(text: &str) -> Option<Self> {
        text.parse::<i64>()
            .ok()
            .and_then(|minutes| i16::try_from(minutes).ok())
            .and_then(Self::from_minutes)
    }
}

/// How many blocking operations the offload runs at once, 1 to [`MAX_OFFLOAD_WORKERS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OffloadWorkers(usize);

impl OffloadWorkers {
    /// The bound `workers`, or `None` outside 1 to [`MAX_OFFLOAD_WORKERS`].
    #[must_use]
    pub const fn new(workers: usize) -> Option<Self> {
        if workers >= 1 && workers <= MAX_OFFLOAD_WORKERS {
            Some(Self(workers))
        } else {
            None
        }
    }

    /// The bound.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl Default for OffloadWorkers {
    /// The predecessor's `offload.py:OFFLOAD_MAX_WORKERS`.
    fn default() -> Self {
        Self(crate::offload::OFFLOAD_MAX_WORKERS)
    }
}

impl Setting for OffloadWorkers {
    const SHAPE: &'static str = "a whole number of workers from 1 to 512";

    fn parse(text: &str) -> Option<Self> {
        text.parse::<usize>().ok().and_then(Self::new)
    }
}

/// The directory systemd passes a unit's credentials in, as `$CREDENTIALS_DIRECTORY` (R11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialsDirectory(PathBuf);

impl CredentialsDirectory {
    /// The directory at `path`, or `None` when the path is not absolute.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Option<Self> {
        let path = path.into();
        path.is_absolute().then_some(Self(path))
    }

    /// The directory the environment names in [`CREDENTIALS_DIRECTORY`].
    ///
    /// # Errors
    ///
    /// [`SettingsError::Missing`] when the variable is unset or blank, and
    /// [`SettingsError::Malformed`] when it is not an absolute path.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        env.required(CREDENTIALS_DIRECTORY)
    }

    /// The directory's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Setting for CredentialsDirectory {
    const SHAPE: &'static str = "an absolute directory path";

    fn parse(text: &str) -> Option<Self> {
        Self::new(text)
    }
}

/// The kernel's own settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelSettings {
    /// When the study day turns over.
    pub study_day_rule: StudyDayRule,
    /// When the daily digest goes out: never before the rollover hour.
    pub digest_hour: Hour,
    /// How many blocking operations the offload runs at once.
    pub offload_workers: OffloadWorkers,
}

impl KernelSettings {
    /// The kernel's settings, read from `env`.
    ///
    /// An unset digest hour is the larger of [`DEFAULT_DIGEST_HOUR`] and the rollover hour, as the
    /// predecessor resolved it (`config.py:_env_digest_hour`), so a default can never refuse
    /// start; only an explicitly set digest hour earlier than the rollover hour is refused.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] for a setting of the wrong shape, and
    /// [`SettingsError::DigestBeforeRollover`] for an explicit digest hour earlier than the
    /// rollover hour.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        let default_rule = StudyDayRule::default();
        let rollover_hour = env
            .optional::<Hour>(ROLLOVER_HOUR)?
            .unwrap_or(default_rule.rollover_hour());
        let utc_offset = env
            .optional::<UtcOffset>(UTC_OFFSET_MINUTES)?
            .unwrap_or(default_rule.utc_offset());
        let digest_hour = match env.optional::<Hour>(DIGEST_HOUR)? {
            // Both hours are at most 23, so the larger of them is an hour.
            None => {
                Hour::new(DEFAULT_DIGEST_HOUR.max(rollover_hour.get())).unwrap_or(rollover_hour)
            }
            Some(explicit) if explicit < rollover_hour => {
                return Err(SettingsError::DigestBeforeRollover {
                    digest: DIGEST_HOUR,
                    rollover: ROLLOVER_HOUR,
                });
            }
            Some(explicit) => explicit,
        };
        let offload_workers = env
            .optional::<OffloadWorkers>(OFFLOAD_WORKERS)?
            .unwrap_or_default();
        Ok(Self {
            study_day_rule: StudyDayRule::new(rollover_hour, utc_offset),
            digest_hour,
            offload_workers,
        })
    }
}
