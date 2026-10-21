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

const MINUTE_MS: i128 = 60_000;
const HOUR_MS: i128 = 3_600_000;
const DAY_MS: i128 = 86_400_000;
/// Days from the proleptic Gregorian 0000-03-01 to 1970-01-01: the calendar below counts eras of
/// 400 years from a March 1st, so that a leap day is the last day of its year.
const EPOCH_FROM_MARCH_ZERO: i128 = 719_468;
/// Days in 400 Gregorian years, one era.
const DAYS_PER_ERA: i128 = 146_097;

/// The rollover hour when none is configured: the predecessor's
/// `constants.py:DEFAULT_ROLLOVER_HOUR`, proved by `goldens/kernel.constants.json`.
pub const DEFAULT_ROLLOVER_HOUR: u8 = 4;

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

    /// The study day `instant` falls in: `floor((t + m*60000 - h*3600000) / 86400000)` for the
    /// instant `t` in milliseconds, the offset `m` in minutes and the rollover hour `h`, in integer
    /// arithmetic wide enough that no instant overflows it.
    #[must_use]
    pub fn study_day(self, instant: UtcMillis) -> StudyDay {
        let local = i128::from(instant.epoch_millis())
            + i128::from(self.utc_offset.minutes()) * MINUTE_MS
            - i128::from(self.rollover_hour.get()) * HOUR_MS;
        // A day of an i64 of milliseconds always fits an i64, so the fallback is never taken.
        StudyDay(i64::try_from(local.div_euclid(DAY_MS)).unwrap_or(i64::MAX))
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
    /// The ISO calendar date (proleptic Gregorian), `YYYY-MM-DD`; a year outside 0000 to 9999
    /// carries its sign, as ISO 8601's expanded years do.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = civil_from_days(self.0);
        if (0..=9999).contains(&year) {
            write!(f, "{year:04}-{month:02}-{day:02}")
        } else {
            write!(f, "{year:+05}-{month:02}-{day:02}")
        }
    }
}

impl FromStr for StudyDay {
    type Err = IsoDateError;

    /// Parses the ISO calendar date [`fmt::Display`] renders back to its study day, and nothing
    /// else: a date the calendar does not have, or text of another shape, is refused.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (signed, rest) = match text.as_bytes().first() {
            Some(b'+') => (Some(false), &text[1..]),
            Some(b'-') => (Some(true), &text[1..]),
            _ => (None, text),
        };
        let mut parts = rest.split('-');
        let (Some(year), Some(month), Some(day), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(IsoDateError);
        };
        let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        let year_shaped = match signed {
            None => year.len() == 4,
            Some(_) => (4..=18).contains(&year.len()),
        };
        if !(year_shaped
            && all_digits(year)
            && month.len() == 2
            && all_digits(month)
            && day.len() == 2
            && all_digits(day))
        {
            return Err(IsoDateError);
        }
        let magnitude: i128 = year.parse().map_err(|_| IsoDateError)?;
        let year = match signed {
            // A sign is written only on a year 0000 to 9999 cannot hold.
            Some(true) if magnitude > 0 => -magnitude,
            Some(false) if magnitude > 9999 => magnitude,
            Some(_) => return Err(IsoDateError),
            None => magnitude,
        };
        let month: u8 = month.parse().map_err(|_| IsoDateError)?;
        let day: u8 = day.parse().map_err(|_| IsoDateError)?;
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return Err(IsoDateError);
        }
        i64::try_from(days_from_civil(year, month, day))
            .map(Self)
            .map_err(|_| IsoDateError)
    }
}

/// Whether `year` of the proleptic Gregorian calendar has a 29th of February.
fn is_leap_year(year: i128) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// How many days `month` of `year` has.
fn days_in_month(year: i128, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 31,
    }
}

/// The civil date of an epoch day, in integer arithmetic (Howard Hinnant's `civil_from_days`).
fn civil_from_days(epoch_day: i64) -> (i128, u8, u8) {
    let from_march_zero = i128::from(epoch_day) + EPOCH_FROM_MARCH_ZERO;
    let era = from_march_zero.div_euclid(DAYS_PER_ERA);
    let day_of_era = from_march_zero.rem_euclid(DAYS_PER_ERA);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + i128::from(month <= 2);
    // A month is 1 to 12 and a day 1 to 31, so neither fallback is ever taken.
    (
        year,
        u8::try_from(month).unwrap_or(1),
        u8::try_from(day).unwrap_or(1),
    )
}

/// The epoch day of a civil date, in integer arithmetic (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i128, month: u8, day: u8) -> i128 {
    let year = year - i128::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month = i128::from(month);
    let month_from_march = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_from_march + 2) / 5 + i128::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * DAYS_PER_ERA + day_of_era - EPOCH_FROM_MARCH_ZERO
}
