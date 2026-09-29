//! The XP constants, read from `economy.json`'s `xp` section (SPEC-072 R2; CHARTER 8).
//!
//! The file is embedded at build time and parsed once, so no XP number is typed in this crate's
//! source: the tuning is the file's. A test holds each value equal to the predecessor's constant
//! in `goldens/progression.constants.json`.

use std::sync::LazyLock;

use serde_json::Value;

/// `economy.json`, embedded at build time.
const ECONOMY_FILE: &str = include_str!("../../../economy.json");

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
///
/// # Panics
///
/// When the embedded file lacks a constant: a build-time fact, held by the constants test.
#[must_use]
pub fn xp() -> &'static XpEconomy {
    static XP: LazyLock<XpEconomy> = LazyLock::new(parse);
    &XP
}

#[allow(
    clippy::expect_used,
    reason = "the embedded file is the build's own, and the constants test reads every field"
)]
fn parse() -> XpEconomy {
    let file: Value = serde_json::from_str(ECONOMY_FILE).expect("economy.json is JSON");
    let xp = &file["xp"];
    let number = |value: &Value| value.as_f64().expect("a number in economy.json");
    let integer = |value: &Value| value.as_i64().expect("an integer in economy.json");
    let four = |section: &Value, names: [&str; 4]| names.map(|name| number(&section[name]));
    let consistency = &xp["bonuses"]["consistency"];
    let ascendant = &xp["bonuses"]["ascendant"];
    let daily = &xp["daily_bonuses"];
    XpEconomy {
        base: number(&xp["base"]),
        ease: four(&xp["ease_multipliers"], ["again", "hard", "good", "easy"]),
        mature_interval_days: integer(&xp["maturity"]["mature_interval_days"]),
        mature: number(&xp["maturity"]["mature"]),
        young: number(&xp["maturity"]["young"]),
        fresh: number(&xp["maturity"]["new"]),
        types: four(
            &xp["type_multipliers"],
            ["learn", "review", "relearn", "filtered"],
        ),
        tier: four(&xp["tier"]["multipliers"], ["T1", "T2", "T3", "T4"]),
        untagged: number(&xp["tier"]["untagged"]),
        studied: integer(&daily["studied"]),
        backlog_zero: integer(&daily["backlog_zero"]),
        streak_per_day: integer(&daily["streak_per_day"]),
        streak_cap: integer(&daily["streak_cap"]),
        score90: integer(&daily["score90"]),
        score90_threshold: integer(&daily["score90_threshold"]),
        graduation: integer(&daily["graduation"]),
        on_pace_score: integer(&consistency["on_pace_score"]),
        step_per_run_day: number(&consistency["step_per_run_day"]),
        max_multiplier: number(&consistency["max_multiplier"]),
        tier_down_step: integer(&consistency["tier_down_step"]),
        window_days: usize::try_from(integer(&consistency["window_days"]))
            .expect("a window of days"),
        ascendant_fraction: number(&ascendant["fraction"]),
        ascendant_cap: integer(&ascendant["cap"]),
        day_base_excludes: xp["day_base_excludes"]
            .as_array()
            .expect("the day base's exclusions")
            .iter()
            .map(|name| name.as_str().expect("a source name").to_owned())
            .collect(),
    }
}
