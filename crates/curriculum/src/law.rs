//! The law track's mastery pillar (SPEC-077 R9) and the law dues' stored value (R10).

use deck_streak_kernel::StudyDay;

/// Points one active law leech takes off the pillar.
pub const MASTERY_LEECH_PENALTY: f64 = 3.0;
/// The most the leeches take off the pillar.
pub const MASTERY_LEECH_PENALTY_CAP: f64 = 30.0;

/// The law mastery pillar for `law_leech_active` active leeches.
#[must_use]
pub fn mastery_pillar(law_leech_active: i64) -> f64 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a leech count is far below 2^52"
    )]
    let penalty = (MASTERY_LEECH_PENALTY * law_leech_active as f64).min(MASTERY_LEECH_PENALTY_CAP);
    (100.0 - penalty).clamp(0.0, 100.0)
}

/// The law dues as the last recompute stored them (R10): the law track's review cards overdue and
/// due on that study day, at its collection day number. Coordination counts them with analytics'
/// card snapshot over the law cards, because curriculum depends on no other context's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LawDues {
    /// The study day they were counted for.
    pub study_day: StudyDay,
    /// The law review cards overdue on that day.
    pub backlog: u32,
    /// The law review cards due on that day.
    pub due_today: u32,
}
