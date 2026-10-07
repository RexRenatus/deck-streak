//! The XP crate's study-event guard is the predecessor's (SPEC-360 A2; R3).
//!
//! The golden `study_event.json` holds what the predecessor's `types.py:Review.is_study_event`
//! returned for every review type and ease it enumerates. Ingest's reader keeps its own copy as its
//! filter; the crate's copy is held to the same cases.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_xp::review_xp::is_study_event;

#[test]
fn the_xp_crates_study_event_rule_matches_the_predecessors_golden() {
    golden::each_case("study_event", |case| {
        let kind = case.input["rtype"].as_i64().unwrap_or_default();
        let ease = case.input["ease"].as_i64().unwrap_or_default();
        let expected = case
            .output
            .as_bool()
            .unwrap_or_else(|| panic!("a verdict: {}", case.output));
        assert_eq!(is_study_event(kind, ease), expected, "{}", case.input);
    });
}
