//! The open lapse (SPEC-049 R12, R13; ADR-088): a pure function of the study days and the skip days.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::StudyDay;

/// The silent study days that open a lapse (`economy.json`, `governor.lapse_after_silent_days`).
pub const LAPSE_AFTER_SILENT_DAYS: u32 = 3;

/// The lapse open on `today`, by its id, if any.
#[must_use]
pub fn open_lapse(
    _today: StudyDay,
    _review_counts: &BTreeMap<StudyDay, u32>,
    _skip_days: &BTreeSet<StudyDay>,
    _silent_days_to_open: u32,
) -> Option<StudyDay> {
    None
}
