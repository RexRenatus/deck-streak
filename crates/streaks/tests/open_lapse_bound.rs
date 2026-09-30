//! The open lapse's bound (SPEC-076 A41; SPEC-049 R12, R13): the walk reads the window's days by a
//! counted range, and answers what the earlier open loop answered, including at the smallest epoch
//! day, where the earlier loop's `checked_sub` answered nothing. Every history is synthetic.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::lapse::open_lapse;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

/// The walk as SPEC-049 first wrote it, a loop on a counter that ends only when the counter's own
/// arithmetic ends it. Kept here as the reference the bounded walk is compared with.
fn reference_open_lapse(
    today: StudyDay,
    review_counts: &BTreeMap<StudyDay, u32>,
    skip_days: &BTreeSet<StudyDay>,
    silent_days_to_open: u32,
) -> Option<StudyDay> {
    let window_start = review_counts.keys().next()?.epoch_day();
    let mut silent: u32 = 0;
    let mut first_silent = None;
    let mut number = today.epoch_day();
    while number >= window_start {
        let here = day(number);
        if review_counts.get(&here).copied().unwrap_or(0) > 0 {
            break;
        }
        if !skip_days.contains(&here) {
            silent = silent.saturating_add(1);
            first_silent = Some(here);
        }
        number = number.checked_sub(1)?;
    }
    if silent >= silent_days_to_open {
        first_silent
    } else {
        None
    }
}

/// A rule (A41): for every window of one to six days at each origin, every assignment of each day
/// to a study review, a zero count, a skip day or no row, every today from two days before the
/// window to two days after it, and every threshold from 0 to 4, the bounded walk answers what the
/// reference walk answers. The origins hold the smallest and largest epoch days a `StudyDay` holds.
#[test]
fn the_bounded_lapse_walk_answers_what_the_reference_walk_answers() {
    let origins = [
        i64::MIN,
        i64::MIN + 1,
        i64::MIN + 3,
        -1,
        0,
        20_000,
        i64::MAX - 8,
    ];
    let mut cases = 0_u64;
    let mut opened = 0_u64;
    let mut at_the_smallest_day = 0_u64;
    for &origin in &origins {
        for width in 1..=6_i64 {
            // Four states per day: 0 no row, 1 a study review, 2 a zero count, 3 a skip day.
            for code in 0..4_u32.pow(u32::try_from(width).expect("small")) {
                let mut counts: BTreeMap<StudyDay, u32> = BTreeMap::new();
                let mut skips: BTreeSet<StudyDay> = BTreeSet::new();
                let mut rest = code;
                for offset in 0..width {
                    let here = day(origin + offset);
                    match rest % 4 {
                        1 => {
                            counts.insert(here, 1);
                        }
                        2 => {
                            counts.insert(here, 0);
                        }
                        3 => {
                            skips.insert(here);
                        }
                        _ => {}
                    }
                    rest /= 4;
                }
                for shift in -2..=width + 1 {
                    let Some(today) = origin.checked_add(shift) else {
                        continue;
                    };
                    for threshold in 0..=4 {
                        let want = reference_open_lapse(day(today), &counts, &skips, threshold);
                        let got = open_lapse(day(today), &counts, &skips, threshold);
                        assert_eq!(
                            got, want,
                            "origin {origin}, width {width}, code {code}, today {today}, \
                             threshold {threshold}"
                        );
                        cases += 1;
                        if got.is_some() {
                            opened += 1;
                        }
                        if origin == i64::MIN {
                            at_the_smallest_day += 1;
                        }
                    }
                }
            }
        }
    }
    println!(
        "open-lapse population: {cases} cases, {opened} opening a lapse, \
         {at_the_smallest_day} at the smallest epoch day"
    );
    assert_eq!(cases, 1_765_680);
    assert!(opened > 100_000 && at_the_smallest_day > 150_000);
}
