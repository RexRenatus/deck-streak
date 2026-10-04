//! The writing habit's rules equal the predecessor's (SPEC-078 A7 to A9; R7, R8): a study day's
//! writing XP and the writing streaks match their goldens over synthetic writing courses, and with
//! no writing course configured no day is all confirmed, so nothing is paid and no streak counts.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_habits::writing::{
    WRITE_ALL_SOURCE, all_confirmed_days, course_streaks, write_source, writing_courses,
    writing_day_xp, writing_streak,
};
use deck_streak_kernel::{CourseCode, Courses, StudyDay};
use serde_json::{Value, json};

/// The study day every writing XP case is read on.
const DAY: i64 = 20_104;

fn code(text: &str) -> CourseCode {
    CourseCode::new(text).expect("a synthetic course code")
}

/// A case's list of codes.
fn codes(value: &Value) -> Vec<CourseCode> {
    value
        .as_array()
        .expect("a list of codes")
        .iter()
        .map(|text| code(text.as_str().expect("a code")))
        .collect()
}

#[test]
fn the_writing_xp_matches_the_predecessors_golden() {
    let mut bonuses = 0;
    let examined = golden::each_case("habit_writing_xp", |case| {
        let writing = codes(&case.input["codes"]);
        let day = StudyDay::from_epoch_day(DAY);
        let rows: Vec<(CourseCode, StudyDay)> = codes(&case.input["done"])
            .into_iter()
            .map(|done| (done, day))
            .collect();
        let xp = writing_day_xp(&writing, day, &rows);
        if xp.all > 0 {
            bonuses += 1;
        }
        let mut grants: Vec<(String, u32)> = xp
            .courses
            .iter()
            .map(|(code, amount)| (write_source(*code), *amount))
            .collect();
        grants.push((WRITE_ALL_SOURCE.to_owned(), xp.all));
        grants.sort();
        let port: Vec<Value> = grants
            .into_iter()
            .map(|(source, amount)| json!([source, amount]))
            .collect();
        assert_eq!(Value::Array(port), case.output, "{}", case.input);
    });
    assert!(examined.count > 0, "the golden holds cases");
    assert!(bonuses > 0, "a case pays the writing bonus");
}

#[test]
fn the_writing_streaks_match_the_predecessors_golden() {
    let mut counted = 0;
    let examined = golden::each_case("habit_writing_streak", |case| {
        let writing = codes(&case.input["codes"]);
        let rows: Vec<(CourseCode, StudyDay)> = case.input["rows"]
            .as_array()
            .expect("the rows")
            .iter()
            .map(|row| {
                (
                    code(row[0].as_str().expect("a code")),
                    StudyDay::from_epoch_day(row[1].as_i64().expect("a day")),
                )
            })
            .collect();
        let today = StudyDay::from_epoch_day(case.input["today"].as_i64().expect("a day"));
        let all = writing_streak(&writing, &rows, today);
        if all > 0 {
            counted += 1;
        }
        let by_course: serde_json::Map<String, Value> = course_streaks(&writing, &rows, today)
            .into_iter()
            .map(|(code, streak)| (code.as_str().to_owned(), json!(streak)))
            .collect();
        let port = json!({"all": all, "by_course": by_course});
        assert_eq!(port, case.output, "{}", case.input);
    });
    assert!(examined.count > 0, "the golden holds cases");
    assert!(counted > 0, "a case counts a writing streak");
}

#[test]
fn no_writing_xp_is_settled_without_a_writing_course() {
    // A planted confirmation of a course that is no longer a writing course, on every day of a
    // week: with no writing course configured, none of those days is all confirmed.
    let day = StudyDay::from_epoch_day(DAY);
    let former = code("qaa");
    let rows: Vec<(CourseCode, StudyDay)> = (DAY - 7..=DAY)
        .map(|number| (former, StudyDay::from_epoch_day(number)))
        .collect();
    assert_eq!(
        all_confirmed_days(&[], &rows),
        std::collections::BTreeSet::new(),
        "no day is all confirmed over no writing course"
    );
    let xp = writing_day_xp(&[], day, &rows);
    assert!(xp.courses.is_empty(), "no course is paid: {xp:?}");
    assert_eq!(xp.all, 0, "no bonus is paid over no writing course");
    assert_eq!(
        writing_streak(&[], &rows, day),
        0,
        "no writing streak counts"
    );
    assert!(
        course_streaks(&[], &rows, day).is_empty(),
        "no course has a streak"
    );
}

#[test]
fn the_writing_courses_are_the_configured_courses_marked_writing() {
    // Three synthetic courses: the first reads only, the other two write, in the file's order.
    let courses = Courses::parse(
        r#"{"schema": "deckstreak.courses.v1", "courses": [
            {"code": "qac", "name": "Course Qac", "flag": "F", "deck_root": "Qac", "alias": "c",
             "writing": true, "unit_bands": {}},
            {"code": "qaa", "name": "Course Qaa", "flag": "F", "deck_root": "Qaa", "alias": "a",
             "writing": false, "unit_bands": {}},
            {"code": "qab", "name": "Course Qab", "flag": "F", "deck_root": "Qab", "alias": "b",
             "writing": true, "unit_bands": {}}
        ], "focus_subjects": []}"#,
    )
    .expect("the synthetic courses parse");
    assert_eq!(
        writing_courses(&courses),
        [code("qac"), code("qab")],
        "the writing courses, in the file's order, without the reading-only course"
    );
    assert!(
        writing_courses(&Courses::default()).is_empty(),
        "no course configured, no writing course"
    );
}
