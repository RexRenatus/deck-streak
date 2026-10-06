//! The XP constants, read from `economy.json`'s `xp` section (SPEC-072 R2; CHARTER 8).
//!
//! The file is embedded at build time and parsed once, so no XP number is typed in this crate's
//! source: the tuning is the file's. A test holds each value equal to the predecessor's constant
//! in `goldens/progression.constants.json`.
//!
//! The per-review table, the base and the ease, maturity, type and tier multipliers, lives in
//! `deck-streak-xp` beside its rule (SPEC-360 R4, ADR-371 D4); this file keeps the day-level
//! constants.

use std::sync::LazyLock;

use serde_json::Value;

/// `economy.json`, embedded at build time.
const ECONOMY_FILE: &str = include_str!("../../../economy.json");

/// Every XP constant of `economy.json`'s `xp` section this crate reads.
#[derive(Clone, Debug, PartialEq)]
pub struct XpEconomy {
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
    let consistency = &xp["bonuses"]["consistency"];
    let ascendant = &xp["bonuses"]["ascendant"];
    let daily = &xp["daily_bonuses"];
    XpEconomy {
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
