//! A track's streak replayed from its study days (SPEC-076 R10, R11, R17): the fold keeps no
//! running value between days, so each day it evaluates is answered from the days themselves, and a
//! recompute of a settled day reaches the state it reached the first time.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

use crate::freeze::{FreezeEvent, freeze_events_for};
use crate::law::{bridged_streak, law_state};
use crate::streak::{StreakState, decay_on_lapse, update_on_study};

/// The language row after `through`, and the freeze events `through` itself wrote.
///
/// Every day from the first study day to `through` is walked once: a study day updates the run, any
/// other day may decay it ([`decay_on_lapse`]). Only `through`'s events are answered, because each
/// day writes its own.
#[must_use]
pub fn language(
    days: &BTreeSet<StudyDay>,
    skips: &BTreeSet<StudyDay>,
    through: StudyDay,
) -> (StreakState, Vec<FreezeEvent>) {
    let mut prev = StreakState::start();
    let mut events = Vec::new();
    let Some(first) = days.iter().next().copied() else {
        return (prev, events);
    };
    for number in first.epoch_day()..=through.epoch_day() {
        let day = StudyDay::from_epoch_day(number);
        let transition = if days.contains(&day) {
            let observed = if prev.last_study_day.is_none() {
                bridged_streak(days, day, skips)
            } else {
                0
            };
            update_on_study(&prev, day, observed, skips)
        } else {
            decay_on_lapse(&prev, day, skips)
        };
        if day == through {
            events = freeze_events_for(&prev, &transition, day);
        }
        prev = transition.state;
    }
    (prev, events)
}

/// The law row after `through`: the run it holds and the longest run any earlier day held.
#[must_use]
pub fn law(
    days: &BTreeSet<StudyDay>,
    skips: &BTreeSet<StudyDay>,
    through: StudyDay,
) -> StreakState {
    let mut best: u32 = 0;
    if let Some(first) = days.iter().next().copied() {
        let mut run: u32 = 0;
        for number in first.epoch_day()..through.epoch_day() {
            let day = StudyDay::from_epoch_day(number);
            if days.contains(&day) {
                run = run.saturating_add(1);
                best = best.max(run);
            } else if !skips.contains(&day) {
                run = 0;
            }
        }
    }
    let seed = StreakState {
        longest: best,
        ..StreakState::start()
    };
    law_state(&seed, days, through, skips)
}
