//! Populations for the streak rules (SPEC-076 A26 to A31): each test generates every case of one
//! rule's input space at test time and compares the code with an oracle written from the SPEC's
//! words, never by calling the function under test. The count of each population is printed and
//! asserted, so a shrinking population fails.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::constants::STREAK_HEAT;
use deck_streak_streaks::freeze::{FreezeEvent, FreezeReason, drops_in_month, freeze_events_for};
use deck_streak_streaks::governor::silence_walk;
use deck_streak_streaks::lapse::{LAPSE_AFTER_SILENT_DAYS, anchor_beyond_the_walk};
use deck_streak_streaks::replay;
use deck_streak_streaks::streak::{StreakState, Transition, heat_tier};
use deck_streak_streaks::strength::persists;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn set(numbers: impl IntoIterator<Item = i64>) -> BTreeSet<StudyDay> {
    numbers.into_iter().map(day).collect()
}

/// Days in each month of `year` in the proleptic Gregorian calendar (R8's month).
fn month_length(year: i64, month: i64) -> i64 {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A rule (A26): the drop cap's month is the calendar month. Every day from 1600 to 2102 (three
/// 400-year era edges, the leap days of 1600, 1904 to 2096 and 2000, and the non-leap 1700, 1800,
/// 1900 and 2100) is paired with 140 probes (and every ninth day with more, out to 400 days and around one era), and the answer is compared with a calendar walked
/// one day at a time from 1970-01-01.
#[test]
fn every_day_falls_in_the_calendar_month_the_cap_reads() {
    const LOW: i64 = -135_000; // 1600-05
    const HIGH: i64 = 48_300; // 2102-03
    let mut months: Vec<(i64, i64)> = Vec::new(); // index = epoch day - LOW
    let mut back: Vec<(i64, i64)> = Vec::new(); // epoch days -1, -2, ...
    let (mut year, mut month, mut dom) = (1969, 12, 31);
    for _ in 0..(-LOW) {
        back.push((year, month));
        if dom > 1 {
            dom -= 1;
        } else {
            month -= 1;
            if month == 0 {
                month = 12;
                year -= 1;
            }
            dom = month_length(year, month);
        }
    }
    months.extend(back.into_iter().rev());
    let (mut year, mut month, mut dom) = (1970, 1, 1);
    for _ in 0..=HIGH {
        months.push((year, month));
        if dom < month_length(year, month) {
            dom += 1;
        } else {
            dom = 1;
            month += 1;
            if month == 13 {
                month = 1;
                year += 1;
            }
        }
    }
    let at = |number: i64| months[usize::try_from(number - LOW).expect("in range")];
    let mut probes: Vec<i64> = (1..=62).flat_map(|k| [k, -k]).collect();
    for k in [365, 366, 730, 731, 1_461, 3_652, 36_524, 146_097] {
        probes.extend([k, -k]);
    }
    assert_eq!(probes.len(), 140);
    // Month-scale offsets, from a little over two months to more than a year, on every ninth day.
    let mut far: Vec<i64> = (63..=400).flat_map(|k| [k, -k]).collect();
    // The same month of the year before and the year after one 400-year era, a month either side.
    for year_days in [365, 366] {
        for step in 0..=62 {
            let k = 146_097 - year_days + step - 31;
            far.extend([k, -k, k + 2 * year_days, -(k + 2 * year_days)]);
        }
    }
    let mut pairs = 0_u64;
    for number in LOW..=HIGH {
        let event_day = |k: i64| {
            [FreezeEvent {
                day: day(number + k),
                delta: 1,
                reason: FreezeReason::Chest,
            }]
        };
        let far_now: &[i64] = if number.rem_euclid(9) == 0 { &far } else { &[] };
        for &k in probes.iter().chain(far_now) {
            if number + k < LOW || number + k > HIGH {
                continue;
            }
            let expected = u32::from(at(number) == at(number + k));
            assert_eq!(
                drops_in_month(&event_day(k), day(number)),
                expected,
                "day {number} against day {}",
                number + k
            );
            pairs += 1;
        }
    }
    println!("civil-month population: {pairs} day pairs");
    assert!(pairs > 38_000_000, "the population shrank: {pairs}");
}

/// A rule (A27): a transition writes a consumed event when a freeze was spent, a break marker when
/// the run broke, and a streak-earn event of the freezes gained (net of the one spent) when that is
/// positive; the freezes are compared over 0 to 4 before and after, both flags, and the u32 extreme.
#[test]
fn the_freeze_events_follow_the_flags_and_the_freezes_gained() {
    let today = day(20_000);
    let mut cases = 0_u32;
    let mut with_events = 0_u32;
    let counts: Vec<u32> = (0..=4).chain([u32::MAX]).collect();
    for &before in &counts {
        for &after in &counts {
            for froze in [false, true] {
                for broke in [false, true] {
                    let prev = StreakState {
                        freezes: before,
                        ..StreakState::start()
                    };
                    let next = Transition {
                        state: StreakState {
                            freezes: after,
                            ..StreakState::start()
                        },
                        froze_today: froze,
                        broke_today: broke,
                    };
                    let mut expected = Vec::new();
                    if froze {
                        expected.push((-1, FreezeReason::Consumed));
                    }
                    if broke {
                        expected.push((0, FreezeReason::StreakBreak));
                    }
                    let gained = i64::from(after) - i64::from(before) + i64::from(froze);
                    if gained > 0 {
                        let delta = i32::try_from(gained).unwrap_or(i32::MAX);
                        expected.push((delta, FreezeReason::StreakEarn));
                    }
                    let expected: Vec<FreezeEvent> = expected
                        .into_iter()
                        .map(|(delta, reason)| FreezeEvent {
                            day: today,
                            delta,
                            reason,
                        })
                        .collect();
                    with_events += u32::from(!expected.is_empty());
                    assert_eq!(
                        freeze_events_for(&prev, &next, today),
                        expected,
                        "freezes {before} to {after}, froze {froze}, broke {broke}"
                    );
                    cases += 1;
                }
            }
        }
    }
    println!("freeze-events population: {cases} cases, {with_events} writing events");
    assert_eq!(cases, 144);
    assert!(with_events > 100);
}

/// The oracle's run of law days after each day of the window, by the rule's words: a study day
/// adds one, a skip day changes nothing, any other day ends the run.
fn law_oracle(
    days: &BTreeSet<i64>,
    skips: &BTreeSet<i64>,
    through: i64,
) -> (u32, u32, Option<i64>) {
    let Some(&first) = days.iter().next() else {
        return (0, 0, None);
    };
    let mut run = 0_u32;
    let mut best = 0_u32;
    let mut before_through = 0_u32;
    for number in first..=through {
        if number == through {
            before_through = run;
        }
        if days.contains(&number) {
            run += 1;
            best = best.max(run);
        } else if !skips.contains(&number) {
            run = 0;
        }
    }
    let current = if through < first {
        0
    } else if days.contains(&through) {
        run
    } else {
        // Today has no study yet: the run ending yesterday still stands, if today is not a break.
        before_through
    };
    let last = days.range(..=through).next_back().copied();
    (current, best.max(current), last)
}

/// A rule (A28): the law row after a day holds the run ending that day (or yesterday while today
/// is unstudied), skip days bridging it, and the longest run any day held. Every assignment of
/// study, skip or empty to each of eight days, and every `through` from before the window to past
/// it.
#[test]
fn the_law_row_follows_the_run_over_every_assignment_of_eight_days() {
    const BASE: i64 = 20_000;
    let mut cases = 0_u32;
    for code in 0..3_u32.pow(8) {
        let (mut days, mut skips) = (BTreeSet::new(), BTreeSet::new());
        let mut rest = code;
        for offset in 0..8 {
            match rest % 3 {
                1 => days.insert(BASE + offset),
                2 => skips.insert(BASE + offset),
                _ => false,
            };
            rest /= 3;
        }
        for through in BASE - 1..=BASE + 10 {
            let row = replay::law(&set(days.clone()), &set(skips.clone()), day(through));
            let (current, longest, last) = law_oracle(&days, &skips, through);
            assert_eq!(
                (
                    row.current,
                    row.longest,
                    row.last_study_day.map(StudyDay::epoch_day)
                ),
                (current, longest, last),
                "days {days:?}, skips {skips:?}, through {through}"
            );
            assert_eq!((row.freezes, row.comeback_armed), (1, false));
            cases += 1;
        }
    }
    println!("law population: {cases} cases");
    assert_eq!(cases, 6_561 * 12);
}

/// The language track after each day, restated from R1 to R6: no skip days, one freeze at the start.
#[derive(Clone, Copy)]
struct Track {
    current: u32,
    longest: u32,
    freezes: u32,
    last: Option<i64>,
    armed: bool,
}

/// One day of the language rules. Returns the events (delta, reason) the day wrote.
fn step(track: &mut Track, number: i64, studied: bool) -> Vec<(i32, FreezeReason)> {
    let before = *track;
    let mut froze = false;
    let mut broke = false;
    if studied {
        match before.last {
            None => track.current = 1,
            Some(last) => {
                let missed = number - last - 1;
                if missed == 0 {
                    track.current += 1;
                } else if missed == 1 && before.freezes > 0 {
                    froze = true;
                    track.freezes -= 1;
                    track.current += 1;
                } else {
                    if before.current > 0 {
                        broke = true;
                        track.armed |= before.current >= 7;
                    }
                    track.current = 1;
                }
            }
        }
        if !broke && track.current.is_multiple_of(7) {
            track.freezes += 1;
        }
        track.freezes = track.freezes.min(3);
        track.longest = track.longest.max(track.current);
        track.last = Some(number);
    } else if let Some(last) = before.last {
        // Two real misses in a row lose a live run; one miss waits for a freeze.
        if number - last > 2 && before.current > 0 {
            broke = true;
            track.armed |= before.current >= 7;
            track.current = 0;
        }
    }
    let mut events = Vec::new();
    if froze {
        events.push((-1, FreezeReason::Consumed));
    }
    if broke {
        events.push((0, FreezeReason::StreakBreak));
    }
    let gained = i64::from(track.freezes) - i64::from(before.freezes) + i64::from(froze);
    if gained > 0 {
        events.push((
            i32::try_from(gained).expect("small"),
            FreezeReason::StreakEarn,
        ));
    }
    events
}

/// A rule (A29): the language row and the events of the day walked to are what the rules give when
/// every day from the first study day is applied in order. Every subset of a twelve-day window as
/// the study days, and every `through` from the first day to three days past the window.
#[test]
fn the_language_row_and_events_follow_the_rules_over_every_subset_of_twelve_days() {
    const BASE: i64 = 20_000;
    let mut cases = 0_u32;
    let mut earning = 0_u32;
    let mut freezing = 0_u32;
    let mut breaking = 0_u32;
    for code in 0..(1_u32 << 12) {
        let days: BTreeSet<i64> = (0..12)
            .filter(|bit| code >> bit & 1 == 1)
            .map(|b| BASE + b)
            .collect();
        let studied = set(days.iter().copied());
        for through in BASE..=BASE + 14 {
            let mut track = Track {
                current: 0,
                longest: 0,
                freezes: 1,
                last: None,
                armed: false,
            };
            let mut events = Vec::new();
            if let Some(&first) = days.iter().next() {
                for number in first..=through {
                    events = step(&mut track, number, days.contains(&number));
                }
            }
            let (row, written) = replay::language(&studied, &BTreeSet::new(), day(through));
            assert_eq!(
                (
                    row.current,
                    row.longest,
                    row.freezes,
                    row.last_study_day.map(StudyDay::epoch_day),
                    row.comeback_armed
                ),
                (
                    track.current,
                    track.longest,
                    track.freezes,
                    track.last,
                    track.armed
                ),
                "study days {days:?}, through {through}"
            );
            let written: Vec<(i32, FreezeReason)> =
                written.iter().map(|e| (e.delta, e.reason)).collect();
            assert_eq!(
                written, events,
                "events, study days {days:?}, through {through}"
            );
            assert!(
                written_on(&studied, through) || events.is_empty(),
                "events only on the day walked to"
            );
            earning += u32::from(events.iter().any(|e| e.1 == FreezeReason::StreakEarn));
            freezing += u32::from(events.iter().any(|e| e.1 == FreezeReason::Consumed));
            breaking += u32::from(events.iter().any(|e| e.1 == FreezeReason::StreakBreak));
            cases += 1;
        }
    }
    println!(
        "language population: {cases} cases; {earning} earn, {freezing} freeze, {breaking} break days"
    );
    assert_eq!(cases, 4_096 * 15);
    assert!(earning > 200 && freezing > 500 && breaking > 500);
}

/// Whether `through` lies at or after the first study day (events exist only from then on).
fn written_on(studied: &BTreeSet<StudyDay>, through: i64) -> bool {
    studied
        .iter()
        .next()
        .is_some_and(|d| d.epoch_day() <= through)
}

/// A rule (A30): the heat tier is how many of the heat thresholds above zero a streak has reached,
/// for every length from 0 to 400.
#[test]
fn the_heat_tier_counts_the_thresholds_reached_for_every_length() {
    assert_eq!(STREAK_HEAT.len(), 6);
    let mut cases = 0_u32;
    for days in 0..=400_u32 {
        let expected = [1_u32, 7, 30, 100, 365]
            .iter()
            .filter(|t| days >= **t)
            .count();
        assert_eq!(
            heat_tier(days),
            u32::try_from(expected).expect("small"),
            "{days} days"
        );
        cases += 1;
    }
    println!("heat-tier population: {cases} lengths");
    assert_eq!(cases, 401);
}

/// A rule (A31): the fold stores today and yesterday always, and on the first run every day up to
/// 400 days back; the truth table is every age from -3 to 403 with the first-run flag both ways.
#[test]
fn the_fold_stores_today_yesterday_and_the_first_run_window() {
    let today = 20_000;
    let mut cases = 0_u32;
    for age in -3..=403_i64 {
        for first_run in [false, true] {
            let expected = age <= 1 || (first_run && age <= 400);
            assert_eq!(
                persists(day(today - age), day(today), first_run),
                expected,
                "age {age}, first run {first_run}"
            );
            cases += 1;
        }
    }
    println!("persists population: {cases} cases");
    assert_eq!(cases, 407 * 2);
}

/// A rule (A32): the walk back from today counts the silent days that are not skip days, up to the
/// 120-day cap, and the stored anchor keeps only a run the walk could not finish; for a study day
/// at every distance from today (0 to 125) or none, with and without a skip day, and five stored
/// anchors around the walk's own.
#[test]
fn the_silence_walk_and_the_stored_anchor_follow_the_run_at_every_distance() {
    let today = 20_000;
    let mut cases = 0_u32;
    let mut open = 0_u32;
    for distance in (0..=125).map(Some).chain([None]) {
        let studied: BTreeSet<i64> = distance.map(|k| today - k).into_iter().collect();
        for skipped in [
            vec![],
            vec![today - 1],
            vec![today - 1, today - 2, today - 3],
        ] {
            let skips: BTreeSet<i64> = skipped.into_iter().collect();
            // The walk examines today and 120 days back, and the day beyond them.
            let mut silent = 0_u32;
            let mut first = today;
            for k in 0..=120 {
                if studied.contains(&(today - k)) {
                    break;
                }
                if !skips.contains(&(today - k)) {
                    silent += 1;
                    first = today - k;
                }
            }
            let exhausted = !(0..=121).any(|k| studied.contains(&(today - k)));
            let walk = silence_walk(day(today), &set(studied.clone()), &set(skips.clone()));
            assert_eq!(
                (
                    walk.silent_days,
                    walk.first_silent.epoch_day(),
                    walk.exhausted
                ),
                (silent, first, exhausted),
                "study day {distance:?} back, skips {skips:?}"
            );
            for stored in [
                None,
                Some(first - 30),
                Some(first - 1),
                Some(first),
                Some(first + 1),
            ] {
                let expected = if silent < LAPSE_AFTER_SILENT_DAYS {
                    None
                } else {
                    match stored {
                        Some(anchor) if exhausted && anchor <= first => Some(anchor),
                        _ => Some(first),
                    }
                };
                let answer = anchor_beyond_the_walk(
                    day(today),
                    &set(studied.clone()),
                    &set(skips.clone()),
                    stored.map(day),
                );
                assert_eq!(
                    answer.map(StudyDay::epoch_day),
                    expected,
                    "study day {distance:?} back, skips {skips:?}, stored {stored:?}"
                );
                open += u32::from(expected.is_some());
                cases += 1;
            }
        }
    }
    println!("silence-walk population: {cases} cases, {open} with an open lapse");
    assert_eq!(cases, 127 * 3 * 5);
    assert!(open > 1_000);
}
