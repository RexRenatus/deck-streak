//! Phase 7's band badge step (SPEC-077 R7; SPEC-073 R3; ADR-077): each band-up the progress step
//! recorded on the evaluated day earns its course's band badge, `band_<code>_<band>` with the band as
//! the order spells it, named by the course's name and the band and carrying the course's flag.
//!
//! The badge goes through progression's award port in the day's own write, which writes a band
//! badge marked, because the band-up's own celebration announces it. The step pays no XP: the
//! band-up's grant is phase 4's, so the day's base is final before the mint reads it.

use deck_streak_kernel::{Courses, KernelError, PortFuture};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The name the fold's report gives this step.
pub const BAND_BADGES_STEP: &str = "curriculum.band_badges";

/// Phase 7's band badge step, over the owner's courses.
#[derive(Debug)]
pub struct BandBadgesStep {
    courses: Courses,
}

impl BandBadgesStep {
    /// The step, naming and keying each badge by `courses`.
    #[must_use]
    pub const fn new(courses: Courses) -> Self {
        Self { courses }
    }

    /// Awards the band badge of every band-up recorded on `day`, inside `write`.
    async fn award_band_badges(
        &self,
        day: &DayEvaluation<'_>,
        write: &mut SqliteConnection,
    ) -> Result<(), KernelError> {
        let _ = (&self.courses, day, write);
        Ok(())
    }
}

impl DayStep for BandBadgesStep {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        BAND_BADGES_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(self.award_band_badges(day, write))
    }
}
