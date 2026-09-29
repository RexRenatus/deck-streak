//! The five-pillar score, each pillar, the grade band, the raw streak and the volume baseline equal
//! the predecessor's, and so does every analytics constant (SPEC-071 A7 to A10; R6, R12, R13):
//! every case of their goldens, floats compared bit for bit and the total rounded half to even.

// An integration test is test code: its helpers panic on a malformed golden, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_analytics::constants::{
    ANSWER_TIME_CAP_SECONDS, GRADE_BANDS, MASTERY_GRADUATION_TARGET, MASTERY_LEECH_PENALTY,
    MASTERY_LEECH_PENALTY_CAP, MATURE_IVL_DAYS, RETENTION_BLEND_TARGET, RETENTION_CEIL_PCT,
    RETENTION_FLOOR_PCT, RETENTION_MIN_SAMPLE, VOLUME_CAP_RATIO, VOLUME_REVIEW_WEIGHT,
    VOLUME_TIME_WEIGHT, WEIGHT_CONSISTENCY, WEIGHT_MASTERY, WEIGHT_RETENTION, WEIGHT_VOLUME,
    WEIGHT_WORKLOAD, WORKLOAD_BACKLOG_DIVISOR, WORKLOAD_BACKLOG_PENALTY_CAP,
};
use deck_streak_analytics::metrics::DailyMetrics;
use deck_streak_analytics::score::{
    Baseline, DayVolume, ScoreState, baseline_window, compute_score, consistency, grade_band,
    mastery, raw_streak, retention, volume, volume_baseline, workload,
};
use deck_streak_analytics::settings::DEFAULT_LEECH_THRESHOLD;
use deck_streak_kernel::StudyDay;
use serde_json::{Value, json};

fn integer(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

fn number(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("a number: {value}"))
}

/// `port` is the predecessor's `golden` to the bit.
fn same(port: f64, golden: &Value, what: &str) {
    assert_eq!(
        port.to_bits(),
        number(golden).to_bits(),
        "{what}: the port's {port} against the predecessor's {golden}"
    );
}

#[test]
fn the_analytics_constants_equal_the_predecessors() {
    let bands: Vec<Value> = GRADE_BANDS
        .iter()
        .map(|band| json!([band.threshold, band.label, band.emoji]))
        .collect();
    let port: BTreeMap<&str, Value> = BTreeMap::from([
        (
            "constants.ANSWER_TIME_CAP_SECONDS",
            json!(ANSWER_TIME_CAP_SECONDS),
        ),
        ("constants.MATURE_IVL_DAYS", json!(MATURE_IVL_DAYS)),
        (
            "constants.DEFAULT_LEECH_THRESHOLD",
            json!(DEFAULT_LEECH_THRESHOLD),
        ),
        ("constants.WEIGHT_CONSISTENCY", json!(WEIGHT_CONSISTENCY)),
        ("constants.WEIGHT_RETENTION", json!(WEIGHT_RETENTION)),
        ("constants.WEIGHT_WORKLOAD", json!(WEIGHT_WORKLOAD)),
        ("constants.WEIGHT_VOLUME", json!(WEIGHT_VOLUME)),
        ("constants.WEIGHT_MASTERY", json!(WEIGHT_MASTERY)),
        ("constants.RETENTION_FLOOR_PCT", json!(RETENTION_FLOOR_PCT)),
        ("constants.RETENTION_CEIL_PCT", json!(RETENTION_CEIL_PCT)),
        (
            "constants.RETENTION_MIN_SAMPLE",
            json!(RETENTION_MIN_SAMPLE),
        ),
        (
            "constants.RETENTION_BLEND_TARGET",
            json!(RETENTION_BLEND_TARGET),
        ),
        (
            "constants.VOLUME_REVIEW_WEIGHT",
            json!(VOLUME_REVIEW_WEIGHT),
        ),
        ("constants.VOLUME_TIME_WEIGHT", json!(VOLUME_TIME_WEIGHT)),
        ("constants.VOLUME_CAP_RATIO", json!(VOLUME_CAP_RATIO)),
        (
            "constants.MASTERY_GRADUATION_TARGET",
            json!(MASTERY_GRADUATION_TARGET),
        ),
        (
            "constants.MASTERY_LEECH_PENALTY",
            json!(MASTERY_LEECH_PENALTY),
        ),
        (
            "constants.MASTERY_LEECH_PENALTY_CAP",
            json!(MASTERY_LEECH_PENALTY_CAP),
        ),
        (
            "constants.WORKLOAD_BACKLOG_DIVISOR",
            json!(WORKLOAD_BACKLOG_DIVISOR),
        ),
        (
            "constants.WORKLOAD_BACKLOG_PENALTY_CAP",
            json!(WORKLOAD_BACKLOG_PENALTY_CAP),
        ),
        ("constants.GRADE_BANDS", Value::Array(bands)),
    ]);
    let mut proved = BTreeSet::new();
    golden::each_case("analytics.constants", |case| {
        let name = case.input["name"].as_str().expect("a constant's name");
        let value = port
            .get(name)
            .unwrap_or_else(|| panic!("the golden names {name}, which analytics does not hold"));
        assert_eq!(value, &case.output, "{name}");
        proved.insert(name.to_owned());
    });
    assert_eq!(
        proved,
        port.keys().map(|&name| name.to_owned()).collect(),
        "every constant analytics ports was proved"
    );
}

