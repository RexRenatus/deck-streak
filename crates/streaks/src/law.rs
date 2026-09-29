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
    let Some(oldest) = days.iter().next().copied() else {
        return 0;
    };
    let mut cursor = today.epoch_day();
    if !days.contains(&today) {
        cursor -= 1;
    }
    let mut run: u32 = 0;
    while cursor >= oldest.epoch_day() {
        let day = StudyDay::from_epoch_day(cursor);
        if days.contains(&day) {
            run = run.saturating_add(1);
        } else if !skips.contains(&day) {
            break;
        }
        cursor -= 1;
    }
    run
}

/// The law row after `today`: the run it holds, its longest, and the freezes it started with.
#[must_use]
pub fn law_state(
    prev: &StreakState,
    days: &BTreeSet<StudyDay>,
    today: StudyDay,
    skips: &BTreeSet<StudyDay>,
) -> StreakState {
    let current = bridged_streak(days, today, skips);
    StreakState {
        current,
        longest: prev.longest.max(current),
        freezes: prev.freezes,
        last_study_day: days
            .range(..=today)
            .next_back()
            .copied()
            .or(prev.last_study_day),
        comeback_armed: false,
    }
}
