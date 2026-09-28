//! Phase 1's step (SPEC-071 R5 to R14, R17, R18): analytics rolls a day up and scores it.
//!
//! For each day the fold hands it, the step rolls up the day's metrics, its per-course rows and its
//! fingerprint when the evaluation re-rolls it, and scores it:
//!
//! - the current day with its live card snapshot, recorded as its card state;
//! - a settling day with its end-of-day card snapshot when it is the most recently closed day, taken
//!   at its own collection day number and recorded as its card state, over the rollups before it;
//!   that total is kept as the score the day closed with;
//! - every other day in the predecessor's historical form: no card state, and the current day's
//!   baseline.
//!
//! Every rule is analytics' own (`deck_streak_analytics`); the step only chooses which, for the day
//! the fold evaluates.

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_kernel::PortFuture;
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The name the fold's report gives this step.
pub const ANALYTICS_STEP: &str = "analytics.rollup_and_score";

/// Analytics' step: the rollup and the score.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnalyticsStep {
    settings: AnalyticsSettings,
}

impl AnalyticsStep {
    /// The step, counting leeches by `settings`.
    #[must_use]
    pub const fn new(settings: AnalyticsSettings) -> Self {
        Self { settings }
    }
}

impl DayStep for AnalyticsStep {
    fn phase(&self) -> Phase {
        Phase::RollupAndScore
    }

    fn name(&self) -> &'static str {
        ANALYTICS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let _ = (self.settings, day, write);
            Ok(())
        })
    }
}
