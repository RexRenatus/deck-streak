//! The generator, its `choices` and `lgamma` equal `CPython`'s for every golden case (SPEC-302 A3,
//! A4).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;

use deck_streak_kernel::pynum::{self, PyRandom};
use serde_json::Value;

/// The unsigned integer `key` of a golden case's input.
fn count(case: &golden::Case, key: &str) -> u64 {
    case.input[key]
        .as_u64()
        .unwrap_or_else(|| panic!("{key} is an unsigned integer in {}", case.input))
}

/// The floats of the JSON array `value`.
fn floats(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|item| item.as_f64().expect("a number"))
        .collect()
}

#[test]
fn the_generator_matches_cpythons_golden() {
    let mut seeds = BTreeSet::new();
    golden::each_case("pynum_random", |case| {
        let seed = count(case, "seed");
        let mut generator = PyRandom::new(seed);
        let draws: Vec<f64> = (0..count(case, "draws"))
            .map(|_| generator.random())
            .collect();
        let want = floats(&case.output["random"]);
        assert_eq!(draws.len(), want.len(), "the draws of seed {seed}");
        for (index, (got, expected)) in draws.iter().zip(&want).enumerate() {
            assert_eq!(
                got.to_bits(),
                expected.to_bits(),
                "draw {index} of seed {seed}: got {got:e}, CPython {expected:e}"
            );
        }
        let length = usize::try_from(count(case, "length")).expect("a length");
        let population: Vec<usize> = (0..length).collect();
        let picks: Vec<usize> = generator
            .choices(
                &population,
                usize::try_from(count(case, "k")).expect("a count"),
            )
            .expect("a population that holds a value")
            .into_iter()
            .copied()
            .collect();
        let expected: Vec<usize> = case.output["choices"]
            .as_array()
            .expect("an array")
            .iter()
            .map(|pick| usize::try_from(pick.as_u64().expect("an index")).expect("an index"))
            .collect();
        assert_eq!(picks, expected, "the choices of seed {seed} over {length}");
        seeds.insert(seed);
    });
    // The seeds the SPEC names, one of them past 32 bits.
    for seed in [0_u64, 1, 20_260_803, 1_099_511_627_783] {
        assert!(seeds.contains(&seed), "seed {seed} was not examined");
    }
}

#[test]
fn an_empty_population_chooses_nothing_and_none_for_k_zero_is_empty() {
    let mut generator = PyRandom::new(0);
    let none: [u8; 0] = [];
    assert!(
        generator.choices(&none, 3).is_none(),
        "CPython raises on an empty sequence"
    );
    assert_eq!(
        generator.choices(&none, 0),
        Some(Vec::new()),
        "no draw is empty"
    );
}

#[test]
fn lgamma_matches_cpythons_golden() {
    let mut classes = BTreeSet::new();
    golden::each_case("pynum_lgamma", |case| {
        let x = case.input["x"].as_f64().expect("a float argument");
        let want = case.output.as_f64().expect("a float");
        let got = pynum::lgamma(x).unwrap_or_else(|| panic!("lgamma of {x} is a number"));
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "lgamma of {x:e}: got {got:e}, CPython {want:e}"
        );
        classes.extend(case.class.clone());
    });
    for class in ["integral", "non_integral", "small"] {
        assert!(classes.contains(class), "no {class} case was examined");
    }
}

#[test]
fn lgamma_is_not_defined_at_or_below_zero_or_for_a_non_finite_argument() {
    for x in [0.0, -1.5, f64::NAN, f64::INFINITY] {
        assert_eq!(pynum::lgamma(x), None, "lgamma of {x}");
    }
}
