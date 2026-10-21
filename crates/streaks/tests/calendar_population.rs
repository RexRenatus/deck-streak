//! The streak calendar's class rule (SPEC-076 A57, A60, A62): every served day carries exactly the
//! markers that sit on it and no other day's, over the predecessor's window. Each history is built
//! BY CONSTRUCTION from tokens (a study day, a declared skip day, a real miss), so the day each
//! marker sits on is known from the layout alone and never from the replay the code under test is
//! built on, and the window is written from the ruling, never read from the code under test.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::calendar::{CalendarDay, Marker, language, law};

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
    days: BTreeSet<StudyDay>,
    skips: BTreeSet<StudyDay>,
    first: i64,
    end: i64,
    language: Vec<Expect>,
    law: Vec<Expect>,
}

const START: i64 = 20_000;

/// The weekday of epoch day `day`, Monday 0 to Sunday 6: epoch day 0, 1970-01-01, was a Thursday.
const fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// The served window's first day: the Monday on or before the served day less 181 days, the
/// predecessor's 26 weeks less the served day itself (`charts.streak_calendar`; SPEC-076 section
/// 27; ADR-302 D2).
const fn window_first(served: i64) -> i64 {
    let reach = served - 181;
    reach - weekday(reach)
}

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
    // law run resets on the day after it, the return day.
    if let Some(at) = covered_at {
        lang.push(Expect {
            fire: day_of(at) + 1,
            day: day_of(at),
            marker: Marker::Freeze,
        });
        lawx.push(Expect {
            fire: day_of(at) + 1,
            day: day_of(at) + 1,
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
        day: day_of(misses[0]) + 1,
        marker: Marker::Break,
    });
    // After the last run no study day follows: the run lapses. The language run breaks on the day
    // after the second real miss, the law run resets on the day after the first.
    let last = day_of(tokens.len() - 1);
    lang.push(Expect {
        fire: last + 3,
        day: last + 3,
        marker: Marker::Break,
    });
    lawx.push(Expect {
        fire: last + 2,
        day: last + 2,
        marker: Marker::Break,
    });
    History {
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
    (window_first(served)..=served)
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
    let mut differs = [false; 3];
    let (mut on_first, mut before_first) = (0_usize, 0_usize);
    for history in &histories {
        // Every served day from the first study day to 192 days past the history's last day: the
        // last marker (three days past it) has left the longest window by then, so each marker is
        // served on every day of the window it sits in, on the window's first day when it is a
        // Monday, and not at all once it is one day before the window.
        for served in history.first..=history.end + 192 {
            // 182 days when the day 181 days back is a Monday, up to 188 when it is a Sunday.
            let length = 182 + weekday(served - 181);
            derived += 2 * usize::try_from(length).expect("a small length");
            let first = window_first(served);
            for mark in history.language.iter().chain(&history.law) {
                if mark.fire <= served && mark.day == first {
                    on_first += 1;
                }
                if mark.fire <= served && mark.day == first - 1 {
                    before_first += 1;
                }
            }
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
                for marker in markers {
                    *seen.entry(*marker).or_default() += 1;
                }
                for (pair, (a, b)) in [(0, 1), (0, 2), (1, 2)].into_iter().enumerate() {
                    let has = |n: u8| markers.iter().any(|marker| *marker as u8 == n);
                    differs[pair] |= has(a) != has(b);
                }
            }
        }
    }
    println!(
        "examined {examined} calendar day(s); a marker on the window's first day {on_first} time(s), \
         one day before it {before_first} time(s)"
    );
    assert_eq!(examined, derived);
    assert!(on_first > 0, "no marker sat on a window's first day");
    assert!(before_first > 0, "no marker sat one day before a window");
    // Non-vacuity: each marker is served on some day, and every pair of markers differs on some day.
    for marker in [Marker::Skip, Marker::Freeze, Marker::Break] {
        assert!(
            seen.get(&marker).copied().unwrap_or(0) > 0,
            "{marker:?} is never served"
        );
    }
    for (pair, differ) in differs.iter().enumerate() {
        assert!(*differ, "the markers of pair {pair} never differ on a day");
    }
    // A history with no study day serves its window with no study day and no marker. START is a
    // Friday: the predecessor serves Monday 19814 through it, 187 days (the measured golden).
    let none = language(
        &BTreeSet::new(),
        &BTreeSet::new(),
        StudyDay::from_epoch_day(START),
    );
    assert_eq!(none.len(), 187);
    assert_eq!(none.first().map(|day| day.day.epoch_day()), Some(19_814));
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

/// Reading (A) on the law track (SPEC-076 A62): its `break` sits on the day its own replay resets
/// the run, the day after the first real miss of a live run, and only once that day is served; a
/// declared skip day is bridged, so the first real miss after it is the one that counts.
#[test]
fn the_law_break_sits_on_the_day_after_the_first_real_miss() {
    // Study days 0 and 1, real misses on 2 and 3, study days again on 4 and 5.
    let days: BTreeSet<StudyDay> = [START, START + 1, START + 4, START + 5]
        .map(StudyDay::from_epoch_day)
        .into();
    let breaks = |skips: &BTreeSet<StudyDay>, served: i64| -> Vec<i64> {
        law(&days, skips, StudyDay::from_epoch_day(served))
            .iter()
            .filter(|day| day.markers.contains(&Marker::Break))
            .map(|day| day.day.epoch_day())
            .collect()
    };
    let none = BTreeSet::new();
    // Served on the first real miss: the run has not reset, so no day carries a break yet.
    assert_eq!(breaks(&none, START + 2), Vec::<i64>::new());
    // Served on the day after it and later: that day carries the break, the miss itself none.
    for served in START + 3..=START + 5 {
        assert_eq!(breaks(&none, served), vec![START + 3], "served {served}");
    }
    // Day 2 declared a skip: the first real miss is day 3, and the run resets on day 4.
    let skip: BTreeSet<StudyDay> = [START + 2].map(StudyDay::from_epoch_day).into();
    assert_eq!(breaks(&skip, START + 3), Vec::<i64>::new());
    assert_eq!(breaks(&skip, START + 4), vec![START + 4]);
}

/// The words the route serves are the SPEC's: a marker is spelled as its name.
#[test]
fn a_marker_is_served_under_its_own_word() {
    assert_eq!(
        [Marker::Skip, Marker::Freeze, Marker::Break].map(Marker::as_str),
        ["skip", "freeze", "break"]
    );
}
