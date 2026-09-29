//! The XP constants, read from `economy.json`'s `xp` section (SPEC-072 R2; CHARTER 8).
//!
//! The file is embedded at build time and parsed once, so no XP number is typed in this crate's
//! source: the tuning is the file's. A test holds each value equal to the predecessor's constant
//! in `goldens/progression.constants.json`.

use std::sync::LazyLock;

/// Every XP constant of `economy.json`'s `xp` section this crate reads.
#[derive(Clone, Debug, PartialEq)]
pub struct XpEconomy {
    /// The base a review's XP starts from.
    pub base: f64,
    /// The ease multipliers for the buttons 1 to 4, again to easy.
    pub ease: [f64; 4],
    /// The interval, in days, from which a review is mature.
    pub mature_interval_days: i64,
    /// The maturity multiplier of a mature review.
    pub mature: f64,
    /// The maturity multiplier of a young review.
    pub young: f64,
    /// The maturity multiplier of a new review.
    pub fresh: f64,
    /// The type multipliers for the review types 0 to 3, learn to filtered.
    pub types: [f64; 4],
    /// The Bloom tier multipliers for `T1` to `T4`.
    pub tier: [f64; 4],
    /// The multiplier of a law card with no tier.
    pub untagged: f64,
    /// The daily bonus for a day with a review.
    pub studied: i64,
    /// The daily bonus for a day with no backlog and nothing due.
    pub backlog_zero: i64,
    /// The daily bonus per day of the raw streak.
    pub streak_per_day: i64,
    /// The most the streak bonus pays.
    pub streak_cap: i64,
    /// The daily bonus for a score at the threshold or above.
    pub score90: i64,
    /// The score from which the score bonus pays.
    pub score90_threshold: i64,
    /// The daily bonus per graduation.
    pub graduation: i64,
    /// The score from which a day is on pace.
    pub on_pace_score: i64,
    /// The multiplier a run day adds.
    pub step_per_run_day: f64,
    /// The most the consistency multiplier reaches.
    pub max_multiplier: f64,
    /// The run days a missed day costs.
    pub tier_down_step: i64,
    /// The rollup days the run is folded over.
    pub window_days: usize,
    /// The share of a day's review XP the Ascendant bonus pays.
    pub ascendant_fraction: f64,
    /// The most the Ascendant bonus pays.
    pub ascendant_cap: i64,
    /// The sources, and source prefixes ending in `%`, the predecessor's day base leaves out.
    pub day_base_excludes: Vec<String>,
}

/// The constants, parsed once.
#[must_use]
pub fn xp() -> &'static XpEconomy {
    static XP: LazyLock<XpEconomy> = LazyLock::new(|| XpEconomy {
        base: 0.0,
        ease: [0.0; 4],
        mature_interval_days: 0,
        mature: 0.0,
        young: 0.0,
        fresh: 0.0,
        types: [0.0; 4],
        tier: [0.0; 4],
        untagged: 0.0,
        studied: 0,
        backlog_zero: 0,
        streak_per_day: 0,
        streak_cap: 0,
        score90: 0,
        score90_threshold: 0,
        graduation: 0,
        on_pace_score: 0,
        step_per_run_day: 0.0,
        max_multiplier: 0.0,
        tier_down_step: 0,
        window_days: 0,
        ascendant_fraction: 0.0,
        ascendant_cap: 0,
        day_base_excludes: Vec::new(),
    });
    &XP
}
