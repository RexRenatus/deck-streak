//! The writing day's XP answers every vector the Lean port writes (SPEC-078 A37; #94): for each
//! input in `formal/vectors/habit-writing.jsonl`, written by `formal/lean/Formal/HabitWriting.lean`'s
//! port, `writing_day_xp` gives the same per-course amounts and bonus, `all_confirmed_days` the same
//! days, and the two amounts equal the port's.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_habits::writing::{
    WRITING_XP_ALL_THREE_BONUS, WRITING_XP_PER_DAY, all_confirmed_days, writing_day_xp,
};
use deck_streak_kernel::{CourseCode, StudyDay};
use serde_json::{Value, json};

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/habit-writing.jsonl");

fn codes(vector: &Value) -> Vec<CourseCode> {
    vector["writing"]
        .as_array()
        .expect("the writing set")
        .iter()
        .map(|code| CourseCode::new(code.as_str().expect("a code")).expect("a synthetic code"))
        .collect()
}

fn rows(vector: &Value) -> Vec<(CourseCode, StudyDay)> {
    vector["rows"]
        .as_array()
        .expect("the rows")
        .iter()
        .map(|row| {
            (
                CourseCode::new(row[0].as_str().expect("a code")).expect("a synthetic code"),
                StudyDay::from_epoch_day(row[1].as_i64().expect("a day")),
            )
        })
        .collect()
}

#[test]
fn the_writing_xp_equals_the_lean_ports_vectors() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "HabitWriting", "{header}");
    assert_eq!(header["covers"], "crates/habits/src/writing.rs", "{header}");
    assert_eq!(header["anchor"], "writing_day_xp", "{header}");
    let constants: Value =
        serde_json::from_str(lines.next().expect("the constants line")).expect("a JSON line");
    assert_eq!(constants["rule"], "constants", "{constants}");
    assert_eq!(
        (
            constants["WRITING_XP_PER_DAY"].as_u64(),
            constants["WRITING_XP_ALL_THREE_BONUS"].as_u64()
        ),
        (
            Some(u64::from(WRITING_XP_PER_DAY)),
            Some(u64::from(WRITING_XP_ALL_THREE_BONUS))
        ),
        "the two amounts"
    );
    let (mut days, mut bonuses, mut none) = (0_u64, 0_u64, 0_u64);
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        let writing = codes(&vector);
        let rows = rows(&vector);
        match vector["rule"].as_str() {
            Some("writing_day_xp") => {
                let day = StudyDay::from_epoch_day(vector["day"].as_i64().expect("a day"));
                let xp = writing_day_xp(&writing, day, &rows);
                let courses: Vec<Value> = xp
                    .courses
                    .iter()
                    .map(|(code, amount)| json!([code.as_str(), amount]))
                    .collect();
                assert_eq!(Value::Array(courses), vector["courses"], "{vector}");
                assert_eq!(json!(xp.all), vector["all"], "{vector}");
                if xp.all > 0 {
                    bonuses += 1;
                } else {
                    none += 1;
                }
            }
            Some("all_confirmed_days") => {
                let got: Vec<i64> = all_confirmed_days(&writing, &rows)
                    .into_iter()
                    .map(StudyDay::epoch_day)
                    .collect();
                assert_eq!(json!(got), vector["days"], "{vector}");
                days += 1;
            }
            _ => panic!("an unknown rule in {vector}"),
        }
    }
    println!("examined {bonuses} bonus, {none} no-bonus and {days} day-set vector(s)");
    assert_eq!(
        (bonuses, none, days),
        (9, 11, 20),
        "every input the writer prints"
    );
    assert_eq!(
        header["vectors"].as_u64(),
        Some(1 + bonuses + none + days),
        "{header}"
    );
}
