//! The next milestone answers every vector the Lean port writes (SPEC-073 R14): for each input in
//! `formal/vectors/next-milestone.jsonl`, written by `formal/lean/Formal/NextMilestone.lean`'s port,
//! `next_milestone` picks the same ladder, rung, current value and remainder.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_progression::milestone::{Ladder, next_milestone};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/next-milestone.jsonl");

/// A vector field as a count.
fn count(vector: &Value, name: &str) -> u64 {
    vector[name]
        .as_u64()
        .unwrap_or_else(|| panic!("a count {name} in {vector}"))
}

#[test]
fn next_milestone_answers_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "NextMilestone", "{header}");
    assert_eq!(
        header["covers"], "crates/progression/src/milestone.rs",
        "{header}"
    );
    let mut examined = 0_u64;
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        let ours = next_milestone(
            count(&vector, "reviews"),
            count(&vector, "streak"),
            count(&vector, "mature"),
        );
        let theirs = &vector["milestone"];
        let at = usize::try_from(count(theirs, "ladder")).expect("a ladder index");
        assert_eq!(
            (ours.ladder, ours.target, ours.current, ours.remaining),
            (
                Ladder::ALL[at],
                count(theirs, "target"),
                count(theirs, "current"),
                count(theirs, "remaining"),
            ),
            "{vector}"
        );
        examined += 1;
    }
    println!("examined {examined} next-milestone vector(s)");
    assert!(examined > 0, "the vectors examine something");
    assert_eq!(header["vectors"].as_u64(), Some(examined), "{header}");
}

#[test]
fn the_recorded_counterexamples_answer_as_proved() {
    // `pick_last_violates`: the last present ladder does not win by being last. One review short
    // of 100 is the smallest fraction left, so the reviews win over the streak and mature cards.
    let nearest = next_milestone(99, 0, 0);
    assert_eq!((nearest.ladder, nearest.target), (Ladder::Reviews, 100));
    // `pick_later_on_tie_violates`: with the reviews complete, the streak's 7 of 7 ties the mature
    // cards' 100 of 100, and the earlier ladder keeps the tie.
    let tie = next_milestone(50_000, 0, 0);
    assert_eq!((tie.ladder, tie.target), (Ladder::Streak, 7));
    // `no_top_rung_violates`: three complete ladders answer the top review rung, never nothing.
    let complete = next_milestone(50_000, 1_000, 5_000);
    assert_eq!(
        (
            complete.ladder,
            complete.current,
            complete.target,
            complete.remaining
        ),
        (Ladder::Reviews, 50_000, 50_000, 0)
    );
}
