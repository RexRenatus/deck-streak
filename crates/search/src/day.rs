//! The engine's day (SPEC-514 R6): the day number a due date counts from and the instant the next
//! day begins, as the pinned engine computes them for a collection whose creation offset is set
//! (`rslib/src/scheduler/timing.rs`, `sched_timing_today_v2_new`).
//!
//! The engine counts the local calendar days from the collection's creation to the study day of
//! now, where the study day turns over at the rollover hour in the current offset: exactly the
//! study day crate's rule, so it is computed with that rule and nothing else.

use deck_streak_study_day::{Hour, StudyDayRule, UtcMillis, UtcOffset};

/// Seconds in a day.
const DAY: i64 = 86_400;

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
    /// The engine's day at `now` (seconds) for a collection created at `created` (seconds) in the
    /// offset `creation`, read in the offset and rollover hour of `rule`.
    #[must_use]
    pub fn at(created: i64, creation: UtcOffset, now: i64, rule: StudyDayRule) -> Self {
        let today = rule
            .study_day(UtcMillis::from_epoch_millis(now.saturating_mul(1000)))
            .epoch_day();
        let created_on = Hour::new(0).map_or(0, |midnight| {
            StudyDayRule::new(midnight, creation)
                .study_day(UtcMillis::from_epoch_millis(created.saturating_mul(1000)))
                .epoch_day()
        });
        let rollover = i64::from(rule.rollover_hour().get()) * 3_600;
        let offset = i64::from(rule.utc_offset().minutes()) * 60;
        Self {
            days_elapsed: today.saturating_sub(created_on).max(0),
            next_day_at: today
                .saturating_add(1)
                .saturating_mul(DAY)
                .saturating_add(rollover)
                .saturating_sub(offset),
            now,
        }
    }
}
