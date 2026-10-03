//! The minutes log's rules equal the predecessor's (SPEC-078 A1, A2, A4c; R2 to R4): a day's
//! reading XP, a course token's course, an entry's bounds and note, and the study week's first day.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_habits::minutes::{
    EntryRefusal, entry, is_week_start, reading_xp, resolve_course, week_end, week_start,
};
use deck_streak_kernel::{Courses, StudyDay};

/// Two synthetic courses: `qaa` (alias `a`) and `qab` (alias `b`).
const COURSES: &str = r#"{
  "schema": "deckstreak.courses.v1",
  "courses": [
    {"code": "qaa", "name": "Course Qaa", "flag": "F", "deck_root": "Qaa", "alias": "a",
     "writing": false, "unit_bands": {}},
    {"code": "qab", "name": "Course Qab", "flag": "F", "deck_root": "Qab", "alias": "b",
     "writing": true, "unit_bands": {}}
  ],
  "focus_subjects": []
}"#;

fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

#[test]
fn the_minutes_xp_matches_the_predecessors_golden() {
    let mut above_the_cap = 0;
    let examined = golden::each_case("habit_reading_xp", |case| {
        let minutes = case.input["minutes_today"].as_u64().expect("whole minutes");
        if minutes > 120 {
            above_the_cap += 1;
        }
        let minutes = u32::try_from(minutes).expect("minutes fit");
        assert_eq!(
            u64::from(reading_xp(minutes)),
            case.output.as_u64().expect("whole XP"),
            "{:?}",
            case.input
        );
    });
    assert!(examined.count > 0, "the golden holds cases");
    assert!(above_the_cap > 0, "a case above 120 minutes is in the golden");
}

#[test]
fn an_entry_resolves_its_course_and_refuses_minutes_outside_the_bounds() {
    let courses = courses();
    let examined = golden::each_case("habit_course_token", |case| {
        let token = case.input["token"].as_str().expect("a token");
        let resolved = resolve_course(&courses, token).map(|code| code.as_str().to_owned());
        assert_eq!(resolved.as_deref(), case.output.as_str(), "{token:?}");
    });
    assert!(examined.count > 0, "the golden holds cases");

    for refused in [0, 601, -1] {
        assert_eq!(
            entry(&courses, "qaa", refused, ""),
            Err(EntryRefusal::Minutes),
            "{refused} minutes"
        );
    }
    for accepted in [1, 600] {
        let logged = entry(&courses, "a", accepted, "").expect("in bounds");
        assert_eq!(logged.code.as_str(), "qaa");
        assert_eq!(i64::from(logged.minutes), accepted);
    }
    assert_eq!(
        entry(&courses, "qzz", 0, ""),
        Err(EntryRefusal::UnknownCourse),
        "the course is checked before the minutes"
    );
    let long = "n".repeat(201);
    let kept = entry(&courses, "qab", 30, &long).expect("a long note is cut, not refused");
    assert_eq!(kept.note.chars().count(), 200, "a note of 201 is cut to 200");
    let wide = "\u{e9}".repeat(201);
    let kept = entry(&courses, "qab", 30, &wide).expect("cut by characters");
    assert_eq!(kept.note, "\u{e9}".repeat(200), "cut by characters, never bytes");
}

#[test]
fn a_study_days_week_starts_on_its_monday() {
    // Epoch day 0 was a Thursday: its week began on day -3, a Monday.
    for (day, first) in [
        (0, -3),
        (3, -3),
        (4, 4),
        (10, 4),
        (-3, -3),
        (-4, -10),
        (-10, -10),
        (20_101, 20_101),
        (20_107, 20_101),
        (20_108, 20_108),
    ] {
        assert_eq!(
            week_start(StudyDay::from_epoch_day(day)),
            StudyDay::from_epoch_day(first),
            "day {day}"
        );
    }
    assert_eq!(
        week_end(StudyDay::from_epoch_day(20_103)),
        StudyDay::from_epoch_day(20_107)
    );
    assert!(is_week_start(StudyDay::from_epoch_day(20_101)));
    assert!(!is_week_start(StudyDay::from_epoch_day(20_102)));
}
