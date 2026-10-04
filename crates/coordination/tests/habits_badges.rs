//! The habit badges step (SPEC-078 A19; R9, R17 as amended; ADR-078): for every case of the
//! predecessor's `evaluate_habit_badges` golden, the logs that make the case's context are written,
//! the fold's habit badges step evaluates the day twice and the offers run twice: each badge the
//! golden names is awarded once, for the evaluated day, and celebrated once, and no other badge
//! is. Every course, entry, confirmation and instant is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_coordination::recompute::Evaluation;
use deck_streak_coordination::recompute::badges::offer_badges;
use deck_streak_coordination::recompute::habit_badges::HabitBadgesStep;
use deck_streak_kernel::{Courses, Db, UtcMillis};
use deck_streak_progression::badges::catalog::catalog;
use serde_json::Value;

use support::{Recorder, at, badges, collection, day, run_step, scratch};

/// A Monday: the first day of the evaluated day's study week.
const MONDAY: i64 = 20_101;
/// The evaluated day, the Sunday that ends [`MONDAY`]'s week.
const SUNDAY: i64 = 20_107;
/// The most minutes one entry holds.
const ENTRY_MAX: i64 = 600;

/// Two synthetic courses, both read, as the golden's two reading courses: `qaa` reads only, `qab`
/// is the one writing course.
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

/// A golden's cases, read whole, as input, output and class.
fn cases(name: &str) -> Vec<(Value, Value, Option<String>)> {
    let mut cases = Vec::new();
    let examined = golden::each_case(name, |case| {
        cases.push((case.input.clone(), case.output.clone(), case.class.clone()));
    });
    println!("examined {} case(s) of {name}", examined.count);
    assert!(examined.count > 0, "the golden holds cases");
    cases
}

fn number(input: &Value, key: &str) -> i64 {
    input[key].as_i64().expect("a number")
}

/// The week's reading entries that make a context's courses read, minutes and goal, on the
/// evaluated day: one course carries every minute; both at the goal split them evenly; both short
/// of it leave one minute to the second course.
fn week_entries(langs: i64, minutes: i64, goal: bool) -> Vec<(&'static str, i64)> {
    let mut entries = Vec::new();
    let mut carry = |code: &'static str, mut minutes: i64| {
        while minutes > 0 {
            let entry = minutes.min(ENTRY_MAX);
            entries.push((code, entry));
            minutes -= entry;
        }
    };
    match (langs, goal) {
        (0, _) => {}
        (1, _) => carry("qaa", minutes),
        (_, true) => {
            carry("qaa", minutes - minutes / 2);
            carry("qab", minutes / 2);
        }
        (_, false) => {
            carry("qaa", minutes - 1);
            carry("qab", 1);
        }
    }
    entries
}

/// Writes the logs that make `input`'s context on [`SUNDAY`]: the week's entries on the day, the
/// rest before the week, the streak's confirmations on the days that end on it, and the rest
/// after a gap that ends the streak.
async fn write_logs(db: &Db, input: &Value) {
    let reading = number(input, "reading_entries");
    let langs = number(input, "langs_read_this_week");
    let minutes = number(input, "week_total_min");
    let goal = input["all_langs_goal_met"].as_bool().expect("a flag");
    let writing = number(input, "writing_entries");
    let streak = number(input, "writing_all_streak");

    let week = week_entries(langs, minutes, goal);
    let before = reading - i64::try_from(week.len()).expect("a count");
    assert!(before >= 0, "the case's entries hold its week: {input}");
    let mut rows: Vec<(&str, i64, i64)> = week
        .into_iter()
        .map(|(code, minutes)| (code, SUNDAY, minutes))
        .collect();
    rows.extend((0..before).map(|index| ("qaa", MONDAY - 1 - index, 10)));

    // A streak of none ends neither on the day nor the day before.
    let gap = if streak == 0 {
        SUNDAY - 1
    } else {
        SUNDAY - streak
    };
    let mut confirmed: Vec<i64> = (0..streak).map(|index| SUNDAY - index).collect();
    confirmed.extend((0..writing - streak).map(|index| gap - 1 - index));

    let mut write = db.write().await.expect("a write");
    for (code, study_day, minutes) in rows {
        sqlx::query(
            "INSERT INTO minutes_log (code, study_day, minutes, note, created_at) \
             VALUES (?1, ?2, ?3, '', 1000)",
        )
        .bind(code)
        .bind(study_day)
        .bind(minutes)
        .execute(&mut *write)
        .await
        .expect("an entry");
    }
    for study_day in confirmed {
        sqlx::query(
            "INSERT INTO writing_log (code, study_day, created_at) VALUES ('qab', ?1, 1000)",
        )
        .bind(study_day)
        .execute(&mut *write)
        .await
        .expect("a confirmation");
    }
    write.commit().await.expect("the logs commit");
}

#[tokio::test]
async fn habit_badges_are_awarded_once_as_the_predecessors_golden() {
    let courses = courses();
    let catalog = catalog(&courses);
    let now = at(SUNDAY, 12);
    for (input, output, class) in cases("habit_badges") {
        let scratch = scratch().await;
        write_logs(&scratch.db, &input).await;
        let step = HabitBadgesStep::new(courses.clone());
        let data = collection(Vec::new(), Vec::new());
        for _ in 0..2 {
            run_step(
                &scratch.db,
                &step,
                &data,
                0,
                (SUNDAY, Evaluation::Current),
                now,
            )
            .await;
        }
        let recorder = Recorder::default();
        for _ in 0..2 {
            offer_badges(
                &recorder,
                &scratch.db,
                UtcMillis::from_epoch_millis(now),
                day(SUNDAY),
            )
            .await
            .expect("the offers run");
        }

        let keys: Vec<&str> = output
            .as_array()
            .expect("the golden's keys")
            .iter()
            .map(|key| key.as_str().expect("a key"))
            .collect();
        let mut expected: Vec<(String, i64, i64, bool)> = keys
            .iter()
            .map(|key| {
                let badge = catalog
                    .iter()
                    .find(|badge| badge.key == *key)
                    .expect("the catalog holds every habit key");
                ((*key).to_owned(), i64::from(badge.tier), SUNDAY, true)
            })
            .collect();
        expected.sort();
        assert_eq!(
            badges(&scratch.db).await,
            expected,
            "each badge of {class:?} {input} is awarded once, for the evaluated day, and marked"
        );
        let mut celebrated = recorder.keys();
        celebrated.sort();
        let mut owed: Vec<String> = expected
            .iter()
            .map(|(key, tier, _, _)| format!("badge:{key}:{tier}"))
            .collect();
        owed.sort();
        assert_eq!(
            celebrated, owed,
            "each badge of {class:?} {input} is celebrated once"
        );
    }
}
