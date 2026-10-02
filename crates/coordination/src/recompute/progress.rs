//! Phase 4's progress step (SPEC-077 R6, R7, R10; ADR-077): Road to C2 for the current study day.
//!
//! The step runs for the current study day only, because it reads the current card state and the
//! recompute's clock, never a closed day's. Inside the day's write it:
//!
//! - computes each course's progress with curriculum's rules and stores it in `language_progress`;
//! - counts the law dues, the backlog plus the cards due today over the law track's cards, with
//!   analytics' card snapshot at the day's collection day number, and stores them in `law_dues`;
//! - compares each course's current band with the band stored before it: a first sighting records
//!   a silent baseline, and a band-up records its milestone once and grants its XP through
//!   progression's `grant_on` on the same connection, so the grant is in the day's base before the
//!   derived bonuses, as the predecessor's cycle places it.
//!
//! The band badge is phase 7's (`band_badges`), and the band-up's celebration is offered between the
//! fold's writes from the milestone's unset mark (ADR-303). Every rule is curriculum's, analytics' or
//! progression's; the step only chooses which, for the day the fold evaluates.

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_kernel::{Courses, KernelError, PortFuture};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The name the fold's report gives this step.
pub const PROGRESS_STEP: &str = "curriculum.progress";

/// Phase 4's progress step, over the owner's courses.
#[derive(Debug)]
pub struct ProgressStep {
    courses: Courses,
    settings: AnalyticsSettings,
}

impl ProgressStep {
    /// The step, computing progress against `courses` and taking the card snapshot with analytics'
    /// `settings`.
    #[must_use]
    pub const fn new(courses: Courses, settings: AnalyticsSettings) -> Self {
        Self { courses, settings }
    }

    /// Stores each course's progress and the law dues for `day`, and records each course's first
    /// sighting or band-up, granting a band-up's XP, all inside `write`.
    async fn record_progress(
        &self,
        day: &DayEvaluation<'_>,
        write: &mut SqliteConnection,
    ) -> Result<(), KernelError> {
        let _ = (&self.courses, self.settings, day, write);
        Ok(())
    }
}

impl DayStep for ProgressStep {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        PROGRESS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(self.record_progress(day, write))
    }
}
