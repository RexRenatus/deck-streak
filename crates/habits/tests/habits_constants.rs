//! Every habit constant part 078a uses equals the predecessor's (SPEC-078 A14; R3): the XP a
//! minute earns, the daily cap, the entry's bound, the weekly goal, its bonus and the presets.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_habits::minutes::{
    READING_GOAL_XP_BONUS, READING_MAX_ENTRY_MIN, READING_PRESETS, READING_WEEKLY_GOAL_MIN,
    READING_XP_DAILY_CAP_PER_LANG, READING_XP_PER_MIN,
};
use serde_json::{Value, json};

#[test]
fn the_habit_constants_equal_the_predecessors() {
    let examined = golden::each_case("habits.constants", |case| {
        let name = case.input["name"].as_str().expect("a name");
        let port: Value = match name {
            "constants.READING_XP_PER_MIN" => json!(READING_XP_PER_MIN),
            "constants.READING_XP_DAILY_CAP_PER_LANG" => json!(READING_XP_DAILY_CAP_PER_LANG),
            "constants.READING_MAX_ENTRY_MIN" => json!(READING_MAX_ENTRY_MIN),
            "constants.READING_WEEKLY_GOAL_MIN" => json!(READING_WEEKLY_GOAL_MIN),
            "constants.READING_GOAL_XP_BONUS" => json!(READING_GOAL_XP_BONUS),
            "bot._READ_PRESETS" => json!(READING_PRESETS),
            other => panic!("{other} has no port constant"),
        };
        assert_eq!(port, case.output, "{name}");
    });
    assert_eq!(examined.count, 6, "every constant is examined");
}
