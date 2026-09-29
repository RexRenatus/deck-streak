//! Phase 2's step (SPEC-072 R4, R5, R11, R12, R19, R20): a day's review XP and daily bonuses.

use deck_streak_kernel::PortFuture;
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The name the fold's report gives this step.
pub const XP_STEP: &str = "progression.base_xp";

/// Progression's base-XP step.
#[derive(Clone, Copy, Debug, Default)]
pub struct XpStep;

impl DayStep for XpStep {
    fn phase(&self) -> Phase {
        Phase::BaseXp
    }

    fn name(&self) -> &'static str {
        XP_STEP
    }

    fn evaluate<'a>(
        &'a self,
        _day: &'a DayEvaluation<'a>,
        _write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move { Ok(()) })
    }
}
