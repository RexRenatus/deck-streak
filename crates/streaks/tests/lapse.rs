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
    // The same day over a window that begins nine days back holds the run that follows it.
    assert_eq!(open(T, &counts(&[T - 9], &[]), &[]), Some(T - 8));
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

/// SPEC-049 R13, in its own words and by a different route from the walk: the window is the days
/// from the earliest one the caller read up to `today`; the silent run is every day of it after
/// the latest day that holds a qualifying review (or the whole window when none does); the run's
/// days that are not skip days are its silent days; and a lapse is open when there are at least
/// `threshold` of them, with the id the earliest of them. A `today` before the window reads no
/// day. It never calls `open_lapse`.
fn r13_oracle(
    today: i64,
    rows: &BTreeMap<i64, u32>,
    skipped: &BTreeSet<i64>,
    threshold: u32,
) -> Option<i64> {
    let first = *rows.keys().next()?;
    if today < first {
        return None;
    }
    let latest_study = rows
        .range(first..=today)
        .filter(|(_, count)| **count > 0)
        .map(|(number, _)| *number)
        .max();
    let run_start = latest_study.map_or(first, |number| number + 1);
    let silent: Vec<i64> = (run_start..=today)
        .filter(|number| !skipped.contains(number))
        .collect();
    let long_enough = u32::try_from(silent.len()).is_ok_and(|held| held >= threshold);
    if long_enough {
        silent.first().copied()
    } else {
        None
    }
}

/// Judge one member: the walk equals the oracle. Returns the oracle's answer.
fn judge_window(
    today: i64,
    rows: &BTreeMap<i64, u32>,
    skipped: &BTreeSet<i64>,
    threshold: u32,
) -> Option<i64> {
    let counts: BTreeMap<StudyDay, u32> = rows.iter().map(|(&n, &c)| (day(n), c)).collect();
    let skips: BTreeSet<StudyDay> = skipped.iter().map(|&n| day(n)).collect();
    let want = r13_oracle(today, rows, skipped, threshold);
    let got = open_lapse(day(today), &counts, &skips, threshold).map(StudyDay::epoch_day);
    assert_eq!(
        got, want,
        "today {today}, rows {rows:?}, skipped {skipped:?}, threshold {threshold}: the walk differs from R13"
    );
    want
}

#[test]
fn a_silent_run_on_the_windows_first_day_is_read() {
    // The window's only silent run starts on its first day, a row of no reviews.
    assert_eq!(open(T, &counts(&[], &[T - 3]), &[]), Some(T - 3));
    assert_eq!(open(T, &counts(&[], &[T - 1]), &[]), None);
    // One study review at the window's first day is the day the run starts after.
    assert_eq!(open(T, &counts(&[T - 3], &[]), &[]), Some(T - 2));
}

/// The distinct members of the window population: `before` of 1 or 2 makes the second and third
/// fills coincide, so 336 of the 2252 members repeat an earlier one.
const DISTINCT_WINDOW_MEMBERS: usize = 1916;

#[test]
fn the_walk_reads_every_day_of_its_window_and_none_outside_it() {
    let mut examined: u64 = 0;
    let (mut opened, mut closed) = (0_u64, 0_u64);
    let threshold = LAPSE_AFTER_SILENT_DAYS;
    let k = i64::from(threshold);
    // A member is every input the judge reads: `today`, the rows, the skip days and the threshold.
    let mut distinct: BTreeSet<(i64, BTreeMap<i64, u32>, BTreeSet<i64>, u32)> = BTreeSet::new();
    let mut record = |today: i64, rows: &BTreeMap<i64, u32>, skipped: &BTreeSet<i64>, at: u32| {
        let answer = judge_window(today, rows, skipped, at);
        distinct.insert((today, rows.clone(), skipped.clone(), at));
        examined += 1;
        if answer.is_some() {
            opened += 1;
        } else {
            closed += 1;
        }
    };
    // Silent runs of one short of the threshold, the threshold and one more, ending on `today`
    // and starting at every position of a window whose earlier days number 0 to 6.
    for run_len in [k - 1, k, k + 1] {
        for before in 0..=6_i64 {
            let today = T + run_len;
            let run_start = today - run_len + 1;
            let first = run_start - before;
            let run: Vec<i64> = (run_start..=today).collect();
            // The days before the run, three ways: all studied; only the day right before the run
            // studied and the rest rows of no reviews; only that day studied and the rest absent
            // but the window's first day, a row of no reviews.
            let mut fills: Vec<BTreeMap<i64, u32>> = Vec::new();
            if before == 0 {
                // The window's first day is in the run: a row of no reviews there.
                fills.push(BTreeMap::from([(first, 0)]));
                fills.push(run.iter().map(|&n| (n, 0)).collect());
            } else {
                fills.push((first..run_start).map(|n| (n, 2)).collect());
                let mut zeros: BTreeMap<i64, u32> = (first..run_start).map(|n| (n, 0)).collect();
                zeros.insert(run_start - 1, 1);
                fills.push(zeros);
                fills.push(BTreeMap::from([(first, 0), (run_start - 1, 1)]));
            }
            // Every subset of the run's days as skip days: at its edges, inside it, and all of it.
            for mask in 0_u32..(1 << run.len()) {
                let mut skipped: BTreeSet<i64> = run
                    .iter()
                    .enumerate()
                    .filter(|(place, _)| *place != 0 && (mask >> place) & 1 == 1)
                    .map(|(_, &n)| n)
                    .collect();
                for skip_the_closing_day in [false, true] {
                    if skip_the_closing_day && before > 0 {
                        skipped.insert(run_start - 1);
                    }
                    for future_study in [false, true] {
                        for fill in &fills {
                            let mut rows = fill.clone();
                            if future_study {
                                // A day after `today` is outside the walk.
                                rows.insert(today + 1, 3);
                            }
                            record(today, &rows, &skipped, threshold);
                        }
                    }
                    skipped.remove(&(run_start - 1));
                }
            }
        }
    }
    // A `today` before the window, and one long past its only row.
    record(
        T,
        &BTreeMap::from([(T + 2, 0)]),
        &BTreeSet::new(),
        threshold,
    );
    record(
        T,
        &BTreeMap::from([(T + 2, 1)]),
        &BTreeSet::new(),
        threshold,
    );
    record(
        T + 40,
        &BTreeMap::from([(T, 0)]),
        &BTreeSet::new(),
        threshold,
    );
    record(
        T + 40,
        &BTreeMap::from([(T, 1)]),
        &BTreeSet::new(),
        threshold,
    );
    // A threshold of one and of five over the same windows.
    for other in [1_u32, 5] {
        for before in 0..=3_i64 {
            let rows: BTreeMap<i64, u32> = (T - before..T)
                .map(|n| (n, 1))
                .chain([(T - before, 0)])
                .collect();
            record(T, &rows, &BTreeSet::new(), other);
        }
    }
    // `record` borrows the set until its last use above.
    println!(
        "examined {examined} window member(s), {} distinct",
        distinct.len()
    );
    assert_eq!(
        examined, 2252,
        "the population is generated, not listed: {examined}"
    );
    assert_eq!(
        distinct.len(),
        DISTINCT_WINDOW_MEMBERS,
        "the population's spread: {} distinct of {examined}",
        distinct.len()
    );
    assert!(
        opened > 0 && closed > 0,
        "both answers occur: {opened} open, {closed} none"
    );
}
