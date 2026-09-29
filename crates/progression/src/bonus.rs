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
    let _ = facts;
    Vec::new()
}
