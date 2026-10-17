//! The chest rules answer every vector the Lean port writes (#102, #103): for each input in
//! `formal/vectors/chest.jsonl`, written by `formal/lean/Formal/Chest.lean`'s port,
//! `epic_odds_pts` gives the same Epic points, `roll_rarity` the same rarity and `payout_xp` the
//! same XP. This test is a cross-check of the port against the code, not a red-first criterion:
//! the code it reads was green before the port was written.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_quests::chests::{Rarity, epic_odds_pts, payout_xp, roll_rarity};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/chest.jsonl");

/// A vector field as a whole number.
fn number(vector: &Value, name: &str) -> i64 {
    vector[name]
        .as_i64()
        .unwrap_or_else(|| panic!("a number {name} in {vector}"))
}

/// A vector field as text.
fn text<'v>(vector: &'v Value, name: &str) -> &'v str {
    vector[name]
        .as_str()
        .unwrap_or_else(|| panic!("a string {name} in {vector}"))
}

/// A whole number of points or XP as the code's float: every value here is far below 2^53.
#[allow(
    clippy::cast_precision_loss,
    reason = "the vectors' counters and buffs are small whole numbers"
)]
fn float(value: i64) -> f64 {
    value as f64
}

/// The draw a vector names: a quarter `q0` to `q3`, or `top`, the largest draw below 1.
fn draw(vector: &Value) -> f64 {
    match text(vector, "draw") {
        "q0" => 0.0,
        "q1" => 0.25,
        "q2" => 0.5,
        "q3" => 0.75,
        "top" => 1.0 - f64::EPSILON / 2.0,
        other => panic!("a known draw, not {other}, in {vector}"),
    }
}

/// The rarity a vector names.
fn rarity(vector: &Value) -> Rarity {
    match text(vector, "rarity") {
        "common" => Rarity::Common,
        "rare" => Rarity::Rare,
        "epic" => Rarity::Epic,
        "legendary" => Rarity::Legendary,
        other => panic!("a known rarity, not {other}, in {vector}"),
    }
}

#[test]
fn the_chest_rules_answer_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "Chest", "{header}");
    assert_eq!(header["covers"], "crates/quests/src/chests.rs", "{header}");
    assert_eq!(header["anchor"], "epic_odds_pts", "{header}");
    let (mut odds, mut rolls, mut payouts) = (0_u64, 0_u64, 0_u64);
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        match vector["rule"].as_str() {
            Some("epic_odds_pts") => {
                let ours = epic_odds_pts(
                    number(&vector, "since_epic"),
                    float(number(&vector, "buff_pts")),
                );
                assert_eq!(
                    ours.to_bits(),
                    float(number(&vector, "odds")).to_bits(),
                    "{vector}"
                );
                odds += 1;
            }
            Some("roll_rarity") => {
                let ours = roll_rarity(
                    draw(&vector),
                    number(&vector, "since_epic"),
                    number(&vector, "since_legendary"),
                    float(number(&vector, "buff_pts")),
                );
                assert_eq!(ours.name(), text(&vector, "rarity"), "{vector}");
                rolls += 1;
            }
            Some("payout_xp") => {
                let ours = payout_xp(
                    rarity(&vector),
                    draw(&vector),
                    number(&vector, "session_base_xp"),
                );
                assert_eq!(ours, number(&vector, "payout"), "{vector}");
                payouts += 1;
            }
            _ => panic!("a vector of a known rule: {vector}"),
        }
    }
    println!("examined {odds} odds, {rolls} roll and {payouts} payout vector(s)");
    assert_eq!(
        (odds, rolls, payouts),
        (84, 840, 300),
        "every input the writer prints"
    );
    assert_eq!(
        header["vectors"].as_u64(),
        Some(odds + rolls + payouts),
        "{header}"
    );
}

#[test]
fn the_recorded_counterexamples_answer_as_proved() {
    // `epic_odds_with_no_ceiling_leave_the_range`: a counter of 30 ramps past the ceiling, which
    // holds the odds at 40.
    assert_eq!(epic_odds_pts(30, 0.0).to_bits(), 40.0_f64.to_bits());
    // `guarantees_read_on_the_bare_counters_come_one_chest_late`: the 40th chest since the last
    // Legendary is a Legendary even for the draw that folds to a Common.
    assert_eq!(
        roll_rarity(1.0 - f64::EPSILON / 2.0, 0, 39, 0.0),
        Rarity::Legendary
    );
    // `a_payout_with_no_cap_passes_it`: a Rare at the band's bottom, for a session of 0 XP, pays
    // the floor's 25, not the band's 30.
    assert_eq!(payout_xp(Rarity::Rare, 0.0, 0), 25);
}
