//! The daily bonuses are the predecessor's (SPEC-072 A14; R11).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_progression::bonus::{DayFacts, daily_bonuses};

#[test]
fn daily_bonus_grants_match_the_parity_golden() {
    golden::each_case("daily_bonus_grants", |case| {
        let number = |name: &str| {
            case.input[name]
                .as_i64()
                .unwrap_or_else(|| panic!("{name} is an integer in {}", case.input))
        };
        let flag = |name: &str| {
            let value = &case.input[name];
            value
                .as_bool()
                .or_else(|| value.as_i64().map(|number| number != 0))
                .unwrap_or_else(|| panic!("{name} is a flag in {}", case.input))
        };
        let facts = DayFacts {
            studied: flag("studied"),
            backlog_zero: flag("backlog_zero"),
            streak_days: number("streak_days"),
            score_total: number("score_total"),
            graduations: number("graduations"),
        };
        let ours: Vec<(String, u64)> = daily_bonuses(&facts)
            .into_iter()
            .map(|(source, amount)| (source.to_owned(), u64::from(amount)))
            .collect();
        let expected: Vec<(String, u64)> = case
            .output
            .as_array()
            .expect("the bonuses are a list")
            .iter()
            .map(|grant| {
                (
                    grant["source"].as_str().expect("a source").to_owned(),
                    grant["amount"].as_u64().expect("an amount"),
                )
            })
            .collect();
        assert_eq!(ours, expected, "the bonuses of {}", case.input);
    });
}
