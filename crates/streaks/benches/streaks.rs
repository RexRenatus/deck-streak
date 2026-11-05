//! CPU benchmarks of the streak replay, the calendars, the habit-strength fold and the silence
//! walk, measured in CI.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::{calendar, governor, replay, strength};
use divan::{Bencher, black_box};

fn main() {
    divan::main();
}

/// The first study day of every history.
const FIRST: i64 = 19_000;

/// A deterministic history of `span` days: studied on most days, with a lapse every so often, and
/// a planned skip on a few of the missed days.
fn history(span: i64) -> (BTreeSet<StudyDay>, BTreeSet<StudyDay>, StudyDay) {
    let mut days = BTreeSet::new();
    let mut skips = BTreeSet::new();
    for offset in 0..span {
        let day = StudyDay::from_epoch_day(FIRST + offset);
        if offset % 11 == 5 || offset % 17 == 9 {
            if offset % 3 == 0 {
                skips.insert(day);
            }
        } else {
            days.insert(day);
        }
    }
    (days, skips, StudyDay::from_epoch_day(FIRST + span))
}

/// The language track's streak replayed from its study days.
#[divan::bench(args = [30, 365, 1_825])]
fn language_replay(bencher: Bencher, span: i64) {
    let (days, skips, through) = history(span);
    bencher.bench(|| replay::language(black_box(&days), black_box(&skips), through));
}

/// The law track's streak replayed from its study days.
#[divan::bench(args = [30, 365, 1_825])]
fn law_replay(bencher: Bencher, span: i64) {
    let (days, skips, through) = history(span);
    bencher.bench(|| replay::law(black_box(&days), black_box(&skips), through));
}

/// The habit strength folded over every day from the first study day.
#[divan::bench(args = [30, 365, 1_825])]
fn strength_fold(bencher: Bencher, span: i64) {
    let (days, _, through) = history(span);
    bencher.bench(|| strength::fold(black_box(&days), through));
}

/// The language track's calendar, marks and all, served at the history's last day.
#[divan::bench(args = [30, 365])]
fn language_calendar(bencher: Bencher, span: i64) {
    let (days, skips, through) = history(span);
    bencher.bench(|| calendar::language(black_box(&days), black_box(&skips), through));
}

/// The law track's calendar, served at the history's last day.
#[divan::bench(args = [30, 365])]
fn law_calendar(bencher: Bencher, span: i64) {
    let (days, skips, through) = history(span);
    bencher.bench(|| calendar::law(black_box(&days), black_box(&skips), through));
}

/// The governor's silence walk back from a day long after the last study day, to its cap.
#[divan::bench]
fn silence_walk(bencher: Bencher) {
    let (days, skips, through) = history(365);
    let today = StudyDay::from_epoch_day(through.epoch_day() + 400);
    bencher.bench(|| governor::silence_walk(today, black_box(&days), black_box(&skips)));
}
