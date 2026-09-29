//! Freezes: the events a transition writes, the drop gate and the port's decision (SPEC-076
//! R7 to R9), after the predecessor's `pipeline.py:_freeze_events_for` and
//! `pipeline_layers/loot.py:pick_epic_prize` at `27ee2bc`.

use deck_streak_kernel::StudyDay;

use crate::streak::{StreakState, Transition};

/// Why a freeze event was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreezeReason {
    /// A freeze covered a miss.
    Consumed,
    /// The run broke: a marker with no freeze.
    StreakBreak,
    /// A streak day earned a freeze.
    StreakEarn,
    /// An Epic chest paid a freeze.
    Chest,
    /// A weekly quest paid a freeze.
    WeeklyQuest,
    /// A season node paid a freeze.
    Season,
    /// The shop sold a freeze.
    Shop,
}

impl FreezeReason {
    /// The stored name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Consumed => "consumed",
            Self::StreakBreak => "streak_break",
            Self::StreakEarn => "streak_earn",
            Self::Chest => "chest",
            Self::WeeklyQuest => "weekly_quest",
            Self::Season => "season",
            Self::Shop => "shop",
        }
    }

    /// The reason for a stored name.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let _ = name;
        None
    }

    /// Whether the reason is a drop, counted against the monthly cap.
    #[must_use]
    pub const fn is_drop(self) -> bool {
        false
    }
}

/// One freeze event: a grant, a spend or a break marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FreezeEvent {
    /// The study day.
    pub day: StudyDay,
    /// Freezes gained (positive), spent (negative), or none (a marker).
    pub delta: i32,
    /// Why.
    pub reason: FreezeReason,
}

/// The event rows a transition produces.
#[must_use]
pub fn freeze_events_for(
    prev: &StreakState,
    next: &Transition,
    today: StudyDay,
) -> Vec<FreezeEvent> {
    let _ = (prev, next, today);
    vec![FreezeEvent {
        day: today,
        delta: 0,
        reason: FreezeReason::Shop,
    }]
}

/// The cap that stopped a freeze.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The hold cap.
    HoldCap,
    /// The monthly cap on drop-style freezes.
    MonthlyDropCap,
}

/// Drop-style freezes `events` hold in the calendar month of `day`.
#[must_use]
pub fn drops_in_month(events: &[FreezeEvent], day: StudyDay) -> u32 {
    let _ = (events, day);
    u32::MAX
}

/// Whether a freeze of `reason` may be granted on `day`.
///
/// # Errors
///
/// The [`Refusal`] naming the cap that stopped it.
pub fn admit(
    held: u32,
    reason: FreezeReason,
    day: StudyDay,
    events: &[FreezeEvent],
) -> Result<(), Refusal> {
    let _ = (held, reason, day, events);
    Ok(())
}

/// What an Epic chest's choice pays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EpicPrize {
    /// A freeze.
    Freeze,
    /// The Double-XP token.
    Token,
}

/// Settle an Epic chest's `choice` opened on `day`: over a cap the freeze falls back to a token.
#[must_use]
pub fn pick_epic_prize(
    choice: EpicPrize,
    held: u32,
    day: StudyDay,
    events: &[FreezeEvent],
) -> EpicPrize {
    let _ = (held, day, events);
    choice
}
