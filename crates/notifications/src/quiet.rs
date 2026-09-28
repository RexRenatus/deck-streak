//! The quiet window (SPEC-041 R9): the predecessor's `quiet_hours.py:in_quiet_hours`, ported, the
//! minute of the local day an instant falls in, and the clock time the policy writes a window in.
//!
//! The window is `[start, end)` in minutes of the local day. It wraps midnight when the start is
//! later than the end (the policy's 23:00 to 07:30), and a start equal to its end disables it. The
//! golden `in_quiet_hours.json` holds the port to the predecessor's own answers (A8).

use deck_streak_kernel::{UtcMillis, UtcOffset};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One day, in minutes.
pub const MINUTES_PER_DAY: i64 = 24 * 60;

/// One minute, in milliseconds.
const MINUTE_MS: i64 = 60_000;

/// Whether the minute `local_minutes` of the local day falls in the window from `start_min` to
/// `end_min`: the predecessor's `quiet_hours.py:in_quiet_hours`. The minute is taken modulo a day
/// first, so a minute before 0 or past the day's last names a minute of the day before or after.
#[must_use]
pub fn in_quiet_hours(local_minutes: i64, start_min: i64, end_min: i64) -> bool {
    if start_min == end_min {
        return false;
    }
    let minute = local_minutes.rem_euclid(MINUTES_PER_DAY);
    if start_min < end_min {
        start_min <= minute && minute < end_min
    } else {
        minute >= start_min || minute < end_min
    }
}

/// The minute of the local day, at `offset`, that `instant` falls in: 0 to 1439.
#[must_use]
pub fn local_minute(instant: UtcMillis, offset: UtcOffset) -> i64 {
    (instant.epoch_millis().div_euclid(MINUTE_MS) + i64::from(offset.minutes()))
        .rem_euclid(MINUTES_PER_DAY)
}

/// A time of day as the policy writes it, `HH:MM` on a 24-hour clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockTime(u16);

impl ClockTime {
    /// The time `text` names, or `None` unless it is `HH:MM` with an hour to 23 and a minute to 59.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (hours, minutes) = text.split_once(':')?;
        let two_digits = |part: &str| {
            (part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_digit()))
                .then(|| part.parse::<u16>().ok())
                .flatten()
        };
        let (hours, minutes) = (two_digits(hours)?, two_digits(minutes)?);
        (hours < 24 && minutes < 60).then_some(Self(hours * 60 + minutes))
    }

    /// Minutes since midnight.
    #[must_use]
    pub fn minutes(self) -> i64 {
        i64::from(self.0)
    }
}

impl Serialize for ClockTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:02}:{:02}", self.0 / 60, self.0 % 60))
    }
}

impl<'de> Deserialize<'de> for ClockTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text)
            .ok_or_else(|| serde::de::Error::custom("a time of day is HH:MM, from 00:00 to 23:59"))
    }
}