/// The metrics a score case names, zero elsewhere, as the golden's adapter built them.
fn metrics(input: &Value) -> DailyMetrics {
    DailyMetrics {
        reviews: integer(&input["reviews"]),
        review_count: integer(&input["review_count"]),
        seconds: number(&input["seconds"]),
        answered: integer(&input["answered"]),
        true_retention: number(&input["true_retention"]),
        graduations: integer(&input["graduations"]),
        ..DailyMetrics::empty(StudyDay::from_epoch_day(0))
    }
}

#[test]
fn the_score_matches_the_predecessors_golden() {
    let mut classes = BTreeSet::new();
    golden::each_case("compute_score", |case| {
        let input = &case.input;
        let state = (!input["snapshot"].is_null()).then(|| ScoreState {
            due_today: integer(&input["snapshot"]["due_today"]),
            backlog: integer(&input["snapshot"]["backlog"]),
            leech_active: integer(&input["snapshot"]["leech_active"]),
        });
        let score = compute_score(
            &metrics(&input["metrics"]),
            state,
            integer(&input["streak_days"]),
            Baseline {
                reviews: number(&input["base_reviews"]),
                minutes: number(&input["base_minutes"]),
            },
        );
        let output = &case.output;
        same(score.consistency, &output["consistency"], "consistency");
        same(score.retention, &output["retention"], "retention");
        same(score.workload, &output["workload"], "workload");
        same(score.volume, &output["volume"], "volume");
        same(score.mastery, &output["mastery"], "mastery");
        assert_eq!(json!(score.total), output["total"], "the total of {input}");
        assert_eq!(json!(score.grade_label), output["grade_label"], "{input}");
        assert_eq!(json!(score.grade_emoji), output["grade_emoji"], "{input}");
        if let Some(class) = &case.class {
            classes.insert(class.clone());
        }
    });
    assert!(classes.contains("tie"), "the golden carries a tie");
    assert!(
        classes.contains("historical"),
        "the golden carries the historical form"
    );
}

#[test]
fn the_five_pillars_match_their_goldens() {
    golden::each_case("score_consistency", |case| {
        let input = &case.input;
        let port = consistency(integer(&input["reviews"]), integer(&input["streak_days"]));
        same(port, &case.output, &format!("consistency of {input}"));
    });
    let mut small = 0;
    golden::each_case("score_retention", |case| {
        let input = &case.input;
        let port = retention(
            number(&input["true_retention"]),
            integer(&input["answered"]),
        );
        same(port, &case.output, &format!("retention of {input}"));
        small += usize::from(case.class.as_deref() == Some("small-sample"));
    });
    assert!(small > 0, "the golden carries small samples");
    golden::each_case("score_workload", |case| {
        let input = &case.input;
        let port = workload(
            integer(&input["review_count"]),
            integer(&input["due_today"]),
            integer(&input["backlog"]),
            input["studied"].as_bool().expect("a flag"),
        );
        same(port, &case.output, &format!("workload of {input}"));
    });
    golden::each_case("score_volume", |case| {
        let input = &case.input;
        let port = volume(
            integer(&input["reviews"]),
            number(&input["minutes"]),
            number(&input["base_reviews"]),
            number(&input["base_minutes"]),
        );
        same(port, &case.output, &format!("volume of {input}"));
    });
    golden::each_case("score_mastery", |case| {
        let input = &case.input;
        let port = mastery(
            integer(&input["graduations"]),
            integer(&input["leech_active"]),
        );
        same(port, &case.output, &format!("mastery of {input}"));
    });
}

fn volumes(rows: &Value) -> Vec<DayVolume> {
    rows.as_array()
        .expect("rollup rows")
        .iter()
        .map(|row| DayVolume {
            day: StudyDay::from_epoch_day(row.get("day").map_or(0, integer)),
            reviews: integer(&row["reviews"]),
            seconds: number(&row["seconds"]),
        })
        .collect()
}

fn pair(baseline: Baseline, golden: &Value, what: &str) {
    same(
        baseline.reviews,
        &golden[0],
        &format!("the base reviews of {what}"),
    );
    same(
        baseline.minutes,
        &golden[1],
        &format!("the base minutes of {what}"),
    );
}

#[test]
fn the_grade_raw_streak_and_baseline_match_their_goldens() {
    golden::each_case("grade_band", |case| {
        let (label, emoji) = grade_band(integer(&case.input["score"]));
        assert_eq!(
            json!([label, emoji]),
            case.output,
            "the band of {}",
            case.input
        );
    });
    golden::each_case("raw_streak", |case| {
        let days: BTreeSet<StudyDay> = case.input["days"]
            .as_array()
            .expect("days")
            .iter()
            .map(|day| StudyDay::from_epoch_day(integer(day)))
            .collect();
        let today = StudyDay::from_epoch_day(integer(&case.input["today"]));
        assert_eq!(
            json!(raw_streak(&days, today)),
            case.output,
            "{}",
            case.input
        );
    });
    golden::each_case("volume_baseline", |case| {
        let port = volume_baseline(&volumes(&case.input["rollups"]));
        pair(port, &case.output, &case.input.to_string());
    });
    golden::each_case("baseline_window", |case| {
        let today = StudyDay::from_epoch_day(integer(&case.input["today"]));
        let port = baseline_window(&volumes(&case.input["rollups"]), today);
        pair(port, &case.output, &case.input.to_string());
    });
}
