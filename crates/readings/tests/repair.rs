//! The one repair (SPEC-046 R7): its cap, and the text it may carry back to the model.
//!
//! Every reading and finding here is synthetic.

use deck_streak_readings::coverage::GateFailure;
use deck_streak_readings::repair::{MAX_ATTEMPTS, Step, next};
use deck_streak_readings::state::ReadingGate;

fn failure(findings: &[&str]) -> GateFailure {
    GateFailure {
        gate: ReadingGate::Anchors,
        findings: findings.iter().map(|f| (*f).to_owned()).collect(),
    }
}

#[test]
fn a_topic_gets_the_first_attempt_and_exactly_one_repair() {
    assert_eq!(MAX_ATTEMPTS, 2, "the first attempt and one repair");
    assert!(matches!(next(1, &failure(&["a"]), ""), Step::Repair(_)));
    assert_eq!(
        next(2, &failure(&["a"]), ""),
        Step::Fail(ReadingGate::Anchors),
        "a second failure ends the topic on the gate"
    );
}

#[test]
fn a_finding_that_repeats_a_long_rejected_line_is_dropped() {
    // Sixteen characters is long enough to be a quote; fifteen is not.
    let sixteen = "abcdefghijklmnop";
    let fifteen = "abcdefghijklmno";
    assert_eq!(sixteen.chars().count(), 16);
    let rejected = format!("{sixteen}\n{fifteen}\n");
    let Step::Repair(text) = next(1, &failure(&[sixteen, fifteen, "kept"]), &rejected) else {
        panic!("attempt one is repaired");
    };
    assert!(
        !text.contains(sixteen),
        "a sixteen-character line is a quote: {text}"
    );
    assert!(
        text.contains("- kept"),
        "an ordinary finding is kept: {text}"
    );
    let Step::Repair(shorter) = next(1, &failure(&[fifteen]), fifteen) else {
        panic!("attempt one is repaired");
    };
    assert!(
        shorter.contains(fifteen),
        "a fifteen-character line is too short to count as a quote: {shorter}"
    );
}

#[test]
fn a_finding_carrying_a_fence_marker_is_dropped() {
    let Step::Repair(text) = next(1, &failure(&["<untrusted x>", "</untrusted>", "plain"]), "")
    else {
        panic!("attempt one is repaired");
    };
    assert!(!text.contains("untrusted"), "{text}");
    assert!(text.contains("- plain"), "{text}");
}
