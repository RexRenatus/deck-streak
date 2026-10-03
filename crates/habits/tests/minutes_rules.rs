//! The minutes log's choices equal the predecessor's rules (SPEC-078 R3, R4): the weekly bonus is
//! earned from 210 minutes up, a minutes-only entry goes to the first course holding the most
//! minutes this week, else of all time, and each course's XP sources are named for it. Every
//! expected value is spelt literally. Every course is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_habits::minutes::{goal_bonus, goal_source, most_used, read_source};
use deck_streak_kernel::CourseCode;

fn minutes(rows: &[(&str, u32)]) -> Vec<(String, u32)> {
    rows.iter()
        .map(|(code, minutes)| ((*code).to_owned(), *minutes))
        .collect()
}

#[test]
fn the_weekly_bonus_is_earned_from_the_goal_up() {
    let bonuses: Vec<(u32, u32)> = [0, 1, 209, 210, 211, 600]
        .into_iter()
        .map(|week| (week, goal_bonus(week)))
        .collect();
    assert_eq!(
        bonuses,
        [(0, 0), (1, 0), (209, 0), (210, 150), (211, 150), (600, 150)],
        "150 from 210 minutes up, nothing below"
    );
}

#[test]
fn the_most_used_course_is_the_first_holding_the_most_minutes() {
    let cases: [(&[(&str, u32)], &[(&str, u32)], Option<&str>, &str); 6] = [
        (
            &[("qaa", 30), ("qab", 45)],
            &[],
            Some("qab"),
            "the week's most minutes",
        ),
        (
            &[("qaa", 45), ("qab", 45)],
            &[("qab", 90)],
            Some("qaa"),
            "the first of a tie in the week, never the later one",
        ),
        (
            &[("qaa", 0)],
            &[("qab", 10)],
            Some("qab"),
            "a week of no minutes falls back to all time",
        ),
        (
            &[],
            &[("qab", 10), ("qaa", 30)],
            Some("qaa"),
            "all time's most minutes when the week has none",
        ),
        (
            &[],
            &[("qab", 30), ("qaa", 30)],
            Some("qab"),
            "the first of a tie in all time",
        ),
        (&[], &[], None, "no course holds a minute"),
    ];
    for (week, all_time, expected, why) in cases {
        assert_eq!(
            most_used(&minutes(week), &minutes(all_time)).as_deref(),
            expected,
            "{why}"
        );
    }
}

#[test]
fn each_courses_xp_sources_are_named_for_it() {
    let code = CourseCode::new("qaa").expect("a synthetic course code");
    assert_eq!(read_source(code), "read:qaa");
    assert_eq!(goal_source(code), "readgoal:qaa");
}
