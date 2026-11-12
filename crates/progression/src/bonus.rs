//! The daily bonuses (SPEC-072 R11): the predecessor's `gamification/xp.py:daily_bonus_grants` at
//! `27ee2bc`, as (source, amount) pairs in the predecessor's order.

use crate::economy_config::xp;

/// What a day earned, for the bonuses to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayFacts {
    /// Whether the day has a review.
    pub studied: bool,
    /// Whether the day's card snapshot has no backlog and nothing due.
    pub backlog_zero: bool,
    /// The raw streak's days at the day.
    pub streak_days: i64,
    /// The day's score total.
    pub score_total: i64,
    /// The graduations the day had.
    pub graduations: i64,
}

/// A bonus: the derived source that pays it and its amount.
pub type Bonus = (&'static str, u32);

/// The once-a-day bonuses `facts` earns, in the order they are settled.
#[must_use]
pub fn daily_bonuses(facts: &DayFacts) -> Vec<Bonus> {
    let economy = xp();
    let mut bonuses = Vec::new();
    let amount = |value: i64| u32::try_from(value.max(0)).unwrap_or(u32::MAX);
    if facts.studied {
        bonuses.push(("studied", amount(economy.studied)));
    }
    if facts.backlog_zero {
        bonuses.push(("backlog_zero", amount(economy.backlog_zero)));
    }
    if facts.streak_days > 0 {
        let streak = facts
            .streak_days
            .saturating_mul(economy.streak_per_day)
            .min(economy.streak_cap);
        bonuses.push(("streak", amount(streak)));
    }
    if facts.score_total >= economy.score90_threshold {
        bonuses.push(("score90", amount(economy.score90)));
    }
    if facts.graduations > 0 {
        bonuses.push((
            "graduations",
            amount(facts.graduations.saturating_mul(economy.graduation)),
        ));
    }
    bonuses
}
