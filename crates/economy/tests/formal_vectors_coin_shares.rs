//! The two share rules answer every vector the Lean port writes (#890): for each input in
//! `formal/vectors/coin-shares.jsonl`, written by `formal/lean/Formal/CoinShares.lean`'s port,
//! `daily_loss_cap` caps the same coins and `scaled_fine` fines the same coins, at both ends of the
//! coin type too.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_economy::rules::{daily_loss_cap, scaled_fine};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/coin-shares.jsonl");

/// A vector field as a whole number.
fn number(vector: &Value, name: &str) -> i64 {
    vector[name]
        .as_i64()
        .unwrap_or_else(|| panic!("a number {name} in {vector}"))
}

#[test]
fn the_share_rules_answer_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "CoinShares", "{header}");
    assert_eq!(header["covers"], "crates/economy/src/rules.rs", "{header}");
    let (mut caps, mut fines) = (0_u64, 0_u64);
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        match vector["rule"].as_str() {
            Some("daily_loss_cap") => {
                let ours = daily_loss_cap(number(&vector, "wallet_at_rollover"));
                assert_eq!(ours, number(&vector, "cap"), "{vector}");
                caps += 1;
            }
            Some("scaled_fine") => {
                let ours = scaled_fine(number(&vector, "configured"), number(&vector, "wallet"));
                assert_eq!(ours, number(&vector, "fine"), "{vector}");
                fines += 1;
            }
            _ => panic!("a vector of a known rule: {vector}"),
        }
    }
    println!("examined {caps} cap and {fines} fine vector(s)");
    assert_eq!((caps, fines), (22, 90), "every input the writer prints");
    assert_eq!(header["vectors"].as_u64(), Some(caps + fines), "{header}");
}

#[test]
fn the_recorded_counterexamples_answer_as_proved() {
    // `a_cap_rounded_up_takes_more_than_its_share`: a wallet of 1 caps the day's loss at 0 coins.
    assert_eq!(daily_loss_cap(1), 0);
    // `a_fine_rounded_down_takes_less_than_its_share`: a fine of 1 against a wallet of 51 is 2.
    assert_eq!(scaled_fine(1, 51), 2);
    // `a_share_taken_as_one_product_leaves_the_coin_type`: the largest wallet caps at 100 and is
    // fined its exact share, with no step past `i64`.
    assert_eq!(daily_loss_cap(i64::MAX), 100);
    assert_eq!(scaled_fine(1, i64::MAX), 184_467_440_737_095_517);
}
