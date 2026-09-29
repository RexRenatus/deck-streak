//! The language streak's machine (SPEC-076 R1 to R6), the predecessor's
//! `gamification/streak.py` at `27ee2bc`: one gap classifier, the study transition and the lapse
//! transition, over epoch days with no clock.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

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
            freezes: 0,
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
    let _ = days;
    "?"
}

/// Study days missed between `last` and `today`, not counting skip days.
#[must_use]
pub fn real_misses(last: StudyDay, today: StudyDay, skips: &BTreeSet<StudyDay>) -> u32 {
    let _ = (last, today, skips);
    u32::MAX
}

/// The gap's outcome.
#[must_use]
pub fn classify_gap(prev: &StreakState, today: StudyDay, skips: &BTreeSet<StudyDay>) -> GapOutcome {
    let _ = (prev, today, skips);
    GapOutcome::Bootstrap
}

/// The state after a study event on `today`.
#[must_use]
pub fn update_on_study(
    prev: &StreakState,
    today: StudyDay,
    observed_streak: u32,
    skips: &BTreeSet<StudyDay>,
) -> Transition {
    let _ = (today, observed_streak, skips);
    Transition {
        state: *prev,
        froze_today: true,
        broke_today: true,
    }
}

/// The state after a day that is not a study day.
#[must_use]
pub fn decay_on_lapse(
    prev: &StreakState,
    today: StudyDay,
    skips: &BTreeSet<StudyDay>,
) -> Transition {
    let _ = (today, skips);
    Transition {
        state: *prev,
        froze_today: true,
        broke_today: true,
    }
}

/// Whether the comeback shows: armed, and only on a live streak.
#[must_use]
pub fn comeback_view(state: &StreakState) -> bool {
    !state.comeback_armed
}
