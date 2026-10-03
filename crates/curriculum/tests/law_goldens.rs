//! The law mastery pillar equals the predecessor's (SPEC-077 A11).

// An integration test is test code: it panics on a malformed golden and prints the examined count.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_curriculum::law::mastery_pillar;

#[test]
fn the_law_mastery_pillar_matches_the_predecessors_golden() {
    let examined = golden::each_case("law_mastery_pillar", |case| {
        let leeches = case.input["law_leech_active"]
            .as_i64()
            .expect("a leech count");
        let ours = mastery_pillar(leeches);
        let theirs = case.output.as_f64().expect("a pillar");
        assert!(
            (ours - theirs).abs() <= 1e-9,
            "the pillar at {leeches}: ours {ours}, theirs {theirs}"
        );
    });
    assert!(examined.count > 0);
}
