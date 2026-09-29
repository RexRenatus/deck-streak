//! The Ascendant buff and the day's bonuses are the predecessor's (SPEC-072 A23 to A25; R18,
//! R22).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_progression::consistency::{ascendant_arms, day_bonuses};

/// A golden field that is a number or a flag, as a number.
fn number(value: &serde_json::Value) -> i64 {
    value
        .as_i64()
        .or_else(|| value.as_bool().map(i64::from))
        .unwrap_or_else(|| panic!("a number or a flag, not {value}"))
}

#[test]
fn the_day_bonuses_match_the_parity_golden() {
    golden::each_case("day_bonuses", |case| {
        let input = &case.input;
        let ours = day_bonuses(
            number(&input["base"]),
            number(&input["run"]),
            number(&input["buff"]) != 0,
            number(&input["reviews"]),
            number(&input["reviews_law"]),
        );
        assert_eq!(
            i64::from(ours.consistency),
            case.output["consistency"].as_i64().expect("consistency"),
            "the consistency bonus of {input}"
        );
        assert_eq!(
            ours.ascendant.map(i64::from),
            case.output["ascendant"].as_i64(),
            "the Ascendant bonus of {input}"
        );
    });
}

#[test]
fn the_ascendant_arms_as_the_predecessor_arms_it() {
    let mut armed = 0_u32;
    golden::each_case("ascendant_arms", |case| {
        let input = &case.input;
        let ours = ascendant_arms(
            number(&input["buff"]) != 0,
            number(&input["skip"]) != 0,
            input["rollup_reviews"].as_i64(),
            number(&input["backlog_zero"]),
        );
        let expected = case.output.as_bool().expect("a flag");
        assert_eq!(ours, expected, "the arming of {input}");
        armed += u32::from(expected);
    });
    assert!(
        armed > 0,
        "no case armed the buff, so the arming was never proved"
    );
}

#[test]
fn the_ascendant_never_arms_on_a_skip_day() {
    for buff in [false, true] {
        for reviews in [None, Some(0), Some(50)] {
            for backlog_zero in [0, 100] {
                assert!(
                    !ascendant_arms(buff, true, reviews, backlog_zero),
                    "a skip day is never armed"
                );
            }
        }
    }
    assert!(
        ascendant_arms(false, false, Some(50), 100),
        "the same day, not a skip, is armed"
    );
}
