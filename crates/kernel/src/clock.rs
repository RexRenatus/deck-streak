//! The clock: the one way the workspace reads the current instant (SPEC-020 R6).
//!
//! Every rule that depends on the time takes a [`Clock`] or an instant, so a test drives time with
//! a [`ManualClock`] instead of sleeping, and a replay runs a past day at the speed of the CPU.
//! [`SystemClock`] is the only reader of the system time in the workspace's production code.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, SystemTime};

// The instant moved to the study day crate, which both clients link without the kernel
// (ADR-515); the kernel re-exports it, so this path names the crate's own type.
pub use deck_streak_study_day::UtcMillis;

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
        UtcMillis::from_system_time(SystemTime::now())
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
            now: AtomicI64::new(start.epoch_millis()),
        }
    }

    /// Moves the clock to `instant`, forward or back.
    pub fn set(&self, instant: UtcMillis) {
        self.now.store(instant.epoch_millis(), Ordering::SeqCst);
    }

    /// Moves the clock forward by `by`, whole milliseconds, saturating at the last instant.
    pub fn advance(&self, by: Duration) {
        let millis = i64::try_from(by.as_millis()).unwrap_or(i64::MAX);
        // The update never declines, so its result carries nothing to handle.
        let _previous = self
            .now
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |now| {
                Some(now.saturating_add(millis))
            });
    }
}

impl Clock for ManualClock {
    fn now(&self) -> UtcMillis {
        UtcMillis::from_epoch_millis(self.now.load(Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, SystemClock};

    #[test]
    fn the_system_clock_reads_an_instant_after_the_epoch() {
        assert!(SystemClock.now().epoch_millis() > 0);
    }
}
