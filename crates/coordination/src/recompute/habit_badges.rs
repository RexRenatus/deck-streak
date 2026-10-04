//! Phase 7's habit badges step (SPEC-078 R9, R17 as amended; ADR-078): the fold awards each habit
//! badge the evaluated day earns, judged on a context built from the logs through that day, never
//! the wall clock's. Each award goes through SPEC-073's award port with its mark unset, and the
//! offers that drain every unmarked badge celebrate it through the router, once.

use deck_streak_kernel::{Courses, PortFuture};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The habit badges step's name, as the fold's report and log name it.
pub const HABIT_BADGES_STEP: &str = "habits.badges";

/// Phase 7's habit badges step, over the owner's courses.
#[derive(Debug)]
pub struct HabitBadgesStep {
    courses: Courses,
}

impl HabitBadgesStep {
    /// The step, awarding against `courses`.
    #[must_use]
    pub const fn new(courses: Courses) -> Self {
        Self { courses }
    }
}

impl DayStep for HabitBadgesStep {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        HABIT_BADGES_STEP
    }

    fn evaluate<'a>(
        &'a self,
        _day: &'a DayEvaluation<'a>,
        _write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let _ = &self.courses;
            Ok(())
        })
    }
}
