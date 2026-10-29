//! The engine's day (SPEC-514 R6): the day number a due date counts from and the instant the next
//! day begins, as the pinned engine computes them for a collection whose creation offset is set
//! (`rslib/src/scheduler/timing.rs`, `sched_timing_today_v2_new`).
//!
//! The engine counts the local calendar days from the collection's creation to the study day of
//! now, where the study day turns over at the rollover hour in the current offset: exactly the
//! study day crate's rule, so it is computed with that rule and nothing else.

use deck_streak_study_day::{StudyDayRule, UtcOffset};

/// The engine's day for one instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineDay {
    /// Days from the collection's creation to today; never negative.
    pub days_elapsed: i64,
    /// The instant today ends and the next day begins, in seconds.
    pub next_day_at: i64,
    /// The instant itself, in seconds.
    pub now: i64,
}

impl EngineDay {
    /// The stub's day: zero days elapsed and no rollover.
    #[must_use]
    pub fn at(created: i64, creation: UtcOffset, now: i64, rule: StudyDayRule) -> Self {
        let _ = (created, creation, rule);
        Self {
            days_elapsed: 0,
            next_day_at: 0,
            now,
        }
    }
}
