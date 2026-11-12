//! The coin rules answer every vector the Lean port writes (#106): for each input in
//! `formal/vectors/wallet.jsonl`, written by `formal/lean/Formal/Wallet.lean`'s port, `clip_debit`
//! allows the same amount and forgives the same way, and `mint_for_base_xp` mints the same coins.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_economy::rules::{clip_debit, mint_for_base_xp};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/wallet.jsonl");

/// A vector field as a whole number.
fn number(vector: &Value, name: &str) -> i64 {
    vector[name]
        .as_i64()
        .unwrap_or_else(|| panic!("a number {name} in {vector}"))
}

#[test]
fn the_coin_rules_answer_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "Wallet", "{header}");
    assert_eq!(header["covers"], "crates/economy/src/rules.rs", "{header}");
    let (mut clips, mut mints) = (0_u64, 0_u64);
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        match vector["rule"].as_str() {
            Some("clip_debit") => {
                let ours = clip_debit(
                    number(&vector, "requested"),
                    number(&vector, "wallet"),
                    number(&vector, "cap_remaining"),
                );
                let forgiven = vector["forgiven"]
                    .as_bool()
                    .unwrap_or_else(|| panic!("a forgiven flag in {vector}"));
                assert_eq!(ours, (number(&vector, "allowed"), forgiven), "{vector}");
                clips += 1;
            }
            Some("mint_for_base_xp") => {
                let ours = mint_for_base_xp(number(&vector, "base_xp"));
                assert_eq!(ours, number(&vector, "minted"), "{vector}");
                mints += 1;
            }
            _ => panic!("a vector of a known rule: {vector}"),
        }
    }
    println!("examined {clips} clip and {mints} mint vector(s)");
    assert_eq!((clips, mints), (729, 18), "every input the writer prints");
    assert_eq!(header["vectors"].as_u64(), Some(clips + mints), "{header}");
}

#[test]
fn the_recorded_counterexamples_answer_as_proved() {
    // `a_clip_that_skips_the_wallet_overpays`: a request of 5 against a wallet of 2 and a
    // remaining cap of 10 pays the wallet's 2 and forgives the rest.
    assert_eq!(clip_debit(5, 2, 10), (2, true));
    // `a_mint_with_no_daily_cap_leaves_the_range`: a base of 2,500 mints the cap of 40, not 100.
    assert_eq!(mint_for_base_xp(2_500), 40);
    // `a_mint_over_the_remainder_falls_as_the_base_grows`: a base of 24 mints nothing and a base
    // of 25 mints one coin.
    assert_eq!((mint_for_base_xp(24), mint_for_base_xp(25)), (0, 1));
}
