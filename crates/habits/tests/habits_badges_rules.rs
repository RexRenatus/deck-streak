//! The habit badges' conditions (SPEC-078 A19a, A19b; R9, R17): they equal the predecessor's golden
//! over two synthetic courses, and no ink badge is earned without a writing course, nor
//! `polyglot_reader` or `bookworm_week` without a configured course.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_habits::badges::{HabitBadgeContext, earned};
use deck_streak_habits::writing::writing_streak;
use deck_streak_kernel::{CourseCode, StudyDay};
use serde_json::{Value, json};

/// A context field as a whole number.
fn number(input: &Value, name: &str) -> u32 {
    u32::try_from(input[name].as_u64().expect("a whole number")).expect("it fits")
}

#[test]
fn the_habit_badge_conditions_match_the_predecessors_golden() {
    let mut keys = std::collections::BTreeSet::new();
    let examined = golden::each_case("habit_badges", |case| {
        let input = &case.input;
        let context = HabitBadgeContext {
            reading_entries: number(input, "reading_entries"),
            writing_entries: number(input, "writing_entries"),
            writing_all_streak: number(input, "writing_all_streak"),
            langs_read_this_week: number(input, "langs_read_this_week"),
            week_total_min: number(input, "week_total_min"),
            all_langs_goal_met: input["all_langs_goal_met"].as_bool().expect("a flag"),
            courses: 2,
        };
        let port = earned(&context);
        keys.extend(port.iter().copied());
        assert_eq!(json!(port), case.output, "{input}");
    });
    assert!(examined.count > 0, "the golden holds cases");
    println!("examined {} badge key(s) earned", keys.len());
    assert_eq!(
        keys.len(),
        8,
        "every habit badge is earned by some case: {keys:?}"
    );
}

#[test]
fn no_course_badge_is_earned_without_its_courses() {
    // A former writing course confirmed on 120 consecutive days, read through no writing course.
    let day = StudyDay::from_epoch_day(20_104);
    let former = CourseCode::new("qaa").expect("a synthetic course code");
    let rows: Vec<(CourseCode, StudyDay)> = (20_104 - 119..=20_104)
        .map(|number| (former, StudyDay::from_epoch_day(number)))
        .collect();
    let streak = writing_streak(&[], &rows, day);
    // No course is configured: the predecessor's `0 >= 0` and `all([])` would both hold.
    let context = HabitBadgeContext {
        writing_entries: 120,
        writing_all_streak: streak,
        all_langs_goal_met: true,
        ..HabitBadgeContext::default()
    };
    let got = earned(&context);
    for key in [
        "ink_week",
        "ink_month",
        "ink_century",
        "polyglot_reader",
        "bookworm_week",
    ] {
        assert!(
            !got.contains(&key),
            "{key} is not earned without its courses: {got:?}"
        );
    }
    assert_eq!(got, ["quill_initiate"], "a confirmation still counts");
}
