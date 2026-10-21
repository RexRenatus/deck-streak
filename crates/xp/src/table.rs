//! The per-review XP constants, read from `economy.json`'s `xp` section (SPEC-360 R4; SPEC-072
//! R2; ADR-371 D4).
//!
//! The file is embedded when the crate is built and parsed once, so no XP number is typed in this
//! crate's source: the tuning is the file's. Progression's constants test holds eight of the nine
//! equal to the predecessor's constants in `goldens/progression.constants.json`; the predecessor
//! names no constant for the ninth, `untagged`.

use std::sync::LazyLock;

use serde_json::Value;

/// `economy.json`, embedded when the crate is built.
const ECONOMY_FILE: &str = include_str!("../../../economy.json");

/// The nine constants of `economy.json`'s `xp` section the per-review rule reads.
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewXpTable {
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
}

/// The table, parsed once.
///
/// # Panics
///
/// When the embedded file lacks a constant: a build-time fact, held by the constants test.
#[must_use]
pub fn table() -> &'static ReviewXpTable {
    static TABLE: LazyLock<ReviewXpTable> = LazyLock::new(parse);
    &TABLE
}

#[allow(
    clippy::expect_used,
    reason = "the embedded file is the build's own, and the constants test reads every field"
)]
fn parse() -> ReviewXpTable {
    let file: Value = serde_json::from_str(ECONOMY_FILE).expect("economy.json is JSON");
    let xp = &file["xp"];
    let number = |value: &Value| value.as_f64().expect("a number in economy.json");
    let integer = |value: &Value| value.as_i64().expect("an integer in economy.json");
    let four = |section: &Value, names: [&str; 4]| names.map(|name| number(&section[name]));
    ReviewXpTable {
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
    }
}
