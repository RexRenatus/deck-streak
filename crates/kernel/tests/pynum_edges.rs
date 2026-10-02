//! Exact-value edge cases of `pynum` (SPEC-302): each row of `fixtures/pynum_edges.txt` is a
//! `CPython` 3.12 result, compared bit for bit, chosen to land on the branches the mutation sweep
//! found the goldens never reach (tie-to-even, exact-halfway and below-half remainders, limb
//! borrows, subnormal and near-overflow ranges, the `round` digit-count boundaries, the Lanczos
//! switch point).

// An integration test is test code: its parser panics on a malformed row.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_kernel::pynum;

const ROWS: &str = include_str!("fixtures/pynum_edges.txt");

fn bits(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).expect("a hex bit pattern"))
}

fn list(hex: &str) -> Vec<f64> {
    hex.split(',').map(bits).collect()
}

/// Runs `check` on the fields of every row whose first word is `kind`, and returns the count.
fn each(kind: &str, mut check: impl FnMut(&[&str])) -> usize {
    let mut count = 0;
    for row in ROWS.lines() {
        let fields: Vec<&str> = row.split(' ').collect();
        if fields.first() == Some(&kind) {
            check(fields.get(1..).expect("a kind names its fields"));
            count += 1;
        }
    }
    count
}

#[test]
fn the_mean_of_every_edge_list_is_cpythons_to_the_last_bit() {
    let examined = each("mean", |f| {
        let got = pynum::mean(&list(f[0])).expect("a non-empty list has a mean");
        assert_eq!(got.to_bits(), bits(f[1]).to_bits(), "mean of {}", f[0]);
    });
    assert!(examined > 1000, "examined {examined}");
}

#[test]
fn the_round_of_every_edge_value_is_cpythons_to_the_last_bit() {
    let examined = each("round", |f| {
        let ndigits: i32 = f[1].parse().expect("a digit count");
        let got = pynum::round(bits(f[0]), ndigits);
        assert_eq!(
            got.to_bits(),
            bits(f[2]).to_bits(),
            "round({}, {ndigits})",
            f[0]
        );
    });
    assert!(examined > 1500, "examined {examined}");
}

#[test]
fn the_sum_of_every_edge_list_is_cpythons_to_the_last_bit() {
    let examined = each("sum", |f| {
        let got = pynum::sum(list(f[0]));
        assert_eq!(got.to_bits(), bits(f[1]).to_bits(), "sum of {}", f[0]);
    });
    assert!(examined >= 5, "examined {examined}");
}

#[test]
fn the_lgamma_at_the_switch_points_is_cpythons_to_the_last_bit() {
    let examined = each("lgamma", |f| {
        let got = pynum::lgamma(bits(f[0])).expect("a positive argument has a value");
        assert_eq!(got.to_bits(), bits(f[1]).to_bits(), "lgamma of {}", f[0]);
    });
    assert!(examined >= 10, "examined {examined}");
}

#[test]
fn a_non_finite_list_has_the_mean_cpython_gives_it() {
    assert!(pynum::mean(&[f64::INFINITY, 1.0]).is_some_and(|m| m == f64::INFINITY));
    assert!(pynum::mean(&[f64::NEG_INFINITY, 1.0, 2.0]).is_some_and(|m| m == f64::NEG_INFINITY));
    assert!(pynum::mean(&[f64::INFINITY, f64::NEG_INFINITY]).is_some_and(f64::is_nan));
    assert!(pynum::mean(&[f64::NAN, 1.0]).is_some_and(f64::is_nan));
}

#[test]
fn lgamma_of_a_finite_argument_whose_result_overflows_is_none() {
    // The six arguments CPython's `math.lgamma` raises `OverflowError` ("math range error") for,
    // as bit patterns: from about 2.6e305 up to the largest float.
    let arguments = [
        "7f57b236a943b4a5",
        "7f76c8e5ca239029",
        "7fac7b1f3cac7433",
        "7fe1ccf385ebc8a0",
        "7fefffffffffffff",
        "7f737a7dfad029c5",
    ];
    for hex in arguments {
        assert_eq!(pynum::lgamma(bits(hex)), None, "lgamma of {hex}");
    }
}
