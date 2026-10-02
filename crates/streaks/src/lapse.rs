//! The open lapse (SPEC-049 R12, R13; ADR-088): a pure function of the study days and the skip
//! days, with no clock, no file and no table, so a caller reads the same answer on every day of one
//! episode and the parity golden can prove it.
//!
//! The rule is the predecessor's `pipeline_layers/governor.py:GovernorLayer._update_governor` at
//! `27ee2bc`: walk back from the current study day while it holds no study review; a day that is
//! not a skip day is silent, and the last one met is the run's first. A skip day neither counts nor
//! ends the run, and a day with a study review ends it. The run is a lapse once it holds
//! [`LAPSE_AFTER_SILENT_DAYS`] silent days, and the lapse's id is the run's first silent day, so
//! every day of one episode reports one id.
//!
//! The walk stops at the window's first day, the earliest day the caller read. A run that reaches
//! back past it is the anchor SPEC-076 stores (R15, R16), not this function's. One exception
//! holds at the smallest epoch day: a walk that reaches it without meeting a study review answers
//! no lapse, whatever its run holds, because the day type has no day before it (SPEC-076
//! section 16, A41).

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::StudyDay;

/// The silent study days that open a lapse: `economy.json`, `governor.lapse_after_silent_days`
/// (the predecessor's `constants.py:LAPSE_AFTER_SILENT_DAYS`), equal to the file's value by the
/// test `the_threshold_is_the_economy_files`.
pub const LAPSE_AFTER_SILENT_DAYS: u32 = 3;

/// The next count of silent days. It saturates at `u32::MAX`, so a walk longer than the counter
/// can hold still answers by the rule (#534); `formal/lean/Formal/OpenLapse.lean`'s
/// `saturatingAdd` models it.
#[must_use]
pub const fn next_silent_count(silent: u32) -> u32 {
    silent.saturating_add(1)
}

/// The lapse open on `today`, by its id (the run's first silent day), if any.
///
/// `review_counts` holds the window the caller read: each study day's count of qualifying
/// reviews, and its earliest key is the window's first day. A day without a key, or with a count
/// of zero, holds no study review. `skip_days` are the declared rest days, which stay empty until
/// the skip day exists (#108). A run of at least `silent_days_to_open` silent days opens a lapse;
/// callers pass [`LAPSE_AFTER_SILENT_DAYS`]. A walk that reaches the smallest epoch day without
/// meeting a study review answers `None`, as the earlier checked step did.
#[must_use]
pub fn open_lapse(
    today: StudyDay,
    review_counts: &BTreeMap<StudyDay, u32>,
    skip_days: &BTreeSet<StudyDay>,
    silent_days_to_open: u32,
) -> Option<StudyDay> {
    let window_start = review_counts.keys().next()?.epoch_day();
    let mut silent: u32 = 0;
    let mut first_silent = None;
    // A bounded walk: the window's days, newest first, so no arithmetic on a counter can spin it.
    for number in (window_start..=today.epoch_day()).rev() {
        let day = StudyDay::from_epoch_day(number);
        if review_counts.get(&day).copied().unwrap_or(0) > 0 {
            break;
        }
        if !skip_days.contains(&day) {
            silent = next_silent_count(silent);
            first_silent = Some(day);
        }
        // The walk has no day before the smallest epoch day, so it answers nothing there, as the
        // earlier `checked_sub` step did (A41).
        if number == i64::MIN {
            return None;
        }
    }
    if silent >= silent_days_to_open {
        first_silent
    } else {
        None
    }
}

/// The lapse anchor when the silence walk may run past its cap (SPEC-076 R15, R25), the
/// predecessor's `_update_governor` at `27ee2bc`: the anchor to store for an open lapse, or `None`
/// when no lapse is open. A run the walk finished takes the walk's first silent day; a run that
/// reached the cap keeps a stored anchor not later than the walk's, else takes the walk's.
#[must_use]
pub fn anchor_beyond_the_walk(
    today: StudyDay,
    study_days: &BTreeSet<StudyDay>,
    skip_days: &BTreeSet<StudyDay>,
    stored_anchor: Option<StudyDay>,
) -> Option<StudyDay> {
    let walk = crate::governor::silence_walk(today, study_days, skip_days);
    if walk.silent_days < LAPSE_AFTER_SILENT_DAYS {
        return None;
    }
    match stored_anchor {
        Some(prev) if walk.exhausted && prev <= walk.first_silent => Some(prev),
        _ => Some(walk.first_silent),
    }
}
