//! The token bonus answers every vector the Lean port writes (#102, #103): for each input in
//! `formal/vectors/token-bonus.jsonl`, written by `formal/lean/Formal/TokenBonus.lean`'s port,
//! `token_bonus_xp` gives the same bonus, `None` when no study review falls in the window on the
//! day. This test is a cross-check of the port against the code, not a red-first criterion: the
//! code it reads was green before the port was written. It also backs the window's inclusive
//! start and exclusive end, which the vectors place on both sides of.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{Hour, StudyDay, StudyDayRule, UtcMillis, UtcOffset};
use deck_streak_quests::tokens::{ReviewXp, TokenWindow, token_bonus_xp};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/token-bonus.jsonl");

/// A vector field as a whole number.
fn number(vector: &Value, name: &str) -> i64 {
    vector[name]
        .as_i64()
        .unwrap_or_else(|| panic!("a number {name} in {vector}"))
}

/// A review line `[id, kind, ease, xp]` as the review and its base XP.
fn review_of(line: &Value, vector: &Value) -> ReviewXp {
    let field = |at: usize| {
        line[at]
            .as_i64()
            .unwrap_or_else(|| panic!("a number at {at} of {line} in {vector}"))
    };
    ReviewXp {
        review: Review {
            id: field(0),
            card_id: 1,
            ease: field(2),
            interval: 0,
            last_interval: 0,
            factor: 2500,
            taken_ms: 0,
            kind: field(1),
        },
        base_xp: field(3),
    }
}

#[test]
fn the_token_bonus_answers_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "TokenBonus", "{header}");
    assert_eq!(header["covers"], "crates/quests/src/tokens.rs", "{header}");
    assert_eq!(header["anchor"], "token_bonus_xp", "{header}");
    let (mut bonuses, mut none) = (0_u64, 0_u64);
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        assert_eq!(vector["rule"], "token_bonus_xp", "{vector}");
        let hour = u8::try_from(number(&vector, "rollover_hour"))
            .ok()
            .and_then(Hour::new)
            .expect("a rollover hour");
        let offset = UtcOffset::from_minutes(0).expect("the zero offset");
        let rule = StudyDayRule::new(hour, offset);
        let day = StudyDay::from_epoch_day(number(&vector, "study_day"));
        let window = TokenWindow {
            start: UtcMillis::from_epoch_millis(number(&vector, "start")),
            end: UtcMillis::from_epoch_millis(number(&vector, "end")),
        };
        let reviews: Vec<ReviewXp> = vector["reviews"]
            .as_array()
            .expect("the reviews")
            .iter()
            .map(|row| review_of(row, &vector))
            .collect();
        let ours = token_bonus_xp(window, day, rule, &reviews);
        let theirs = if vector["bonus"].is_null() {
            none += 1;
            None
        } else {
            bonuses += 1;
            Some(number(&vector, "bonus"))
        };
        assert_eq!(ours, theirs, "{vector}");
    }
    println!("examined {bonuses} bonus and {none} no-bonus vector(s)");
    assert_eq!((bonuses, none), (84, 330), "every input the writer prints");
    assert_eq!(header["vectors"].as_u64(), Some(bonuses + none), "{header}");
}
