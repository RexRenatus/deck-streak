//! The open lapse walk against its Lean port (#472): `formal/vectors/open-lapse.jsonl` holds the
//! answer of `lean/OpenLapse`'s port for every input of a population, and this test answers each
//! one with the Rust function. The population is derived here from the same axes the Lean writer
//! reads, never read back from the file, so a vector the writer dropped or added is red. Every
//! history is synthetic.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::lapse::open_lapse;
use serde_json::Value;

/// One day of a window, as the Lean writer's axes name it.
#[derive(Clone, Copy)]
enum DayState {
    /// No entry and no skip day.
    Absent,
    /// An entry with one study review.
    Review,
    /// An entry with a count of zero.
    Zero,
    /// No entry, on a skip day.
    Skip,
    /// An entry with a count of zero, on a skip day.
    ZeroSkip,
}

/// The window's first day holds an entry.
const FIRST_STATES: [DayState; 3] = [DayState::Review, DayState::Zero, DayState::ZeroSkip];
/// A day between the first and the last.
const MIDDLE_STATES: [DayState; 4] = [
    DayState::Absent,
    DayState::Review,
    DayState::Zero,
    DayState::Skip,
];
/// The last day of a window wider than one is never a day with nothing.
const LAST_STATES: [DayState; 3] = [DayState::Review, DayState::Zero, DayState::Skip];
/// The window's first days: the smallest day, the day after it, day zero, and the fourth day
/// before the largest.
const ORIGINS: [i64; 4] = [i64::MIN, i64::MIN + 1, 0, i64::MAX - 3];
/// The thresholds.
const THRESHOLDS: [u32; 2] = [1, 3];
/// The window's widths.
const WIDTHS: [usize; 3] = [1, 2, 3];

/// One input of the population.
#[derive(Debug, PartialEq, Eq)]
struct Input {
    today: i64,
    counts: Vec<(i64, u32)>,
    skips: Vec<i64>,
    threshold: u32,
}

/// Every window of a width, the first day's state outermost and the last day's innermost.
fn windows(width: usize) -> Vec<Vec<DayState>> {
    let mut tails: Vec<Vec<DayState>> = LAST_STATES.iter().map(|state| vec![*state]).collect();
    for _ in 2..width {
        tails = MIDDLE_STATES
            .iter()
            .flat_map(|state| {
                tails.iter().map(move |rest| {
                    let mut days = vec![*state];
                    days.extend(rest.iter().copied());
                    days
                })
            })
            .collect();
    }
    FIRST_STATES
        .iter()
        .flat_map(|first| {
            let tails = if width == 1 {
                vec![Vec::new()]
            } else {
                tails.clone()
            };
            tails.into_iter().map(move |rest| {
                let mut days = vec![*first];
                days.extend(rest);
                days
            })
        })
        .collect()
}

/// The day `offset` days after `origin`.
fn day_after(origin: i64, offset: usize) -> i64 {
    origin
        .checked_add(i64::try_from(offset).expect("an offset of at most three"))
        .expect("every window ends within the day type")
}

/// The population, in the order the Lean writer prints it.
fn population() -> Vec<Input> {
    let mut inputs = Vec::new();
    for origin in ORIGINS {
        for threshold in THRESHOLDS {
            for width in WIDTHS {
                for states in windows(width) {
                    let mut counts = Vec::new();
                    let mut skips = Vec::new();
                    for (offset, state) in states.iter().enumerate() {
                        let here = day_after(origin, offset);
                        match state {
                            DayState::Review => counts.push((here, 1)),
                            DayState::Zero => counts.push((here, 0)),
                            DayState::ZeroSkip => {
                                counts.push((here, 0));
                                skips.push(here);
                            }
                            DayState::Skip => skips.push(here),
                            DayState::Absent => {}
                        }
                    }
                    for step in 0..width + 2 {
                        let today = i128::from(origin) + i128::try_from(step).expect("a step") - 1;
                        let Ok(today) = i64::try_from(today) else {
                            continue;
                        };
                        inputs.push(Input {
                            today,
                            counts: counts.clone(),
                            skips: skips.clone(),
                            threshold,
                        });
                    }
                }
            }
        }
    }
    inputs
}

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

/// The Rust function's answer for one input.
fn rust_answer(input: &Input) -> Option<i64> {
    let review_counts: BTreeMap<StudyDay, u32> = input
        .counts
        .iter()
        .map(|(number, count)| (day(*number), *count))
        .collect();
    let skip_days: BTreeSet<StudyDay> = input.skips.iter().map(|number| day(*number)).collect();
    open_lapse(
        day(input.today),
        &review_counts,
        &skip_days,
        input.threshold,
    )
    .map(StudyDay::epoch_day)
}

fn int(value: &Value, what: &str) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("{what} is not an integer: {value}"))
}

/// The committed vectors: the header, then each input with the port's answer.
fn vectors() -> (Value, Vec<(Input, Value)>) {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../formal/vectors/open-lapse.jsonl");
    let text = std::fs::read_to_string(&path).expect("the vectors file is committed");
    let mut lines = text.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("the header is JSON");
    let rows = lines
        .map(|line| {
            let row: Value = serde_json::from_str(line).expect("each vector is JSON");
            let counts = row["counts"]
                .as_array()
                .expect("counts")
                .iter()
                .map(|pair| {
                    let count = u32::try_from(int(&pair[1], "a count")).expect("a u32 count");
                    (int(&pair[0], "a day"), count)
                })
                .collect();
            let skips = row["skips"]
                .as_array()
                .expect("skips")
                .iter()
                .map(|skip| int(skip, "a skip day"))
                .collect();
            let threshold =
                u32::try_from(int(&row["threshold"], "the threshold")).expect("a u32 threshold");
            let input = Input {
                today: int(&row["today"], "today"),
                counts,
                skips,
                threshold,
            };
            (input, row["answer"].clone())
        })
        .collect();
    (header, rows)
}

