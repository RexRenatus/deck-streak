//! The collection day number equals the predecessor's (SPEC-071 A11; R7): a study day minus the
//! study day of the collection's creation instant, under the configured rule, proved against
//! `goldens/today_day_number.json` on both sides of every rollover.

// An integration test is test code: its helpers panic on a malformed golden, and the golden reader
// prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_ingest::calendar::collection_day_number;
use deck_streak_kernel::{Hour, StudyDayRule, UtcMillis, UtcOffset};
use serde_json::{Value, json};

fn integer(input: &Value, key: &str) -> i64 {
    input[key]
        .as_i64()
        .unwrap_or_else(|| panic!("the case's {key} is an integer: {input}"))
}

#[test]
fn the_collection_day_number_matches_the_predecessors_golden() {
    let mut rollover_cases = 0;
    golden::each_case("today_day_number", |case| {
        let input = &case.input;
        let hour = u8::try_from(integer(input, "rollover_hour"))
            .ok()
            .and_then(Hour::new)
            .expect("an hour");
        let offset = i16::try_from(integer(input, "utc_offset_minutes"))
            .ok()
            .and_then(UtcOffset::from_minutes)
            .expect("an offset");
        let rule = StudyDayRule::new(hour, offset);
        let created = UtcMillis::from_epoch_millis(integer(input, "creation_sec") * 1000);
        let day = rule.study_day(UtcMillis::from_epoch_millis(integer(input, "now_ms")));
        assert_eq!(
            json!(collection_day_number(rule, created, day)),
            case.output,
            "the day number of {input}"
        );
        if case.class.as_deref() == Some("rollover") {
            rollover_cases += 1;
        }
    });
    assert!(rollover_cases > 0, "the golden carries its rollover class");
}
