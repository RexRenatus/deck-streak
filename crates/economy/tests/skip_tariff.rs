//! The skip tariff's ladder (SPEC-083 A14; R8, R10): read from `economy.json`, equal to the
//! predecessor's constant, and priced at every count of a month's earlier skips, past the ladder's
//! last step included.

// An integration test is test code: its helpers panic on a malformed golden, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_economy::tariff::{ladder, price};
use serde_json::Value;

/// `economy.json`, embedded at build time.
const ECONOMY_FILE: &str = include_str!("../../../economy.json");

#[test]
fn the_tariff_ladder_is_read_from_economy_json_and_equals_the_golden() {
    let economy: Value = serde_json::from_str(ECONOMY_FILE).expect("economy.json is JSON");
    let file: Vec<i64> = economy["streak"]["skip_tariff_coins"]
        .as_array()
        .expect("a ladder of prices")
        .iter()
        .map(|coins| coins.as_i64().expect("a whole price"))
        .collect();
    assert_eq!(ladder(), file.as_slice(), "the ladder is economy.json's");
    let mut ladders = 0;
    golden::each_case("skip.constants", |case| {
        if case.input["name"] == "constants.SKIP_TARIFF_LADDER" {
            ladders += 1;
            assert_eq!(
                Value::from(ladder().to_vec()),
                case.output,
                "the ladder equals the predecessor's"
            );
        }
    });
    assert_eq!(ladders, 1, "the golden records the ladder once");
    // Every count from a month's first skip to past the ladder's end: the last price repeats.
    let prices: Vec<i64> = (0..=6).map(|earlier| price(ladder(), earlier)).collect();
    println!("A14: examined {} counts", prices.len());
    assert_eq!(prices, [0, 50, 100, 100, 100, 100, 100]);
    assert_eq!(price(&[], 0), 0, "an empty ladder prices nothing");
}
