//! The streak calendar's class rule (SPEC-076 A57, A60): every served day carries exactly the
//! markers that sit on it and no other day's. Each history is built BY CONSTRUCTION from tokens
//! (a study day, a declared skip day, a real miss), so the day each marker sits on is known from
//! the layout alone and never from the replay the code under test is built on.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::calendar::{CalendarDay, Marker, language, law};
use deck_streak_streaks::constants::CALENDAR_DAYS;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Token {
    Study,
    Skip,
    Miss,
}
use Token::{Miss, Skip, Study};

/// An expected marker: it sits on `day` once the served day reaches `fire`.
struct Expect {
    fire: i64,
    day: i64,
    marker: Marker,
}

struct History {
    tokens: usize,
    days: BTreeSet<StudyDay>,
    skips: BTreeSet<StudyDay>,
    first: i64,
    end: i64,
    language: Vec<Expect>,
    law: Vec<Expect>,
}

const START: i64 = 20_000;

fn run(study: usize, hole: bool) -> Vec<Token> {
    if hole {
        vec![Study, Skip, Study]
    } else {
        vec![Study; study]
    }
}

/// One history: run 1, an optional covered miss and a second run, a gap of `real` misses with an
/// optional skip day at slot `skip_at`, and a last run.
fn build(
    first_run: (usize, bool),
    covered: Option<usize>,
    real: usize,
    skip_at: Option<usize>,
    last_run: (usize, bool),
) -> History {
    let mut tokens = run(first_run.0, first_run.1);
    let covered_at = covered.map(|second| {
        tokens.push(Miss);
        let at = tokens.len() - 1;
        tokens.extend(vec![Study; second]);
        at
    });
    let mut gap = vec![Miss; real];
    if let Some(slot) = skip_at {
        gap.insert(slot, Skip);
    }
    let gap_from = tokens.len();
    tokens.extend(gap);
    let gap_to = tokens.len();
    tokens.extend(run(last_run.0, last_run.1));

    let day_of = |index: usize| START + i64::try_from(index).expect("small index");
    let (mut days, mut skips) = (BTreeSet::new(), BTreeSet::new());
    for (index, token) in tokens.iter().enumerate() {
        match token {
            Study => days.insert(StudyDay::from_epoch_day(day_of(index))),
            Skip => skips.insert(StudyDay::from_epoch_day(day_of(index))),
            Miss => false,
        };
    }
    let mut lang = Vec::new();
    let mut lawx = Vec::new();
    // The covered miss: the freeze sits on it once the return day (the next day) is served; the
    // law run resets on it once it is past.
    if let Some(at) = covered_at {
        lang.push(Expect {
            fire: day_of(at) + 1,
            day: day_of(at),
            marker: Marker::Freeze,
        });
        lawx.push(Expect {
            fire: day_of(at) + 1,
            day: day_of(at),
            marker: Marker::Break,
        });
    }
    let misses: Vec<usize> = (gap_from..gap_to).filter(|i| tokens[*i] == Miss).collect();
    let return_day = day_of(gap_to);
    if real == 1 {
        if covered.is_none() {
            lang.push(Expect {
                fire: return_day,
                day: day_of(misses[0]),
                marker: Marker::Freeze,
            });
        } else {
            lang.push(Expect {
                fire: return_day,
                day: return_day,
                marker: Marker::Break,
            });
        }
    } else {
        // The break sits on the day after the second real miss.
        let after = day_of(misses[1]) + 1;
        lang.push(Expect {
            fire: after,
            day: after,
            marker: Marker::Break,
        });
    }
    lawx.push(Expect {
        fire: day_of(misses[0]) + 1,
        day: day_of(misses[0]),
        marker: Marker::Break,
    });
    // After the last run no study day follows: the run lapses. The language run breaks on the day
    // after the second real miss, the law run resets on the first.
    let last = day_of(tokens.len() - 1);
    lang.push(Expect {
        fire: last + 3,
        day: last + 3,
        marker: Marker::Break,
    });
    lawx.push(Expect {
        fire: last + 2,
        day: last + 1,
        marker: Marker::Break,
    });
    History {
        tokens: tokens.len(),
        days,
        skips,
        first: START,
        end: day_of(tokens.len() - 1),
        language: lang,
        law: lawx,
    }
}

fn population() -> Vec<History> {
    let first_runs = [(1, false), (2, false), (2, true)];
    let covered = [None, Some(1), Some(2)];
    let last_runs = [(1, false), (2, false), (2, true)];
    let mut out = Vec::new();
    for first_run in first_runs {
        for second in covered {
            for real in 1..=3_usize {
                let slots: Vec<Option<usize>> =
                    std::iter::once(None).chain((0..=real).map(Some)).collect();
                for skip_at in slots {
                    for last_run in last_runs {
                        out.push(build(first_run, second, real, skip_at, last_run));
                    }
                }
            }
        }
    }
    out
}

