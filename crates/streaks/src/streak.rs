//! The language streak's machine (SPEC-076 R1 to R6), the predecessor's
//! `gamification/streak.py` at `27ee2bc`: one gap classifier, the study transition and the lapse
//! transition, over epoch days with no clock.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

use crate::constants::{
    STREAK_COMEBACK_MIN, STREAK_DAYS_PER_FREEZE, STREAK_FREEZE_CAP, STREAK_HEAT,
    STREAK_START_FREEZES,
};

/// One track's persisted streak.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreakState {
    /// The run's length.
    pub current: u32,
    /// The longest run held.
    pub longest: u32,
    /// Freezes banked.
    pub freezes: u32,
    /// The study day the run last advanced on.
    pub last_study_day: Option<StudyDay>,
    /// Whether a broken streak of the comeback's length is waiting to be won back.
    pub comeback_armed: bool,
}

impl StreakState {
    /// A track's start state: no run, the start freezes, no study day.
    #[must_use]
    pub const fn start() -> Self {
        Self {
            current: 0,
            longest: 0,
            freezes: STREAK_START_FREEZES,
            last_study_day: None,
            comeback_armed: false,
        }
    }
}

/// What the gap between the last study day and a day means for the run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapOutcome {
    /// No study day yet.
    Bootstrap,
    /// The day was already counted.
    SameDay,
    /// Every day between was studied or skipped.
    Continue,
    /// One real miss, and a freeze covers it.
    Freeze,
    /// The run is lost.
    Break,
}

impl GapOutcome {
    /// The predecessor's name for the outcome.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::SameDay => "same_day",
            Self::Continue => "continue",
            Self::Freeze => "freeze",
            Self::Break => "break",
        }
    }
}

/// A state after a transition, with the flags that live only for the day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    /// The new state.
    pub state: StreakState,
    /// A freeze was spent today.
    pub froze_today: bool,
    /// The run broke today.
    pub broke_today: bool,
}

/// The heat emoji for a streak of `days`.
#[must_use]
pub fn heat_for(days: u32) -> &'static str {
    STREAK_HEAT
        .iter()
        .find(|(threshold, _)| days >= *threshold)
        .map_or("", |(_, emoji)| emoji)
}

/// Study days missed between `last` and `today`, not counting skip days: the gap less one, less the
/// skip days strictly between the two, never below zero.
#[must_use]
pub fn real_misses(last: StudyDay, today: StudyDay, skips: &BTreeSet<StudyDay>) -> u32 {
    let between = i64::try_from(
        skips
            .iter()
            .filter(|day| last < **day && **day < today)
            .count(),
    )
    .unwrap_or(i64::MAX);
    let missed = today
        .epoch_day()
        .saturating_sub(last.epoch_day())
        .saturating_sub(1)
        .saturating_sub(between);
    u32::try_from(missed.max(0)).unwrap_or(u32::MAX)
}

/// The gap's outcome: the ONE entrance the study path and the lapse path share, so what counts as
/// a break never diverges between them.
#[must_use]
pub fn classify_gap(prev: &StreakState, today: StudyDay, skips: &BTreeSet<StudyDay>) -> GapOutcome {
    let Some(last) = prev.last_study_day else {
        return GapOutcome::Bootstrap;
    };
    if last == today {
        return GapOutcome::SameDay;
    }
    match real_misses(last, today, skips) {
        0 => GapOutcome::Continue,
        1 if prev.freezes > 0 => GapOutcome::Freeze,
        _ => GapOutcome::Break,
    }
}

/// The state after a study event on `today`.
#[must_use]
pub fn update_on_study(
    prev: &StreakState,
    today: StudyDay,
    observed_streak: u32,
    skips: &BTreeSet<StudyDay>,
) -> Transition {
    let outcome = classify_gap(prev, today, skips);
    if outcome == GapOutcome::SameDay {
        return Transition {
            state: *prev,
            froze_today: false,
            broke_today: false,
        };
    }
    let mut froze = false;
    let mut broke = false;
    let mut comeback = prev.comeback_armed;
    let mut freezes = prev.freezes;
    let current = match outcome {
        GapOutcome::Bootstrap => observed_streak.max(1),
        GapOutcome::Continue => prev.current.saturating_add(1),
        GapOutcome::Freeze => {
            froze = true;
            freezes = freezes.saturating_sub(1);
            prev.current.saturating_add(1)
        }
        GapOutcome::Break | GapOutcome::SameDay => {
            // A break already recorded during the lapse is not counted twice: start the fresh run.
            if prev.current > 0 {
                broke = true;
                if prev.current >= STREAK_COMEBACK_MIN {
                    comeback = true;
                }
            }
            1
        }
    };
    // Earn a freeze when crossing a multiple of the earning span, never on a break.
    if !broke && current % STREAK_DAYS_PER_FREEZE == 0 {
        freezes = freezes.saturating_add(1);
    }
    freezes = freezes.min(STREAK_FREEZE_CAP);
    Transition {
        state: StreakState {
            current,
            longest: prev.longest.max(current),
            freezes,
            last_study_day: Some(today),
            comeback_armed: comeback,
        },
        froze_today: froze,
        broke_today: broke,
    }
}

/// The state after a day that is not a study day: the state itself for every outcome except a live
/// streak with two real misses, which is zeroed and arms the comeback, never touching the last
/// study day, the freezes or the longest run. One miss stays rescuable by a freeze bought today.
#[must_use]
pub fn decay_on_lapse(
    prev: &StreakState,
    today: StudyDay,
    skips: &BTreeSet<StudyDay>,
) -> Transition {
    let held = Transition {
        state: *prev,
        froze_today: false,
        broke_today: false,
    };
    let Some(last) = prev.last_study_day else {
        return held;
    };
    if classify_gap(prev, today, skips) != GapOutcome::Break || prev.current == 0 {
        return held;
    }
    if real_misses(last, today, skips) < 2 {
        return held;
    }
    Transition {
        state: StreakState {
            current: 0,
            comeback_armed: prev.comeback_armed || prev.current >= STREAK_COMEBACK_MIN,
            ..*prev
        },
        froze_today: false,
        broke_today: true,
    }
}

/// Whether the comeback shows: armed, and only on a live streak.
#[must_use]
pub fn comeback_view(state: &StreakState) -> bool {
    state.comeback_armed && state.current > 0
}
