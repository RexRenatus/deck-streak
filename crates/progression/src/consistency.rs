//! The consistency run and the day's bonuses over its base (SPEC-072 R15 to R18): the
//! predecessor's `tier_down_run`, `consistency_multiplier`, `projected_multiplier_drop`,
//! `on_pace_run`, `day_base_xp` and `apply_day_bonuses` at `27ee2bc`, as pure functions over what
//! the fold hands in (this crate cannot read the rollups; docs/CONTEXT-MAP.md).

use std::collections::BTreeSet;

use crate::economy_config::xp;

/// The source prefixes the day base leaves out beside `economy.json`'s `day_base_excludes`: the
/// predecessor's `NOT LIKE` list and the surprise bonus, plus the readings' own grants (R17).
const DAY_BASE_LEFT_OUT: [&str; 7] = [
    "surprise",
    "readgoal:",
    "leech:",
    "focus",
    "chest",
    "2x:",
    "reading:",
];

/// The XP a day's consistency and Ascendant bonuses multiply from: the sum of `rows`, each a
/// source and an amount, without the sources the base leaves out.
#[must_use]
pub fn day_base_xp<'a>(rows: impl IntoIterator<Item = (&'a str, u32)>) -> i64 {
    let _ = rows;
    0
}

/// The run after folding `days`, oldest first, each a score and whether it was a skip day: a skip
/// leaves the run alone, an on-pace day adds one, any other day takes two off, to no less than 0.
#[must_use]
pub fn tier_down_run(days: impl IntoIterator<Item = (i64, bool)>) -> i64 {
    let _ = days;
    0
}

/// The multiplier a run earns: 1 with no run, else one plus the step per run day, to the most.
#[must_use]
pub fn consistency_multiplier(run: i64) -> f64 {
    let _ = run;
    1.0
}

/// The multiplier now and the one a missed day would leave.
#[must_use]
pub fn projected_multiplier_drop(run: i64) -> (f64, f64) {
    let _ = run;
    (1.0, 1.0)
}

/// The run as of the day before `exclude_day`: the newest `window_days` of `rollups` (a day and its
/// score, in any order) walked from the oldest to the newest day, a day with no rollup scoring 0,
/// a day in `skips` a skip. 0 with no rollups.
#[must_use]
pub fn on_pace_run(rollups: &[(i64, i64)], skips: &BTreeSet<i64>, exclude_day: i64) -> i64 {
    let _ = (rollups, skips, exclude_day);
    0
}

/// The bonuses a day earns over its base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayBonuses {
    /// The consistency bonus: the base times the multiplier's excess, truncated.
    pub consistency: u32,
    /// The Ascendant bonus, when the day holds the buff.
    pub ascendant: Option<u32>,
}

/// The day's bonuses from its `base`, the `run`, whether it holds the Ascendant buff, and the
/// reviews of both tracks.
#[must_use]
pub fn day_bonuses(base: i64, run: i64, buff: bool, reviews: i64, reviews_law: i64) -> DayBonuses {
    let _ = (base, run, buff, reviews, reviews_law);
    DayBonuses {
        consistency: 0,
        ascendant: None,
    }
}

/// Whether a day arms the Ascendant buff for its successor: it holds none, is no skip day, and the
/// day before had reviews and a backlog-zero bonus.
#[must_use]
pub fn ascendant_arms(
    buff: bool,
    skip: bool,
    rollup_reviews: Option<i64>,
    backlog_zero: i64,
) -> bool {
    let _ = (buff, skip, rollup_reviews, backlog_zero);
    false
}
