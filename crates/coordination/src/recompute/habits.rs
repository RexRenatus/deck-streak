//! Phase 4's habit step (SPEC-078 R5; ADR-078).

use deck_streak_kernel::PortFuture;
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The name the fold's report gives this step.
pub const HABITS_STEP: &str = "habits.reading_xp";

/// Habits' step.
#[derive(Clone, Copy, Debug, Default)]
pub struct HabitsStep;

impl DayStep for HabitsStep {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        HABITS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let _ = (day, write);
            Ok(())
        })
    }
}
