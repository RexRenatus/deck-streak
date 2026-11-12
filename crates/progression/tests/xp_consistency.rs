//! The consistency run, its multiplier and the day base are the predecessor's (SPEC-072 A19 to
//! A22; R15 to R17).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;

use deck_streak_progression::consistency::{
    consistency_multiplier, day_base_xp, on_pace_run, projected_multiplier_drop, tier_down_run,
};

#[test]
fn the_consistency_run_and_multiplier_match_the_parity_goldens() {
    golden::each_case("tier_down_run", |case| {
        let days = case.input["day_results"].as_array().expect("day results");
        let run = tier_down_run(days.iter().map(|day| {
            (
                day[0].as_i64().expect("a score"),
                day[1].as_bool().expect("a skip flag"),
            )
        }));
        assert_eq!(
            run,
            case.output.as_i64().expect("a run"),
            "the run of {}",
            case.input
        );
    });
    golden::each_case("consistency_multiplier", |case| {
        let run = case.input["consecutive_on_pace_days"]
            .as_i64()
            .expect("a run");
        assert_eq!(
            consistency_multiplier(run).to_bits(),
            case.output.as_f64().expect("a multiplier").to_bits(),
            "the multiplier of a run of {run}"
        );
    });
    golden::each_case("projected_multiplier_drop", |case| {
        let run = case.input["run"].as_i64().expect("a run");
        let (now, after) = projected_multiplier_drop(run);
        let expected = case.output.as_array().expect("a pair");
        assert_eq!(now.to_bits(), expected[0].as_f64().expect("now").to_bits());
        assert_eq!(
            after.to_bits(),
            expected[1].as_f64().expect("after").to_bits()
        );
    });
    golden::each_case("on_pace_run", |case| {
        let rollups: Vec<(i64, i64)> = case.input["rollups"]
            .as_array()
            .expect("rollups")
            .iter()
            .map(|rollup| {
                (
                    rollup["day"].as_i64().expect("a day"),
                    rollup["score"].as_i64().expect("a score"),
                )
            })
            .collect();
        let skips: BTreeSet<i64> = case.input["skips"]
            .as_array()
            .expect("skips")
            .iter()
            .map(|day| day.as_i64().expect("a day"))
            .collect();
        let exclude = case.input["exclude_day"].as_i64().expect("a day");
        assert_eq!(
            on_pace_run(&rollups, &skips, exclude),
            case.output.as_i64().expect("a run"),
            "the run of {}",
            case.input
        );
    });
}

/// The rows of a day-base golden case, as (source, amount), for the day the case names.
fn rows_of(case: &golden::Case) -> Vec<(String, u32)> {
    let day = case.input["day"].as_i64().expect("a day");
    case.input["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .filter(|row| row["day"].as_i64() == Some(day))
        .map(|row| {
            (
                row["source"].as_str().expect("a source").to_owned(),
                u32::try_from(row["amount"].as_u64().expect("an amount")).expect("small"),
            )
        })
        .collect()
}

#[test]
fn the_day_base_matches_the_parity_golden_over_both_tables() {
    golden::each_case("day_base_xp", |case| {
        let rows = rows_of(case);
        assert_eq!(
            day_base_xp(
                rows.iter()
                    .map(|(source, amount)| (source.as_str(), *amount))
            ),
            case.output.as_i64().expect("a base"),
            "the base of {}",
            case.input
        );
    });
}

#[test]
fn the_consistency_grant_is_unchanged_by_ascendant_token_and_chest_xp() {
    let base = [("reviews", 300), ("reviews_law", 200), ("studied", 50)];
    let with_extras = [
        ("reviews", 300),
        ("reviews_law", 200),
        ("studied", 50),
        ("ascendant", 150),
        ("consistency", 75),
        ("double_xp_token", 80),
        ("chest:common", 40),
        ("2x:token", 25),
    ];
    assert_eq!(day_base_xp(base), 550);
    assert_eq!(
        day_base_xp(with_extras),
        550,
        "the extras leave the base alone"
    );
}

#[test]
fn the_day_base_leaves_out_the_readings_grants() {
    let rows = [
        ("reviews", 300),
        ("reading:read:r1", 60),
        ("reading:studied:r1", 40),
    ];
    assert_eq!(day_base_xp(rows), 300, "a reading's grant is not review XP");
}
