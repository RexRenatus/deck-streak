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

#[test]
fn a_finding_quoting_a_span_of_the_rejected_text_is_dropped() {
    let rejected = "The deadline is 2027-03-01 for this synthetic card\n";
    // A probe writes a fragment with its `repr`: single quotes, or double when the text holds one.
    let single = "output.md:1: date-iso '2027-03-01'";
    let double = "output.md:1: date-iso \"2027-03-01\"";
    let unrelated = "output.md:1: length 'nothing of the rejected text'";
    let Step::Repair(text) = next(1, &failure(&[single, double, unrelated]), rejected) else {
        panic!("attempt one is repaired");
    };
    assert!(
        !text.contains("2027-03-01"),
        "a span found in the rejected text is quoted back: {text}"
    );
    assert!(
        text.contains("- output.md:1: length 'nothing of the rejected text'"),
        "a quoted span absent from the rejected text is kept: {text}"
    );
}

#[test]
fn a_finding_quoting_an_escaped_span_is_dropped() {
    // The word as the rejected text holds it, and as a probe's `repr` prints it: `\xa0`, not the
    // no-break space itself, so the span is not found as written.
    let rejected = "x-new-words: [\"casa\u{a0}grande\"]\n";
    let escaped =
        "i1-glosses: output.md: x-new-words lists 'casa\\xa0grande', which is not glossed";
    let plain = "i1-glosses: output.md: the glosses section names 'nothing escaped'";
    let Step::Repair(text) = next(1, &failure(&[escaped, plain]), rejected) else {
        panic!("attempt one is repaired");
    };
    assert!(
        !text.contains("grande"),
        "an escaped span of the rejected text is quoted back: {text}"
    );
    assert!(
        text.contains("- i1-glosses: output.md: the glosses section names 'nothing escaped'"),
        "a span with no escape and absent from the rejected text is kept: {text}"
    );
}
