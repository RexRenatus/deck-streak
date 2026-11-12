//! The next milestone is the predecessor's (SPEC-073 A17, A18; R14): the nearest unreached rung by
//! its remaining fraction, ties to reviews, then the streak, then mature cards; a complete ladder
//! contributes nothing, and three complete ladders report the top review rung at 100%.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_progression::milestone::{
    Ladder, MATURE_LADDER, REVIEW_LADDER, STREAK_LADDER, next_milestone,
};
use serde_json::Value;

/// A golden field as a count.
fn count(value: &Value) -> u64 {
    value
        .as_u64()
        .unwrap_or_else(|| panic!("a count, not {value}"))
}

#[test]
fn next_milestone_matches_the_parity_golden() {
    let mut ladders = [0_usize; 3];
    golden::each_case("next_milestone", |case| {
        let input = &case.input;
        let ours = next_milestone(
            count(&input["lifetime_reviews"]),
            count(&input["streak_days"]),
            count(&input["mature_total"]),
        );
        let theirs = &case.output;
        assert_eq!(
            ours.ladder.label(),
            theirs["label"].as_str().expect("a label"),
            "{input}"
        );
        assert_eq!(
            ours.ladder.emoji(),
            theirs["emoji"].as_str().expect("an emoji"),
            "{input}"
        );
        assert_eq!(
            ours.current,
            count(&theirs["current"]),
            "the current value of {input}"
        );
        assert_eq!(
            ours.target,
            count(&theirs["target"]),
            "the target of {input}"
        );
        assert_eq!(
            ours.remaining,
            count(&theirs["remaining"]),
            "the remainder of {input}"
        );
        assert!(
            (ours.pct - theirs["pct"].as_f64().expect("a percentage")).abs() == 0.0,
            "the percentage of {input}: {} against {}",
            ours.pct,
            theirs["pct"]
        );
        let at = Ladder::ALL
            .iter()
            .position(|ladder| *ladder == ours.ladder)
            .expect("a ladder");
        ladders[at] += 1;
    });
    println!("examined milestones by ladder (reviews, streak, mature): {ladders:?}");
    assert!(
        ladders.iter().all(|seen| *seen > 0),
        "every ladder wins a case"
    );
}

#[test]
fn a_complete_ladder_contributes_nothing_and_all_complete_reports_the_top_review_rung() {
    let top = |ladder: &[u64]| *ladder.last().expect("a rung");
    let (reviews, streak, mature) = (
        top(&REVIEW_LADDER),
        top(&STREAK_LADDER),
        top(&MATURE_LADDER),
    );
    // The review ladder complete: the nearest rung is the streak's or the mature cards'.
    let cases = [
        ((reviews, 0, 0), Ladder::Streak, 7),
        ((reviews, streak, 0), Ladder::Mature, 100),
        ((reviews, 0, mature), Ladder::Streak, 7),
        ((0, streak, mature), Ladder::Reviews, 100),
    ];
    println!("examined {} partly complete case(s)", cases.len());
    for ((r, s, m), ladder, target) in cases {
        let ours = next_milestone(r, s, m);
        assert_eq!(
            (ours.ladder, ours.target),
            (ladder, target),
            "{r}, {s}, {m}"
        );
    }
    // All three complete, and beyond: the top review rung at 100%.
    for (r, s, m) in [
        (reviews, streak, mature),
        (reviews + 1, streak * 2, mature + 7),
    ] {
        let ours = next_milestone(r, s, m);
        assert_eq!(ours.ladder, Ladder::Reviews);
        assert_eq!(
            (ours.current, ours.target, ours.remaining),
            (reviews, reviews, 0)
        );
        assert!((ours.pct - 100.0).abs() == 0.0, "100%: {}", ours.pct);
    }
}