/// The port's answer as the Rust function's type; a walk that did not answer is refused.
fn port_answer(answer: &Value) -> Option<i64> {
    if answer.is_null() {
        None
    } else {
        Some(int(answer, "the port's answer"))
    }
}

/// The run the rule reads: from the day after the latest study review up to today, or from the
/// window's first day, to today; and its silent days.
fn run_of(input: &Input) -> Option<(i64, u32)> {
    let start = input.counts.first()?.0;
    let latest_review = input
        .counts
        .iter()
        .filter(|(number, count)| *count > 0 && *number <= input.today)
        .map(|(number, _)| *number)
        .max();
    let earliest = latest_review.map_or(start, |number| number + 1);
    let silent = (earliest..=input.today)
        .filter(|number| !input.skips.contains(number))
        .count();
    Some((earliest, u32::try_from(silent).expect("a short run")))
}

#[test]
fn the_open_lapse_walk_answers_every_vector_its_lean_port_wrote() {
    let (header, rows) = vectors();
    let derived = population();
    assert!(!derived.is_empty(), "the axes derive no input");
    assert_eq!(header["entry"], "OpenLapse");
    assert_eq!(header["covers"], "crates/streaks/src/lapse.rs");
    assert_eq!(header["anchor"], "open_lapse");
    assert_eq!(
        usize::try_from(int(&header["vectors"], "the header's count")).expect("a count"),
        rows.len(),
        "the header counts the lines that follow it"
    );
    assert_eq!(
        rows.len(),
        derived.len(),
        "the file holds {} vectors and the axes derive {}",
        rows.len(),
        derived.len()
    );
    let mut differences = Vec::new();
    for (index, ((input, answer), expected_input)) in rows.iter().zip(&derived).enumerate() {
        assert_eq!(
            input, expected_input,
            "vector {index} is not the derived input"
        );
        let rust = rust_answer(input);
        let port = port_answer(answer);
        if rust != port {
            differences.push(format!(
                "vector {index}: {input:?}: the port answers {port:?}, the Rust function {rust:?}"
            ));
        }
    }
    println!("examined {} vector(s)", rows.len());
    assert!(
        differences.is_empty(),
        "{} of {} vector(s) differ; the first: {}",
        differences.len(),
        rows.len(),
        differences
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("; ")
    );
}

#[test]
fn the_vectors_hold_every_case_the_property_names() {
    let derived = population();
    let mut cases: BTreeMap<&str, usize> = BTreeMap::new();
    for input in &derived {
        let start = input
            .counts
            .first()
            .expect("every window has a first day")
            .0;
        let mut mark = |case| *cases.entry(case).or_insert(0) += 1;
        if input.today == i64::MIN {
            mark("today is the smallest day");
        }
        if start == i64::MIN {
            mark("the window starts at the smallest day");
        }
        if input.counts.first().is_some_and(|(_, count)| *count > 0) {
            mark("a study review on the window's first day");
        }
        let Some((earliest, silent)) = run_of(input) else {
            continue;
        };
        if earliest > input.today {
            continue;
        }
        if input.skips.contains(&earliest) {
            mark("a skip day at a run's earliest end");
        }
        if input.skips.contains(&input.today) {
            mark("a skip day at a run's latest end");
        }
        if input
            .skips
            .iter()
            .any(|skip| earliest < *skip && *skip < input.today)
        {
            mark("a skip day inside a run");
        }
        match silent.cmp(&input.threshold) {
            std::cmp::Ordering::Less => mark("a run below the threshold"),
            std::cmp::Ordering::Equal => mark("a run at the threshold"),
            std::cmp::Ordering::Greater => mark("a run above the threshold"),
        }
    }
    for (case, count) in &cases {
        println!("{case}: {count} vector(s)");
    }
    let named = [
        "today is the smallest day",
        "the window starts at the smallest day",
        "a study review on the window's first day",
        "a skip day at a run's earliest end",
        "a skip day at a run's latest end",
        "a skip day inside a run",
        "a run below the threshold",
        "a run at the threshold",
        "a run above the threshold",
    ];
    println!("examined {} vector(s)", derived.len());
    for case in named {
        assert!(
            cases.get(case).is_some_and(|count| *count > 0),
            "no vector holds {case}"
        );
    }
}

#[test]
fn a_walk_that_reaches_the_smallest_day_without_a_review_answers_none_in_the_vectors() {
    let silent_smallest_day = Input {
        today: i64::MIN,
        counts: vec![(i64::MIN, 0)],
        skips: Vec::new(),
        threshold: 1,
    };
    assert_eq!(rust_answer(&silent_smallest_day), None);
    let (_, rows) = vectors();
    let answer = rows
        .iter()
        .find(|(input, _)| *input == silent_smallest_day)
        .map(|(_, answer)| answer.clone())
        .expect("the vectors hold the smallest day with no review");
    assert_eq!(
        port_answer(&answer),
        None,
        "the port answers none there too"
    );
}
