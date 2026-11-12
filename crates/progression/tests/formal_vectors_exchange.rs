//! The XP exchange readout answers every vector the Lean port writes (SPEC-075 R4, R5, A13): for
//! each input in `formal/vectors/exchange.jsonl`, written by `formal/lean/Formal/Exchange.lean`'s
//! port, `exchange_rates` answers the same buckets in the same order, with the same XP, graduated
//! cards and defined flag, and each defined rate is the bucket's XP over its graduated cards.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;

use deck_streak_kernel::StudyDay;
use deck_streak_progression::board::{DayScore, best_day};
use deck_streak_progression::exchange::{XpRow, bucket, exchange_rates};
use serde_json::{Value, json};

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/exchange.jsonl");

/// A vector field as an integer.
fn int(value: &Value, name: &str) -> i64 {
    value[name]
        .as_i64()
        .unwrap_or_else(|| panic!("an integer {name} in {value}"))
}

/// A vector field's array.
fn array<'a>(value: &'a Value, name: &str) -> &'a Vec<Value> {
    value[name]
        .as_array()
        .unwrap_or_else(|| panic!("an array {name} in {value}"))
}

/// The study day with epoch day number `day`.
const fn day(day: i64) -> StudyDay {
    StudyDay::from_epoch_day(day)
}

/// An XP row.
fn row(on: i64, source: &str, amount: i64) -> XpRow {
    XpRow {
        study_day: day(on),
        source: source.to_owned(),
        amount,
    }
}

#[test]
fn exchange_rates_answers_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "Exchange", "{header}");
    assert_eq!(
        header["covers"], "crates/progression/src/exchange.rs",
        "{header}"
    );
    let mut examined = 0_u64;
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        let rows: Vec<XpRow> = array(&vector, "rows")
            .iter()
            .map(|r| {
                row(
                    int(r, "day"),
                    r["source"].as_str().expect("a source"),
                    int(r, "amount"),
                )
            })
            .collect();
        let graduations: BTreeMap<StudyDay, i64> = array(&vector, "graduations")
            .iter()
            .map(|g| (day(int(g, "day")), int(g, "graduations")))
            .collect();
        let ours = exchange_rates(&rows, &graduations);
        let answered: Vec<Value> = ours
            .iter()
            .map(|rate| {
                json!({
                    "source": rate.source,
                    "total_xp": rate.total_xp,
                    "graduated_cards": rate.graduated_cards,
                    "rate_defined": rate.rate_defined,
                })
            })
            .collect();
        assert_eq!(&answered, array(&vector, "rates"), "{vector}");
        for rate in &ours {
            #[allow(
                clippy::cast_precision_loss,
                reason = "the vectors' counts are far below 2^53"
            )]
            let expected = rate
                .rate_defined
                .then(|| rate.total_xp as f64 / rate.graduated_cards as f64);
            assert_eq!(
                rate.rate.map(f64::to_bits),
                expected.map(f64::to_bits),
                "{vector}"
            );
        }
        examined += 1;
    }
    println!("examined {examined} exchange vector(s)");
    assert!(examined > 0, "the vectors examine something");
    assert_eq!(header["vectors"].as_u64(), Some(examined), "{header}");
}

#[test]
fn the_exchange_counterexamples_answer_as_proved() {
    // `rows_without_a_rollup_dropped_violates`: a row on a day with no rollup keeps its XP.
    let unrolled = exchange_rates(&[row(5, "a", 10)], &BTreeMap::new());
    assert_eq!(
        unrolled
            .iter()
            .map(|rate| (rate.source.as_str(), rate.total_xp))
            .collect::<Vec<_>>(),
        vec![("a", 10)]
    );
    // `per_row_denominator_violates`: two rows of one bucket on one day count its graduations once.
    let once = exchange_rates(
        &[row(1, "a", 1), row(1, "a", 1)],
        &BTreeMap::from([(day(1), 2)]),
    );
    assert_eq!(
        once.iter()
            .map(|rate| (rate.total_xp, rate.graduated_cards))
            .collect::<Vec<_>>(),
        vec![(2, 2)]
    );
    // `defined_on_xp_violates`: XP with no graduation has no rate.
    let undefined = exchange_rates(&[row(1, "a", 5)], &BTreeMap::new());
    assert_eq!(
        undefined
            .iter()
            .map(|rate| (rate.rate_defined, rate.rate))
            .collect::<Vec<_>>(),
        vec![(false, None)]
    );
    // `last_colon_bucket_violates`: a source's bucket ends at its first `:`.
    assert_eq!(bucket("a:b:"), "a:");
    // `oldest_on_tie_violates`: of two days that score as much, the best day is the later one.
    let scores = [
        DayScore {
            day: day(2),
            score: 5,
        },
        DayScore {
            day: day(1),
            score: 5,
        },
    ];
    assert_eq!(best_day(&scores).map(|best| best.day), Some(day(2)));
}
