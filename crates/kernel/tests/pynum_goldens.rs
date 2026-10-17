//! The kernel's sum, median, mean and round equal `CPython`'s, and its nearest-rank percentile
//! equals the predecessor's, for every golden case (SPEC-302 A1, A2).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;

use deck_streak_kernel::pynum;
use serde_json::Value;

/// The number `value` holds, as a float: JSON writes a whole float and an integer alike.
fn float(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("{value} is a number"))
}

/// The floats of the JSON array `value`.
fn floats(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{value} is an array"))
        .iter()
        .map(float)
        .collect()
}

#[test]
fn the_numeric_basics_match_cpythons_golden() {
    let mut ops = BTreeSet::new();
    let mut classes = BTreeSet::new();
    golden::each_case("pynum_basics", |case| {
        let op = case.input["op"].as_str().expect("an op names the function");
        let want = float(&case.output);
        let got = match op {
            "sum" => pynum::sum(floats(&case.input["values"])),
            "median" => pynum::median(&floats(&case.input["values"]))
                .expect("a median of a list that holds a value"),
            "mean" => pynum::mean(&floats(&case.input["values"]))
                .expect("a mean of a list that holds a value"),
            "round" => pynum::round(
                float(&case.input["x"]),
                i32::try_from(case.input["ndigits"].as_i64().expect("an integer"))
                    .expect("digits within an i32"),
            ),
            other => panic!("{other} is not an op of the golden"),
        };
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "{op} of {}: got {got:e}, CPython {want:e}",
            case.input
        );
        ops.insert(op.to_owned());
        classes.extend(case.class.clone());
    });
    for op in ["sum", "median", "mean", "round"] {
        assert!(ops.contains(op), "no {op} case was examined");
    }
    for class in [
        "cancellation",
        "shortest_decimal_tie",
        "tie",
        "running_sum_differs",
    ] {
        assert!(classes.contains(class), "no {class} case was examined");
    }
}

#[test]
fn an_empty_median_and_an_empty_mean_are_none() {
    assert_eq!(
        pynum::median(&[]),
        None,
        "CPython raises on an empty median"
    );
    assert_eq!(pynum::mean(&[]), None, "CPython raises on an empty mean");
    assert_eq!(
        pynum::sum(Vec::new()).to_bits(),
        0.0_f64.to_bits(),
        "an empty sum is 0"
    );
}

#[test]
fn the_percentile_matches_the_predecessors_golden() {
    let mut classes = BTreeSet::new();
    golden::each_case("percentile", |case| {
        let values: Vec<i64> = case.input["values"]
            .as_array()
            .expect("values is an array")
            .iter()
            .map(|value| value.as_i64().expect("an integer"))
            .collect();
        let pct = float(&case.input["pct"]);
        assert_eq!(
            Some(pynum::percentile(&values, pct)),
            case.output.as_i64(),
            "the percentile {pct} of {values:?}"
        );
        classes.extend(case.class.clone());
    });
    for class in ["empty", "float_edge", "rank_zero", "fractional_rank"] {
        assert!(classes.contains(class), "no {class} case was examined");
    }
}

#[test]
fn the_nearest_rank_stays_inside_the_list_for_every_pct() {
    // Over 1..=n the percentile is the rank itself, so it lies in 1..=n and equals the ceiling of
    // pct * n floored at 1: the all-inputs bound a golden's cases cannot reach.
    let mut examined = 0_u32;
    for count in 1_i64..=120 {
        let values: Vec<i64> = (1..=count).collect();
        for step in 0_i64..=1000 {
            #[allow(clippy::cast_precision_loss, reason = "a small step of a sweep")]
            let pct = step as f64 / 1000.0;
            let rank = pynum::percentile(&values, pct);
            assert!(
                (1..=count).contains(&rank),
                "rank {rank} of {count} at {pct}"
            );
            examined += 1;
        }
    }
    assert!(examined > 0, "the sweep examined nothing");
    // Outside [0, 1] the predecessor's list index raises; the port stays on the list's ends.
    let values = [3_i64, 1, 2];
    assert_eq!(
        pynum::percentile(&values, 1.5),
        3,
        "above 1 reads the largest"
    );
    assert_eq!(
        pynum::percentile(&values, -0.5),
        1,
        "below 0 reads the smallest"
    );
}
