//! Every XP constant is the predecessor's and is read from `economy.json` (SPEC-072 A2; R2).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_progression::economy_config::xp;
use deck_streak_progression::level::LEVEL_TITLES;
use deck_streak_progression::xp::{LEVEL_CURVE_LINEAR, LEVEL_CURVE_QUADRATIC};
use deck_streak_xp::table::table;
use serde_json::{Value, json};

/// Whether two values are the same constant: numbers by value, whichever way JSON writes them.
fn same(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| same(a, b)))
        }
        _ => left == right,
    }
}

#[test]
fn the_progression_constants_equal_the_predecessors_and_economy_json() {
    let economy = xp();
    // The eight per-review constants are the XP crate's own (SPEC-360 R4).
    let per_review = table();
    let keyed = |values: &[f64], keys: &[&str]| -> Value {
        keys.iter()
            .zip(values)
            .map(|(key, value)| ((*key).to_owned(), json!(value)))
            .collect::<serde_json::Map<_, _>>()
            .into()
    };
    golden::each_case("progression.constants", |case| {
        let name = case.input["name"].as_str().expect("a constant's name");
        let ours: Value = match name {
            "constants.XP_BASE" => json!(per_review.base),
            "constants.EASE_XP_MULT" => keyed(&per_review.ease, &["1", "2", "3", "4"]),
            "constants.TYPE_XP_MULT" => keyed(&per_review.types, &["0", "1", "2", "3"]),
            "constants.XP_MATURE_MULT" => json!(per_review.mature),
            "constants.XP_YOUNG_MULT" => json!(per_review.young),
            "constants.XP_NEUTRAL_MULT" => json!(per_review.fresh),
            "constants.MATURE_IVL_DAYS" => json!(per_review.mature_interval_days),
            "constants.TIER_XP_MULT" => keyed(&per_review.tier, &["T1", "T2", "T3", "T4"]),
            "constants.XP_BONUS_STUDIED" => json!(economy.studied),
            "constants.XP_BONUS_BACKLOG_ZERO" => json!(economy.backlog_zero),
            "constants.XP_BONUS_STREAK_PER_DAY" => json!(economy.streak_per_day),
            "constants.XP_BONUS_STREAK_CAP" => json!(economy.streak_cap),
            "constants.XP_BONUS_SCORE_90" => json!(economy.score90),
            "constants.XP_BONUS_GRADUATION_EACH" => json!(economy.graduation),
            "constants.SCORE_90_THRESHOLD" => json!(economy.score90_threshold),
            "constants.LEVEL_A" => json!(LEVEL_CURVE_QUADRATIC),
            "constants.LEVEL_B" => json!(LEVEL_CURVE_LINEAR),
            "constants.LEVEL_TITLES" => LEVEL_TITLES
                .iter()
                .map(|(level, title, emoji)| json!([level, title, emoji]))
                .collect(),
            "constants.ON_PACE_SCORE" => json!(economy.on_pace_score),
            "constants.TIER_DOWN_STEP" => json!(economy.tier_down_step),
            "constants.ASCENDANT_XP_FRAC" => json!(economy.ascendant_fraction),
            "constants.ASCENDANT_XP_CAP" => json!(economy.ascendant_cap),
            "gamification.adaptive.CONSISTENCY_STEP" => json!(economy.step_per_run_day),
            "gamification.adaptive.CONSISTENCY_CAP" => json!(economy.max_multiplier),
            other => panic!("the golden names a constant this test does not map: {other}"),
        };
        assert!(
            same(&ours, &case.output),
            "the constant {name}: {ours} against {}",
            case.output
        );
    });
    // The on-pace window is the file's own and the predecessor's is a literal in a function, which
    // the consistency golden proves; here it is only held to the file's value.
    assert_eq!(economy.window_days, 90, "the on-pace window");
}
