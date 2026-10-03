//! The silence walk's bound (SPEC-076 A38, A39; R14, R15, R25): the walk reads at most the cap's
//! days back, however the days it reads are laid out. Every history is synthetic.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::constants::SILENCE_WALK_CAP_DAYS;
use deck_streak_streaks::governor::{Silence, silence_walk};

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

/// The walk as SPEC-076 first wrote it, a loop on a counter that ends only when the counter's own
/// arithmetic ends it. Kept here as the reference the bounded walk is compared with.
fn reference_walk(
    today: StudyDay,
    study_days: &BTreeSet<StudyDay>,
    skip_days: &BTreeSet<StudyDay>,
) -> Silence {
    let mut silent: u32 = 0;
    let mut first_silent = today;
    let mut number = today.epoch_day();
    while !study_days.contains(&day(number)) && today.epoch_day() - number <= SILENCE_WALK_CAP_DAYS
    {
        let here = day(number);
        if !skip_days.contains(&here) {
            silent = silent.saturating_add(1);
            first_silent = here;
        }
        number -= 1;
    }
    Silence {
        silent_days: silent,
        first_silent,
        exhausted: !study_days.contains(&day(number)),
    }
}

/// A rule (A38): the bounded walk answers what the reference walk answers. Today at three places
/// (an epoch day of zero, a modern one and one before 1970), the newest study day at every distance
/// from none through the cap and two past it (so the cap's edge, minus one, at and plus one, is
/// each read), with and without an older study day beyond the cap, and five skip layouts.
#[test]
fn the_bounded_walk_answers_what_the_reference_walk_answers() {
    let cap = SILENCE_WALK_CAP_DAYS;
    let mut cases = 0_u32;
    let mut exhausted = 0_u32;
    let mut reached = 0_u32;
    for today in [0_i64, 20_102, -400] {
        for newest in std::iter::once(None).chain((0..=cap + 2).map(Some)) {
            for older in [false, true] {
                for layout in 0..5_i64 {
                    let mut study = BTreeSet::new();
                    if let Some(distance) = newest {
                        study.insert(day(today - distance));
                        if older {
                            study.insert(day(today - distance - cap - 5));
                        }
                    } else if older {
                        study.insert(day(today - cap - 5));
                    }
                    let skips: BTreeSet<StudyDay> = (0..=cap + 3)
                        .filter(|offset| match layout {
                            0 => false,
                            1 => true,
                            2 => offset % 2 == 0,
                            3 => *offset == 0,
                            _ => *offset >= cap - 1,
                        })
                        .map(|offset| day(today - offset))
                        .collect();
                    let expected = reference_walk(day(today), &study, &skips);
                    let got = silence_walk(day(today), &study, &skips);
                    assert_eq!(
                        got, expected,
                        "today {today}, newest {newest:?}, older {older}, layout {layout}"
                    );
                    cases += 1;
                    exhausted += u32::from(got.exhausted);
                    reached += u32::from(!got.exhausted);
                }
            }
        }
    }
    println!("silence-walk population: {cases} cases, {exhausted} exhausted, {reached} reached");
    assert_eq!(cases, 3 * 124 * 2 * 5);
    assert!(exhausted > 50 && reached > 500);
}

/// A rule (A39), from the SPEC's words: with one study day at distance `d` and no skip day, the
/// walk counts `d` silent days back from today (its first silent day is `d - 1` back) and has not
/// run out, up to the cap plus one; beyond it the walk counts the cap plus one days and is
/// exhausted. The cap's edge, minus one, at and plus one, and an empty history, are each checked.
#[test]
fn the_walk_stops_at_the_cap_from_the_specs_words() {
    let cap = SILENCE_WALK_CAP_DAYS;
    let today = 20_102;
    let mut cases = 0_u32;
    let none = BTreeSet::new();
    for distance in 0..=cap + 4 {
        let study = BTreeSet::from([day(today - distance)]);
        let got = silence_walk(day(today), &study, &none);
        let walked = distance.min(cap + 1);
        assert_eq!(i64::from(got.silent_days), walked, "distance {distance}");
        let first = if walked == 0 {
            today
        } else {
            today - walked + 1
        };
        assert_eq!(got.first_silent, day(first), "distance {distance}");
        assert_eq!(got.exhausted, distance > cap + 1, "distance {distance}");
        cases += 1;
    }
    let empty = silence_walk(day(today), &BTreeSet::new(), &none);
    assert_eq!(i64::from(empty.silent_days), cap + 1);
    assert_eq!(empty.first_silent, day(today - cap));
    assert!(empty.exhausted);
    println!("silence-walk cap population: {cases} distances plus the empty history");
    assert_eq!(i64::from(cases), cap + 5);
}
