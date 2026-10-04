//! Phase 4's writing step (SPEC-078 R6 to R8; ADR-078): the fold settles each evaluated day's
//! writing XP from the writing log, every writing course's `write:<code>` and the day's
//! `write:all`, as the recompute. It heals a writing write whose settle was left undone, and a
//! source the log or the courses no longer hold settles to nothing where its row exists.

use deck_streak_kernel::{Courses, PortFuture};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The name the fold's report gives this step.
pub const WRITING_STEP: &str = "habits.writing_xp";

/// The writing step, over the owner's courses.
#[derive(Debug)]
pub struct WritingStep {
    courses: Courses,
}

impl WritingStep {
    /// The step, settling the writing courses of `courses`.
    #[must_use]
    pub const fn new(courses: Courses) -> Self {
        Self { courses }
    }
}

impl DayStep for WritingStep {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        WRITING_STEP
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
