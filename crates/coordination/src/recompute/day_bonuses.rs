//! Phase 5's step (SPEC-072 R16, R19, R21): a day's consistency and Ascendant bonuses.

use deck_streak_kernel::PortFuture;
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The name the fold's report gives this step.
pub const DAY_BONUSES_STEP: &str = "progression.derived_bonuses";

/// Progression's derived-bonuses step.
#[derive(Clone, Copy, Debug, Default)]
pub struct DayBonusesStep;

impl DayStep for DayBonusesStep {
    fn phase(&self) -> Phase {
        Phase::DerivedBonuses
    }

    fn name(&self) -> &'static str {
        DAY_BONUSES_STEP
    }

    fn evaluate<'a>(
        &'a self,
        _day: &'a DayEvaluation<'a>,
        _write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move { Ok(()) })
    }
}
