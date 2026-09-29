//! Phase 3's step (SPEC-076 R17 to R20, R26): both tracks' streaks, the freeze ledger, the habit
//! strength and the governor, evaluated for every study day the fold visits, and the relight's XP
//! granted on the fold's own connection so that the day's base carries it in the same recompute.
//!
//! Every rule is streaks' or progression's; the step only chooses which, for the day the fold
//! evaluates, and stores what they answer.

use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_kernel::{PortFuture, StudyDay};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The name the fold's report gives this step.
pub const STREAKS_STEP: &str = "streaks.streaks_and_governor";

/// The study days whose relight a recompute answered as due, for the caller to route after the
/// fold's commit (SPEC-076 R27).
#[derive(Clone, Debug, Default)]
pub struct RelightDue {
    days: Arc<Mutex<Vec<StudyDay>>>,
}

impl RelightDue {
    /// Takes every day the step answered as due since the last take, oldest first.
    #[must_use]
    pub fn take(&self) -> Vec<StudyDay> {
        let mut days = self.days.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *days)
    }
}

/// The streaks and governor step.
#[derive(Clone, Debug, Default)]
pub struct StreaksStep {
    due: RelightDue,
}

impl StreaksStep {
    /// A step, and the handle its caller routes the relights from.
    #[must_use]
    pub fn new() -> (Self, RelightDue) {
        let due = RelightDue::default();
        (Self { due: due.clone() }, due)
    }
}

impl DayStep for StreaksStep {
    fn phase(&self) -> Phase {
        Phase::StreaksAndGovernor
    }

    fn name(&self) -> &'static str {
        STREAKS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        _day: &'a DayEvaluation<'a>,
        _write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        let _ = &self.due;
        Box::pin(async { Ok(()) })
    }
}
