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

/// The milliseconds of one hour, the unit a reaction's age is bounded in.
const HOUR_MS: i64 = 3_600_000;

/// The tier a celebration of `event` asks for: its rarity's tier when it has a rarity the ladder
/// knows, else its event's, else the tier of an unknown event (R1).
#[must_use]
pub fn requested_tier(policy: &Policy, event: &str, rarity: Option<&str>) -> Tier {
    let ladder = &policy.ladder;
    rarity
        .and_then(|rarity| ladder.rarity.get(rarity))
        .or_else(|| ladder.events.get(event))
        .copied()
        .unwrap_or(ladder.unknown_event)
}

/// The weekly budget of `intensity`, an unknown one read as the default: its T4 and T5 counts
/// (R2).
#[must_use]
pub fn weekly_budget(policy: &Policy, intensity: &str) -> (u32, u32) {
    let budgets = &policy.celebration_budgets;
    let counts = budgets
        .intensities
        .get(intensity)
        .or_else(|| budgets.intensities.get(&budgets.default_intensity));
    let count = |tier| {
        counts
            .and_then(|counts| counts.get(&tier))
            .copied()
            .unwrap_or(0)
    };
    (count(Tier::T4), count(Tier::T5))
}

/// The floor an Epic or a Legendary keeps through the budget, or none for any other rarity (R3).
#[must_use]
pub fn rare_floor(policy: &Policy, rarity: Option<&str>) -> Option<Tier> {
    rarity.and_then(|rarity| policy.ladder.rarity_floor.get(rarity).copied())
}

/// Whether `event` keeps its tier over budget, as a band-up does (R3).
#[must_use]
pub fn budget_exempt(policy: &Policy, event: &str) -> bool {
    policy
        .celebration_budgets
        .exempt_events
        .iter()
        .any(|exempt| exempt == event)
}

/// `requested` after the weekly budget: with `used` of the week's (T4, T5) `budget` spent, a T5
/// over its budget steps down to T4, and a T4 over its budget to T3, never below `floor`; an
/// exempt event keeps its tier (R3).
#[must_use]
pub fn apply_budget(
    policy: &Policy,
    requested: Tier,
    used: (u32, u32),
    budget: (u32, u32),
    exempt: bool,
    floor: Option<Tier>,
) -> Tier {
    if exempt {
        return requested;
    }
    let downgrade = &policy.celebration_budgets.downgrade;
    let step = |tier: Tier| downgrade.get(&tier).copied().unwrap_or(tier);
    let mut tier = requested;
    if tier >= Tier::T5 && used.1 >= budget.1 {
        tier = step(tier);
    }
    if tier == Tier::T4 && used.0 >= budget.0 {
        tier = step(tier);
    }
    floor.map_or(tier, |floor| tier.max(floor))
}

/// The highest tier the day allows: the streak-break cap on a day the streak broke, else T5 (R5).
#[must_use]
pub fn outcome_cap(policy: &Policy, broke: bool) -> Tier {
    if broke {
        policy.streak_break.cap
    } else {
        Tier::T5
    }
}

/// Whether the streak broke on `day`: its last study day is `day`, its length is back to 1 and it
/// was once longer. No facts read as no break (R5).
#[must_use]
pub fn streak_broke_on(facts: Option<&StreakFacts>, day: StudyDay) -> bool {
    facts.is_some_and(|facts| {
        facts.last_study_day == Some(day) && facts.current == 1 && facts.longest > 1
    })
}

/// Whether a gap of `remaining` toward `target` may be quoted: it is positive, and at most the
/// policy's units or at most its fraction of the target (R12).
#[must_use]
pub fn near_miss_ok(policy: &Policy, remaining: f64, target: f64) -> bool {
    if remaining <= 0.0 || target <= 0.0 {
        return false;
    }
    let near_miss = &policy.near_miss;
    remaining <= f64::from(near_miss.max_units) || remaining / target <= near_miss.max_fraction
}

/// The Monday that starts the week of `day` (the epoch's first day was a Thursday) (R4).
#[must_use]
pub const fn week_start(day: StudyDay) -> StudyDay {
    let epoch_day = day.epoch_day();
    StudyDay::from_epoch_day(epoch_day - (epoch_day + 3).rem_euclid(7))
}

/// Whether a message that arrived at `arrived_at` may still be reacted to at `now`: it is at most
/// `ladder.reaction_max_age_hours` old (R9).
#[must_use]
pub fn reaction_fresh(policy: &Policy, arrived_at: UtcMillis, now: UtcMillis) -> bool {
    now.epoch_millis() - arrived_at.epoch_millis()
        <= i64::from(policy.ladder.reaction_max_age_hours) * HOUR_MS
}
