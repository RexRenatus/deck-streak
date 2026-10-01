//! The badge step (SPEC-073 R4, R5, R8; ADR-303): phase 7 of the fold awards each study badge the
//! evaluated day earns, and the offers raise each award's celebration until the router answers.
//!
//! A closing day at its settle and the current day are evaluated (ADR-071): the day's badge context
//! is built from its rollup (the score it closed with, or the live score, and its recorded card
//! state), the language streak, the window's reviews and the lifetime, and every met study
//! condition is awarded through progression's award port in the day's own write, its mark unset.
//! An award is never celebrated from inside that write: the router opens its own, so the fold's
//! offers hand every unmarked badge to it between the writes, and set the mark in a write of its
//! own, only once the router has answered and only while the row is still unmarked.

use deck_streak_kernel::{Courses, Db, KernelError, PortFuture, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use super::{Celebrate, DayEvaluation, DayStep, Phase};

/// The badge step's name, as the fold's report and log name it.
pub const BADGES_STEP: &str = "progression.badges";

/// The ladder's event a badge celebration names.
pub const BADGE_EVENT: &str = "badge";

/// The study badges whose condition reads the card snapshot: a day with no recorded card state
/// judges none of them, rather than judging them on a snapshot it never had.
pub const SNAPSHOT_KEYS: [&str; 5] = [
    "inbox_zero",
    "backlog_slayer",
    "maturity_milestone",
    "forest_guardian",
    "leech_tamer",
];

/// The dedupe key of a badge's celebration: `badge:<key>:<tier>` (R4).
#[must_use]
pub fn badge_key(key: &str, tier: u32) -> String {
    let _ = (key, tier);
    String::new()
}

/// The line a badge's celebration carries.
#[must_use]
pub fn badge_line(emoji: &str, name: &str) -> String {
    let _ = (emoji, name);
    String::new()
}

/// Phase 7's badge step, over the owner's courses.
#[derive(Debug)]
pub struct BadgesStep {
    courses: Courses,
}

impl BadgesStep {
    /// The step, awarding against `courses`.
    #[must_use]
    pub const fn new(courses: Courses) -> Self {
        Self { courses }
    }
}

impl DayStep for BadgesStep {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        BADGES_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let _ = (&self.courses, day, write);
            Ok(())
        })
    }
}

/// Offers every badge whose mark is unset to `celebrate`, oldest first, and marks each one it
/// answered at `now`, in a write of its own (ADR-303). An offer the router did not answer leaves
/// the badge owed for the next offers.
///
/// # Errors
///
/// [`KernelError`] when the owed badges cannot be read, or a mark cannot be written.
pub async fn offer_badges(
    celebrate: &dyn Celebrate,
    db: &Db,
    now: UtcMillis,
    today: StudyDay,
) -> Result<(), KernelError> {
    let _ = (celebrate, db, now, today);
    Ok(())
}
