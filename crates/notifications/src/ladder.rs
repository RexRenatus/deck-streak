//! The celebration ladder (SPEC-084 R1 to R5, R7, R12; ADR-041): the tier a celebration asks for,
//! the owner's weekly budget of loud tiers, the step down over budget with its rare floor, the
//! streak-break cap and the near-miss gate, each read from `notifications-policy.json`.

use std::time::Duration;

use deck_streak_kernel::{StudyDay, UtcMillis};

use crate::occasion::{StreakFacts, Tier};
use crate::policy::Policy;

/// The dice a T4 and a T5 open with (the predecessor's default dice).
pub const DICE_EMOJI: &str = "\u{1f3b0}";
/// The reaction a T1 sets on the owner's latest message.
pub const REACTION_EMOJI: &str = "\u{1f389}";
/// How long a T3 reveal holds its placeholder before the edit.
pub const REVEAL_PAUSE: Duration = Duration::from_millis(2_500);
/// The placeholder a T3 reveal opens with.
pub const REVEAL_PLACEHOLDER: &str = "\u{1f4e6} <b>opening\u{2026}</b>";

/// The tier a celebration of `event` asks for.
#[must_use]
pub fn requested_tier(_policy: &Policy, _event: &str, _rarity: Option<&str>) -> Tier {
    Tier::T0
}

/// The weekly budget of `intensity`: its T4 and T5 counts.
#[must_use]
pub fn weekly_budget(_policy: &Policy, _intensity: &str) -> (u32, u32) {
    (0, 0)
}

/// The floor of `rarity`.
#[must_use]
pub fn rare_floor(_policy: &Policy, _rarity: Option<&str>) -> Option<Tier> {
    None
}

/// Whether `event` keeps its tier over budget.
#[must_use]
pub fn budget_exempt(_policy: &Policy, _event: &str) -> bool {
    false
}

/// `requested` after the weekly budget.
#[must_use]
pub fn apply_budget(
    _policy: &Policy,
    _requested: Tier,
    _used: (u32, u32),
    _budget: (u32, u32),
    _exempt: bool,
    _floor: Option<Tier>,
) -> Tier {
    Tier::T0
}

/// The highest tier the day allows.
#[must_use]
pub fn outcome_cap(_policy: &Policy, _broke: bool) -> Tier {
    Tier::T0
}

/// Whether the streak broke on `day`.
#[must_use]
pub fn streak_broke_on(_facts: Option<&StreakFacts>, _day: StudyDay) -> bool {
    false
}

/// Whether a gap of `remaining` toward `target` may be quoted.
#[must_use]
pub fn near_miss_ok(_policy: &Policy, _remaining: f64, _target: f64) -> bool {
    false
}

/// The Monday that starts the week of `day`.
#[must_use]
pub const fn week_start(day: StudyDay) -> StudyDay {
    day
}

/// Whether a message that arrived at `arrived_at` may still be reacted to at `now`.
#[must_use]
pub const fn reaction_fresh(_policy: &Policy, _arrived_at: UtcMillis, _now: UtcMillis) -> bool {
    false
}
