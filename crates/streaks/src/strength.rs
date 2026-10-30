//! Habit strength (SPEC-076 R12), the predecessor's `gamification/strength.py` at `27ee2bc`.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

use crate::constants::{STRENGTH_HALF_LIFE_DAYS, STRENGTH_PERSIST_DAYS};

/// The per-day multiplier `0.5 ** (1 / half_life)`.
#[must_use]
pub fn decay() -> f64 {
    0.5_f64.powf(1.0 / f64::from(STRENGTH_HALF_LIFE_DAYS))
}

/// Yesterday's strength advanced by one day.
#[must_use]
pub fn advance(prev: f64, studied: bool) -> f64 {
    let d = decay();
    let target = if studied { 1.0 } else { 0.0 };
    prev.clamp(0.0, 1.0) * d + target * (1.0 - d)
}

/// Strength for each day from the first study day to `today`, oldest first.
#[must_use]
pub fn fold(study_days: &BTreeSet<StudyDay>, today: StudyDay) -> Vec<(StudyDay, f64)> {
    let Some(first) = study_days.iter().next().copied() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut value = 0.0_f64;
    for number in first.epoch_day()..=today.epoch_day() {
        let day = StudyDay::from_epoch_day(number);
        value = advance(value, study_days.contains(&day));
        out.push((day, value));
    }
    out
}

/// Whether the fold stores `day`'s value: today and yesterday always, and on the first run every
/// day of the persist window.
#[must_use]
pub fn persists(day: StudyDay, today: StudyDay, first_run: bool) -> bool {
    let age = today.epoch_day() - day.epoch_day();
    age <= 1 || (first_run && age <= STRENGTH_PERSIST_DAYS)
}
