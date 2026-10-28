//! The instant the study day is computed from: whole milliseconds since the Unix epoch, in UTC
//! (SPEC-020 R6; SPEC-497 R1). Nothing here reads a clock: the kernel's `SystemClock` reads the
//! system time and hands it to [`UtcMillis::from_system_time`].

use std::time::{SystemTime, UNIX_EPOCH};

/// An instant: whole milliseconds since the Unix epoch, in UTC.
///
/// It is the one representation of an instant in the kernel, in the goldens (ADR-029) and in the
/// database, so no layer converts between two. It has no arithmetic: a duration is computed by the
/// rule that needs it, from [`UtcMillis::epoch_millis`], never by adding two instants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcMillis(i64);

impl UtcMillis {
    /// The instant `millis` milliseconds after the Unix epoch, or before it when negative.
    #[must_use]
    pub const fn from_epoch_millis(millis: i64) -> Self {
        Self(millis)
    }

    /// Whole milliseconds since the Unix epoch.
    #[must_use]
    pub const fn epoch_millis(self) -> i64 {
        self.0
    }

    /// The instant `time` names: whole milliseconds since the Unix epoch, negative before it,
    /// saturating at the ends of an `i64`. The kernel's `SystemClock` reads the system time through
    /// it, and it is apart from that read so a test can hand it any time, one before the epoch too.
    #[must_use]
    pub fn from_system_time(time: SystemTime) -> Self {
        Self(match time.duration_since(UNIX_EPOCH) {
            Ok(since) => i64::try_from(since.as_millis()).unwrap_or(i64::MAX),
            Err(before) => {
                i64::try_from(before.duration().as_millis()).map_or(i64::MIN, |millis| -millis)
            }
        })
    }
}