fn expected(
    history: &History,
    served: i64,
    marks: &[Expect],
    with_skips: bool,
) -> Vec<(i64, bool, Vec<Marker>)> {
    let span = i64::try_from(CALENDAR_DAYS).expect("small");
    (served - span + 1..=served)
        .map(|day| {
            let mut markers = Vec::new();
            if with_skips && history.skips.contains(&StudyDay::from_epoch_day(day)) {
                markers.push(Marker::Skip);
            }
            for mark in marks {
                if mark.day == day && mark.fire <= served {
                    markers.push(mark.marker);
                }
            }
            markers.sort();
            (
                day,
                history.days.contains(&StudyDay::from_epoch_day(day)),
                markers,
            )
        })
        .collect()
}

fn read(served: &[CalendarDay]) -> Vec<(i64, bool, Vec<Marker>)> {
    served
        .iter()
        .map(|day| (day.day.epoch_day(), day.studied, day.markers.clone()))
        .collect()
}

#[test]
fn every_served_calendar_day_carries_exactly_its_own_markers() {
    let histories = population();
    // 3 first runs x 3 covered shapes x (3 gaps, with 3, 4 and 5 skip slots) x 3 last runs.
    let members: usize = 3 * 3 * (3 + 4 + 5) * 3;
    assert_eq!(histories.len(), members);
    let mut examined = 0_usize;
    let mut derived = 0_usize;
    let mut seen: BTreeMap<Marker, usize> = BTreeMap::new();
    let mut masks: Vec<u8> = Vec::new();
    for history in &histories {
        // Every served day from the first study day to 36 days past the history's last day: the
        // window's first day sits on each marker (served = marker + 34) and one day past it (the
        // marker is one day before the window, and not served).
        derived += (history.tokens + 36) * CALENDAR_DAYS * 2;
        for served in history.first..=history.end + 36 {
            let day = StudyDay::from_epoch_day(served);
            let got = language(&history.days, &history.skips, day);
            let want = expected(history, served, &history.language, true);
            assert_eq!(read(&got), want, "language, served {served}");
            let got_law = law(&history.days, &history.skips, day);
            let want_law = expected(history, served, &history.law, true);
            assert_eq!(
                read(&got_law),
                want_law,
                "law, served {served}, days {:?}, skips {:?}",
                history.days,
                history.skips
            );
            examined += got.len() + got_law.len();
            for (_, _, markers) in &want {
                let mut mask = 0_u8;
                for marker in markers {
                    *seen.entry(*marker).or_default() += 1;
                    mask |= 1 << (*marker as u8);
                }
                masks.push(mask);
            }
        }
    }
    println!("examined {examined} calendar day(s)");
    assert_eq!(examined, derived);
    // Non-vacuity: each marker is served on some day, and every pair of markers differs on some day.
    for marker in [Marker::Skip, Marker::Freeze, Marker::Break] {
        assert!(
            seen.get(&marker).copied().unwrap_or(0) > 0,
            "{marker:?} is never served"
        );
    }
    for (a, b) in [(0, 1), (0, 2), (1, 2)] {
        assert!(
            masks.iter().any(|mask| (mask >> a) & 1 != (mask >> b) & 1),
            "markers {a} and {b} never differ on a day"
        );
    }
    // A history with no study day serves its window with no study day and no marker.
    let none = language(
        &BTreeSet::new(),
        &BTreeSet::new(),
        StudyDay::from_epoch_day(START),
    );
    assert_eq!(none.len(), CALENDAR_DAYS);
    assert!(
        none.iter()
            .all(|day| !day.studied && day.markers.is_empty())
    );
}

#[test]
fn the_open_day_is_the_windows_last_day_studied_or_not_yet() {
    // Studied day 0, one real miss (day 1), the return on day 2 with the start freeze held.
    let days: BTreeSet<StudyDay> = [START, START + 2].map(StudyDay::from_epoch_day).into();
    let skips = BTreeSet::new();
    let gap_day = StudyDay::from_epoch_day(START + 1);
    // Served on the missed day, not studied: the window ends there, no freeze yet.
    let open = language(&days, &skips, gap_day);
    assert_eq!(open.last().map(|d| d.day), Some(gap_day));
    assert!(open.iter().all(|d| d.markers.is_empty()));
    assert!(open.last().is_some_and(|d| !d.studied));
    // Served on the return day, studied: the covered day carries the freeze and the return day none.
    let returned = language(&days, &skips, StudyDay::from_epoch_day(START + 2));
    assert_eq!(
        returned.last().map(|d| (d.day.epoch_day(), d.studied)),
        Some((START + 2, true))
    );
    let marked: Vec<(i64, Vec<Marker>)> = returned
        .iter()
        .filter(|d| !d.markers.is_empty())
        .map(|d| (d.day.epoch_day(), d.markers.clone()))
        .collect();
    assert_eq!(marked, vec![(START + 1, vec![Marker::Freeze])]);
    // Served on the return day but before it is studied: the same days, no freeze spent.
    let before: BTreeSet<StudyDay> = [START].map(StudyDay::from_epoch_day).into();
    let waiting = language(&before, &skips, StudyDay::from_epoch_day(START + 2));
    assert!(waiting.iter().all(|d| d.markers.is_empty()));
    assert!(waiting.last().is_some_and(|d| !d.studied));
}

/// The words the route serves are the SPEC's: a marker is spelled as its name.
#[test]
fn a_marker_is_served_under_its_own_word() {
    assert_eq!(
        [Marker::Skip, Marker::Freeze, Marker::Break].map(Marker::as_str),
        ["skip", "freeze", "break"]
    );
}
