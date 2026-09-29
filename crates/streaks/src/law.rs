//! The law streak (SPEC-076 R10, R11; ADR-076): the predecessor's `analytics.py:bridged_streak`,
//! evaluated on every study day, with no freeze and no comeback.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

use crate::streak::StreakState;

/// The run of law study days ending today, or yesterday when today has none yet, with skip days
/// bridging it.
#[must_use]
pub fn bridged_streak(
    days: &BTreeSet<StudyDay>,
    today: StudyDay,
    skips: &BTreeSet<StudyDay>,
) -> u32 {
    let _ = (days, today, skips);
    u32::MAX
}

/// The law row after `today`: the run it holds, its longest, and the freezes it started with.
#[must_use]
pub fn law_state(
    prev: &StreakState,
    days: &BTreeSet<StudyDay>,
    today: StudyDay,
    skips: &BTreeSet<StudyDay>,
) -> StreakState {
    let _ = (days, today, skips);
    StreakState {
        freezes: prev.freezes + 1,
        ..*prev
    }
}
