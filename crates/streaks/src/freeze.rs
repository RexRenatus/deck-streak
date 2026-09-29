//! Freezes: the events a transition writes, the drop gate and the port's decision (SPEC-076
//! R7 to R9), after the predecessor's `pipeline.py:_freeze_events_for` and
//! `pipeline_layers/loot.py:pick_epic_prize` at `27ee2bc`.

use deck_streak_kernel::StudyDay;

use crate::constants::{FREEZE_DROP_MONTHLY_CAP, STREAK_FREEZE_CAP};
use crate::streak::{StreakState, Transition};

/// The (year, month) of a study day, by the proleptic Gregorian calendar (Hinnant's civil-from-days).
fn civil_month(day: StudyDay) -> (i64, i64) {
    let z = day.epoch_day() + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month)
}

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
        Some(match name {
            "consumed" => Self::Consumed,
            "streak_break" => Self::StreakBreak,
            "streak_earn" => Self::StreakEarn,
            "chest" => Self::Chest,
            "weekly_quest" => Self::WeeklyQuest,
            "season" => Self::Season,
            "shop" => Self::Shop,
            _ => return None,
        })
    }

    /// Whether the reason is a drop, counted against the monthly cap.
    #[must_use]
    pub const fn is_drop(self) -> bool {
        matches!(self, Self::Chest | Self::WeeklyQuest | Self::Season)
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
    let mut out = Vec::new();
    let mut push = |delta: i32, reason: FreezeReason| {
        out.push(FreezeEvent {
            day: today,
            delta,
            reason,
        });
    };
    if next.froze_today {
        push(-1, FreezeReason::Consumed);
    }
    if next.broke_today {
        push(0, FreezeReason::StreakBreak);
    }
    let spent = i64::from(next.froze_today);
    let earned = i64::from(next.state.freezes) - i64::from(prev.freezes) + spent;
    if earned > 0 {
        push(
            i32::try_from(earned).unwrap_or(i32::MAX),
            FreezeReason::StreakEarn,
        );
    }
    out
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
    let month = civil_month(day);
    let count = events
        .iter()
        .filter(|e| e.delta > 0 && e.reason.is_drop() && civil_month(e.day) == month)
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
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
    if held >= STREAK_FREEZE_CAP {
        return Err(Refusal::HoldCap);
    }
    if reason.is_drop() && drops_in_month(events, day) >= FREEZE_DROP_MONTHLY_CAP {
        return Err(Refusal::MonthlyDropCap);
    }
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
    match choice {
        EpicPrize::Freeze if admit(held, FreezeReason::Chest, day, events).is_err() => {
            EpicPrize::Token
        }
        other => other,
    }
}
