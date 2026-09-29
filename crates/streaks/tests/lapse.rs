//! The open lapse (SPEC-049 A11 to A13; R12 to R14; ADR-088): three silent study days that are not
//! skip days open a lapse, a skip day neither counts nor ends the run, the id is the run's first
//! unskipped silent day, and a study day closes it. The golden's cases are the predecessor's own
//! answers over synthetic days.

// An integration test is test code: its helpers panic on a malformed golden.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::lapse::{LAPSE_AFTER_SILENT_DAYS, open_lapse};
use serde_json::Value;

/// The economy file, as the repository holds it.
const ECONOMY: &str = include_str!("../../../economy.json");
/// A study day near the present.
const T: i64 = 20_000;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn integer(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

/// One review on each of `studied` days and a zero row on each of `zeros`.
fn counts(studied: &[i64], zeros: &[i64]) -> BTreeMap<StudyDay, u32> {
    let mut rows: BTreeMap<StudyDay, u32> = zeros.iter().map(|&number| (day(number), 0)).collect();
    for &number in studied {
        rows.insert(day(number), 1);
    }
    rows
}

fn skips(numbers: &[i64]) -> BTreeSet<StudyDay> {
    numbers.iter().map(|&number| day(number)).collect()
}

fn open(today: i64, counts: &BTreeMap<StudyDay, u32>, skipped: &[i64]) -> Option<i64> {
    open_lapse(day(today), counts, &skips(skipped), LAPSE_AFTER_SILENT_DAYS)
        .map(StudyDay::epoch_day)
}

#[test]
fn the_open_lapse_matches_the_parity_golden() {
    let examined = golden::each_case("lapse_episode", |case| {
        let rows: BTreeMap<StudyDay, u32> = case.input["review_counts"]
            .as_array()
            .expect("the review counts")
            .iter()
            .map(|row| {
                let count = u32::try_from(integer(&row["count"])).expect("a count");
                (day(integer(&row["day"])), count)
            })
            .collect();
        let skipped: BTreeSet<StudyDay> = case.input["skip_days"]
            .as_array()
            .expect("the skip days")
            .iter()
            .map(|value| day(integer(value)))
            .collect();
        let got = open_lapse(
            day(integer(&case.input["today"])),
            &rows,
            &skipped,
            LAPSE_AFTER_SILENT_DAYS,
        )
        .map(StudyDay::epoch_day);
        assert_eq!(
            got,
            case.output.as_i64(),
            "class {:?}, input {}: the predecessor's lapse differs",
            case.class,
            case.input
        );
    });
    println!("{examined}");
}

#[test]
fn a_skip_day_neither_counts_nor_ends_a_silent_run() {
    // Two silent days do not open a lapse; three do, and the id is the first of them.
    assert_eq!(open(T, &counts(&[T - 30, T - 2], &[]), &[]), None);
    assert_eq!(
        open(T, &counts(&[T - 30, T - 3], &[]), &[]),
        Some(T - 2),
        "three silent days"
    );
    // A skip day inside the run does not count: three silent days, one skipped, are two.
    assert_eq!(open(T, &counts(&[T - 30, T - 3], &[]), &[T - 1]), None);
    // It does not end the run either: the run reaches back past the skip day to its first
    // unskipped day, and the skip day is not the id.
    assert_eq!(
        open(T, &counts(&[T - 30, T - 5], &[]), &[T - 3]),
        Some(T - 4)
    );
    // A skip day at the run's first day passes the id to the next unskipped day.
    assert_eq!(
        open(T, &counts(&[T - 30, T - 4], &[]), &[T - 3]),
        Some(T - 2)
    );
    // A skip day at the run's last day (today) does not count either.
    assert_eq!(open(T, &counts(&[T - 30, T - 3], &[]), &[T]), None);
    assert_eq!(open(T, &counts(&[T - 30, T - 4], &[]), &[T]), Some(T - 3));
}

#[test]
fn one_episode_keeps_one_lapse_id_until_a_study_day_closes_it() {
    // Studied on T-9 and T-5; the silent run starts on T-4.
    let history = counts(&[T - 9, T - 5], &[]);
    assert_eq!(open(T - 4, &history, &[]), None, "one silent day");
    assert_eq!(open(T - 3, &history, &[]), None, "two silent days");
    let ids: Vec<Option<i64>> = (T - 2..=T + 6)
        .map(|today| open(today, &history, &[]))
        .collect();
    assert_eq!(ids[0], Some(T - 4), "the third silent day");
    assert!(
        ids.iter().all(|id| *id == Some(T - 4)),
        "one episode reports one id: {ids:?}"
    );
    // A study day with a review closes the episode, and a row of no reviews does not.
    let closed = counts(&[T - 9, T - 5, T + 1], &[]);
    assert_eq!(open(T + 1, &closed, &[]), None, "the study day closes it");
    assert_eq!(open(T, &closed, &[]), Some(T - 4), "the day before it");
    let unstudied = counts(&[T - 9, T - 5], &[T + 1]);
    assert_eq!(
        open(T + 1, &unstudied, &[]),
        Some(T - 4),
        "a row of no reviews closes nothing"
    );
    // A study day that is also a skip day still closes the run.
    assert_eq!(open(T + 1, &closed, &[T + 1]), None);
    // The next silence is a new episode with its own id.
    assert_eq!(open(T + 4, &closed, &[]), Some(T + 2));
    assert_eq!(open(T + 5, &closed, &[]), Some(T + 2));
}

#[test]
fn an_empty_window_holds_no_lapse() {
    assert_eq!(open(T, &BTreeMap::new(), &[]), None);
}

#[test]
fn the_threshold_is_the_economy_files() {
    let economy: Value = serde_json::from_str(ECONOMY).expect("the economy file parses");
    let named = economy["governor"]["lapse_after_silent_days"]
        .as_u64()
        .expect("a whole number of days");
    assert_eq!(u64::from(LAPSE_AFTER_SILENT_DAYS), named);
}

#[test]
fn the_days_that_open_a_lapse_follow_the_file() {
    // The same run of four silent days, judged by a copy of the file that names 5 and by the
    // constant, which equals the file as it stands.
    let altered = ECONOMY.replace(
        "\"lapse_after_silent_days\": 3",
        "\"lapse_after_silent_days\": 5",
    );
    assert_ne!(altered, ECONOMY, "the copy must differ from the file");
    let economy: Value = serde_json::from_str(&altered).expect("the copy parses");
    let named = u32::try_from(
        economy["governor"]["lapse_after_silent_days"]
            .as_u64()
            .expect("a whole number of days"),
    )
    .expect("a small number");
    let history = counts(&[T - 30, T - 4], &[]);
    let judged = |threshold| open_lapse(day(T), &history, &BTreeSet::new(), threshold);
    assert_eq!(judged(named), None, "four silent days are short of five");
    assert_eq!(judged(LAPSE_AFTER_SILENT_DAYS), Some(day(T - 3)));
}
