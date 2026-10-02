//! The wallet's rules and constants equal the predecessor's (SPEC-082 A1, A2, A3, A10).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_economy::constants;
use deck_streak_economy::rules;
use serde_json::{Value, json};

/// `economy.json`, embedded at build time.
const ECONOMY_FILE: &str = include_str!("../../../economy.json");

/// A whole number an input holds under `key`.
fn whole(input: &Value, key: &str) -> i64 {
    input[key].as_i64().expect("a whole number in the case")
}

#[test]
fn the_mint_matches_the_parity_golden() {
    let examined = golden::each_case("mint_for_base_xp", |case| {
        let ours = rules::mint_for_base_xp(whole(&case.input, "base_xp"));
        assert_eq!(json!(ours), case.output, "the mint of {}", case.input);
    });
    assert!(examined.count > 0, "the mint golden examined nothing");
}

#[test]
fn the_cap_the_fine_and_the_clip_match_the_parity_goldens() {
    let caps = golden::each_case("daily_loss_cap", |case| {
        let ours = rules::daily_loss_cap(whole(&case.input, "wallet_at_rollover"));
        assert_eq!(json!(ours), case.output, "the cap of {}", case.input);
    });
    let fines = golden::each_case("scaled_fine", |case| {
        let ours = rules::scaled_fine(
            whole(&case.input, "configured"),
            whole(&case.input, "wallet"),
        );
        assert_eq!(json!(ours), case.output, "the fine of {}", case.input);
    });
    let clips = golden::each_case("clip_debit", |case| {
        let (allowed, capped) = rules::clip_debit(
            whole(&case.input, "requested"),
            whole(&case.input, "wallet"),
            whole(&case.input, "cap_remaining"),
        );
        assert_eq!(
            json!([allowed, capped]),
            case.output,
            "the clip of {}",
            case.input
        );
    });
    assert!(caps.count > 0 && fines.count > 0 && clips.count > 0);
}

/// The value of a dotted path in `economy.json`.
fn declared<'a>(file: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(file, |node, key| &node[*key])
}

/// Whether two numbers are the same value, whichever way JSON writes them.
fn same(left: &Value, right: &Value) -> bool {
    left.as_f64() == right.as_f64() && left.as_f64().is_some()
}

#[test]
fn the_coin_constants_and_economy_json_match_the_predecessors() {
    let file: Value = serde_json::from_str(ECONOMY_FILE).expect("economy.json parses");
    let examined = golden::each_case("economy.constants", |case| {
        let name = case.input["name"].as_str().expect("a constant's name");
        let ours: Value = match name {
            "constants.COIN_MINT_XP_DIVISOR" => json!(constants::COIN_MINT_XP_DIVISOR),
            "constants.COIN_MINT_DAILY_CAP" => json!(constants::COIN_MINT_DAILY_CAP),
            "constants.DAILY_LOSS_CAP_COINS" => json!(constants::DAILY_LOSS_CAP_COINS),
            "constants.DAILY_LOSS_CAP_WALLET_FRAC" => json!(constants::DAILY_LOSS_CAP_WALLET_FRAC),
            "constants.FINE_WALLET_FRAC" => json!(constants::FINE_WALLET_FRAC),
            "constants.SHOP_FREEZE_PRICE" => json!(constants::SHOP_FREEZE_PRICE),
            "constants.SHOP_SCROLL_PASS_PRICE" => json!(constants::SHOP_SCROLL_PASS_PRICE),
            "constants.SCROLL_PASS_MINUTES" => json!(constants::SCROLL_PASS_MINUTES),
            "constants.PASS_SURCHARGE_MULT" => json!(constants::PASS_SURCHARGE_MULT),
            "constants.PASS_SURCHARGE_HOURS" => json!(constants::PASS_SURCHARGE_HOURS),
            "constants.STREAK_FREEZE_CAP" => json!(constants::STREAK_FREEZE_CAP),
            other => panic!("a constant the golden holds and this test does not know: {other}"),
        };
        assert!(
            same(&ours, &case.output),
            "{name}: ours {ours}, the predecessor's {}",
            case.output
        );
    });
    assert_eq!(
        examined.count, 11,
        "the constants golden holds eleven constants"
    );

    // economy.json's coin and shop values equal the engine's (SPEC-082 R9).
    let pairs: [(&[&str], Value); 7] = [
        (
            &["coins", "mint", "xp_divisor"],
            json!(constants::COIN_MINT_XP_DIVISOR),
        ),
        (
            &["coins", "mint", "daily_cap"],
            json!(constants::COIN_MINT_DAILY_CAP),
        ),
        (
            &["coins", "loss_cap", "absolute"],
            json!(constants::DAILY_LOSS_CAP_COINS),
        ),
        (
            &["coins", "loss_cap", "wallet_fraction"],
            json!(constants::DAILY_LOSS_CAP_WALLET_FRAC),
        ),
        (
            &["coins", "fines", "wallet_fraction"],
            json!(constants::FINE_WALLET_FRAC),
        ),
        (
            &["shop", "freeze", "price"],
            json!(constants::SHOP_FREEZE_PRICE),
        ),
        (
            &["shop", "scroll_pass", "price"],
            json!(constants::SHOP_SCROLL_PASS_PRICE),
        ),
    ];
    for (path, ours) in &pairs {
        assert!(
            same(declared(&file, path), ours),
            "economy.json {} is {}, the engine's is {ours}",
            path.join("."),
            declared(&file, path)
        );
    }
    assert_eq!(
        declared(&file, &["coins", "wallet_floor"]).as_i64(),
        Some(constants::WALLET_FLOOR),
        "economy.json's wallet floor is the engine's"
    );
}
