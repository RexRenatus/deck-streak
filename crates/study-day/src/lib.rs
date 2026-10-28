//! The study day crate's shape before the move (SPEC-497): every type and function its tests name,
//! with no behaviour. The move replaces this file.

use std::fmt;
use std::time::SystemTime;

pub use study_day::{Hour, StudyDay, StudyDayRule, UtcOffset};

/// An instant, before the move: every conversion reads zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcMillis(i64);

impl UtcMillis {
    /// Zero, whatever it is given.
    #[must_use]
    pub const fn from_epoch_millis(_millis: i64) -> Self {
        Self(0)
    }

    /// The milliseconds held.
    #[must_use]
    pub const fn epoch_millis(self) -> i64 {
        self.0
    }

    /// Zero, whatever it is given.
    #[must_use]
    pub fn from_system_time(_time: SystemTime) -> Self {
        Self(0)
    }
}

/// The date error, before the move: it reads as nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IsoDateError;

impl fmt::Display for IsoDateError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl std::error::Error for IsoDateError {}

/// The study day, before the move: every rule answers zero, nothing renders and nothing parses.
pub mod study_day {
    use std::fmt;
    use std::str::FromStr;

    use super::{IsoDateError, UtcMillis};

    /// The rollover hour, before the move.
    pub const DEFAULT_ROLLOVER_HOUR: u8 = 0;

    /// An hour, before the move: every value is admitted.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Hour(u8);

    impl Hour {
        /// Any value, as an hour.
        #[must_use]
        pub const fn new(hour: u8) -> Option<Self> {
            Some(Self(hour))
        }

        /// The value held.
        #[must_use]
        pub const fn get(self) -> u8 {
            self.0
        }
    }

    /// An offset, before the move: every value is admitted.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct UtcOffset(i16);

    impl UtcOffset {
        /// Any value, as an offset.
        #[must_use]
        pub const fn from_minutes(minutes: i16) -> Option<Self> {
            Some(Self(minutes))
        }

        /// The value held.
        #[must_use]
        pub const fn minutes(self) -> i16 {
            self.0
        }
    }

    /// A rule, before the move: every instant falls on day zero.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct StudyDayRule {
        rollover_hour: Hour,
        utc_offset: UtcOffset,
    }

    impl StudyDayRule {
        /// The rule of the two values.
        #[must_use]
        pub const fn new(rollover_hour: Hour, utc_offset: UtcOffset) -> Self {
            Self {
                rollover_hour,
                utc_offset,
            }
        }

        /// Day zero, whatever the instant.
        #[must_use]
        pub fn study_day(self, _instant: UtcMillis) -> StudyDay {
            StudyDay(0)
        }
    }

    /// A study day, before the move.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct StudyDay(i64);

    impl StudyDay {
        /// The day `day`.
        #[must_use]
        pub const fn from_epoch_day(day: i64) -> Self {
            Self(day)
        }

        /// The day held.
        #[must_use]
        pub const fn epoch_day(self) -> i64 {
            self.0
        }
    }

    impl fmt::Display for StudyDay {
        fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
            Ok(())
        }
    }

    impl FromStr for StudyDay {
        type Err = IsoDateError;

        fn from_str(_text: &str) -> Result<Self, Self::Err> {
            Err(IsoDateError)
        }
    }
}
