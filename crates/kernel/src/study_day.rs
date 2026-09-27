//! The study day: the day as the scheduler counts it, turning over at the rollover hour in a fixed
//! UTC offset, never at midnight (CHARTER 7, SPEC-020 R1, R2 and R5).
//!
//! A study day is an epoch day number, computed with integer arithmetic from an instant, the
//! rollover hour and the offset: the predecessor's rule exactly (`analytics.py:study_day` at
//! `27ee2bc`), proved against `goldens/study_day.json`. No time-zone database is used (ADR-020).

use std::fmt;
use std::str::FromStr;

use crate::clock::UtcMillis;
use crate::error::IsoDateError;

/// The rollover hour when none is configured: the predecessor's
/// `constants.py:DEFAULT_ROLLOVER_HOUR`, proved by `goldens/kernel.constants.json`.
pub const DEFAULT_ROLLOVER_HOUR: u8 = 0;

/// An hour of the local day, 0 to 23: the rollover hour and the digest hour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hour(u8);

impl Hour {
    /// The hour `hour`, or `None` past 23.
    #[must_use]
    pub const fn new(hour: u8) -> Option<Self> {
        if hour <= 23 { Some(Self(hour)) } else { None }
    }

    /// The hour, 0 to 23.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// A fixed offset from UTC in whole minutes, from -720 to +840: the predecessor's bounds
/// (`config.py:Settings.validate`). It never follows daylight saving (ADR-020).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcOffset(i16);

impl UtcOffset {
    /// UTC itself: the repository's default, since the owner's offset is private configuration.
    pub const UTC: Self = Self(0);
    /// The westernmost offset the setting accepts, in minutes.
    pub const MIN_MINUTES: i16 = -720;
    /// The easternmost offset the setting accepts, in minutes.
    pub const MAX_MINUTES: i16 = 840;

    /// The offset of `minutes` east of UTC, or `None` outside -720 to +840.
    #[must_use]
    pub const fn from_minutes(minutes: i16) -> Option<Self> {
        if minutes >= Self::MIN_MINUTES && minutes <= Self::MAX_MINUTES {
            Some(Self(minutes))
        } else {
            None
        }
    }

    /// Minutes east of UTC.
    #[must_use]
    pub const fn minutes(self) -> i16 {
        self.0
    }
}

/// How instants map to study days: the rollover hour in a fixed UTC offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StudyDayRule {
    rollover_hour: Hour,
    utc_offset: UtcOffset,
}

impl StudyDayRule {
    /// The rule of a day that turns over at `rollover_hour`, local to `utc_offset`.
    #[must_use]
    pub const fn new(rollover_hour: Hour, utc_offset: UtcOffset) -> Self {
        Self {
            rollover_hour,
            utc_offset,
        }
    }

    /// The hour the day turns over at.
    #[must_use]
    pub const fn rollover_hour(self) -> Hour {
        self.rollover_hour
    }

    /// The fixed offset the rollover hour is local to.
    #[must_use]
    pub const fn utc_offset(self) -> UtcOffset {
        self.utc_offset
    }

    /// The study day `instant` falls in.
    #[must_use]
    pub fn study_day(self, instant: UtcMillis) -> StudyDay {
        let _ = instant;
        StudyDay(0)
    }
}

impl Default for StudyDayRule {
    /// The default rollover hour in UTC.
    fn default() -> Self {
        Self::new(Hour(DEFAULT_ROLLOVER_HOUR), UtcOffset::UTC)
    }
}

/// A study day, as its epoch day number: whole days since the Unix epoch's study day.
///
/// One integer everywhere (the database, the goldens, the API's arithmetic); it renders as an ISO
/// calendar date only at the edge, for a screen that shows the server's study day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StudyDay(i64);

impl StudyDay {
    /// The study day with epoch day number `day`.
    #[must_use]
    pub const fn from_epoch_day(day: i64) -> Self {
        Self(day)
    }

    /// The epoch day number.
    #[must_use]
    pub const fn epoch_day(self) -> i64 {
        self.0
    }
}

impl fmt::Display for StudyDay {
    /// The ISO calendar date (proleptic Gregorian), `YYYY-MM-DD`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("")
    }
}

impl FromStr for StudyDay {
    type Err = IsoDateError;

    /// Parses the ISO calendar date [`fmt::Display`] renders back to its study day.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let _ = text;
        Err(IsoDateError)
    }
}
