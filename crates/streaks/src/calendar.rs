//! The streak calendar (SPEC-076 section 27; ADR-302): the window of days ending at the study day
//! served, each with whether it was a study day and the freeze, skip and break markers that sit on
//! it. Derived on read from the study days and the declared skip days; nothing is stored.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

use crate::constants::CALENDAR_DAYS;
use crate::replay;

/// A marker that sits on a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Marker {
    /// A declared skip day.
    Skip,
    /// A real miss a freeze covered.
    Freeze,
    /// The day the run broke.
    Break,
}

impl Marker {
    /// The word the route serves.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skip => "skip",
            Self::Freeze => "freeze",
            Self::Break => "break",
        }
    }
}

/// One served day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarDay {
    /// The day.
    pub day: StudyDay,
    /// Whether the track had a study day on it.
    pub studied: bool,
    /// The markers that sit on it, in the order of [`Marker`].
    pub markers: Vec<Marker>,
}

fn window(days: &BTreeSet<StudyDay>, served: StudyDay) -> Vec<CalendarDay> {
    let span = i64::try_from(CALENDAR_DAYS).unwrap_or(i64::MAX);
    (served.epoch_day() - span + 1..=served.epoch_day())
        .map(|number| {
            let day = StudyDay::from_epoch_day(number);
            CalendarDay {
                day,
                studied: days.contains(&day),
                markers: Vec::new(),
            }
        })
        .collect()
}

/// Puts `marker` on `day` when `day` is in the window.
fn mark(calendar: &mut [CalendarDay], day: StudyDay, marker: Marker) {
    if let Some(served) = calendar.iter_mut().find(|served| served.day == day) {
        served.markers.push(marker);
    }
}

fn settle_order(calendar: &mut [CalendarDay]) {
    for day in calendar {
        day.markers.sort();
        day.markers.dedup();
    }
}

/// The language track's calendar ending at `served`.
///
/// A `freeze` sits on the one real miss the replay's freeze covered, found between the last study
/// day and the return day (never on the return day); a `break` sits on the day the replay marks the
/// run broken; a `skip` sits on each skip day.
#[must_use]
pub fn language(
    days: &BTreeSet<StudyDay>,
    skips: &BTreeSet<StudyDay>,
    served: StudyDay,
) -> Vec<CalendarDay> {
    let mut calendar = window(days, served);
    replay::walk(days, skips, served, |prev, transition, day| {
        if transition.froze_today
            && let Some(last) = prev.last_study_day
        {
            let covered = (last.epoch_day() + 1..day.epoch_day())
                .map(StudyDay::from_epoch_day)
                .find(|between| !skips.contains(between));
            if let Some(covered) = covered {
                mark(&mut calendar, covered, Marker::Freeze);
            }
        }
        if transition.broke_today {
            mark(&mut calendar, day, Marker::Break);
        }
    });
    for skip in skips {
        mark(&mut calendar, *skip, Marker::Skip);
    }
    settle_order(&mut calendar);
    calendar
}

/// The law track's calendar ending at `served`.
///
/// The law track holds no freezes, so it serves none. Its `break` sits on the first real miss after
/// a live run, the day its replay resets the run, once the day has passed; a `skip` sits on each
/// skip day.
#[must_use]
pub fn law(
    days: &BTreeSet<StudyDay>,
    skips: &BTreeSet<StudyDay>,
    served: StudyDay,
) -> Vec<CalendarDay> {
    let mut calendar = window(days, served);
    if let Some(first) = days.iter().next().copied() {
        let mut live = false;
        for number in first.epoch_day()..served.epoch_day() {
            let day = StudyDay::from_epoch_day(number);
            if days.contains(&day) {
                live = true;
            } else if !skips.contains(&day) && live {
                live = false;
                mark(&mut calendar, day, Marker::Break);
            }
        }
    }
    for skip in skips {
        mark(&mut calendar, *skip, Marker::Skip);
    }
    settle_order(&mut calendar);
    calendar
}
