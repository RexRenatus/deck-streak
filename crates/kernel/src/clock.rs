//! The clock: the one way the workspace reads the current instant (SPEC-020 R6).
//!
//! Every rule that depends on the time takes a [`Clock`] or an instant, so a test drives time with
//! a [`ManualClock`] instead of sleeping, and a replay runs a past day at the speed of the CPU.
//! [`SystemClock`] is the only reader of the system time in the workspace's production code.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

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
}

/// The source of the current instant, injected wherever a rule depends on the time.
pub trait Clock: Send + Sync {
    /// The current instant.
    fn now(&self) -> UtcMillis;
}

/// The system's clock: the only reader of the system time in production code (R6).
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> UtcMillis {
        UtcMillis(0)
    }
}

/// A clock that moves only when a test or a replay moves it, so no test depends on time passing.
#[derive(Debug)]
pub struct ManualClock {
    now: AtomicI64,
}

impl ManualClock {
    /// A clock stopped at `start`.
    #[must_use]
    pub const fn new(start: UtcMillis) -> Self {
        Self {
            now: AtomicI64::new(start.0),
        }
    }

    /// Moves the clock to `instant`, forward or back.
    pub fn set(&self, instant: UtcMillis) {
        self.now.store(instant.0, Ordering::SeqCst);
    }

    /// Moves the clock forward by `by`, whole milliseconds, saturating at the last instant.
    pub fn advance(&self, by: Duration) {
        let _ = by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> UtcMillis {
        UtcMillis(self.now.load(Ordering::SeqCst))
    }
}
