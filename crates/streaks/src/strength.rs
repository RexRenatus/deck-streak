//! Habit strength (SPEC-076 R12), the predecessor's `gamification/strength.py` at `27ee2bc`.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

/// The per-day multiplier `0.5 ** (1 / half_life)`.
#[must_use]
pub fn decay() -> f64 {
    0.0
}

/// Yesterday's strength advanced by one day.
#[must_use]
pub fn advance(prev: f64, studied: bool) -> f64 {
    let _ = studied;
    prev
}

/// Strength for each day from the first study day to `today`, oldest first.
#[must_use]
pub fn fold(study_days: &BTreeSet<StudyDay>, today: StudyDay) -> Vec<(StudyDay, f64)> {
    let _ = (study_days, today);
    Vec::new()
}

/// Whether the fold stores `day`'s value: today and yesterday always, and on the first run every
/// day of the persist window.
#[must_use]
pub fn persists(day: StudyDay, today: StudyDay, first_run: bool) -> bool {
    let _ = (day, today, first_run);
    false
}
