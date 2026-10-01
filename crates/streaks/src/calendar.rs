//! The streak calendar (SPEC-076 section 27; ADR-302): the window of days ending at the study day
//! served, each with whether it was a study day and the freeze, skip and break markers that sit on
//! it. Derived on read from the study days and the declared skip days; nothing is stored.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

use crate::constants::CALENDAR_DAYS;

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

/// The language track's calendar ending at `served`.
#[must_use]
pub fn language(
    days: &BTreeSet<StudyDay>,
    _skips: &BTreeSet<StudyDay>,
    served: StudyDay,
) -> Vec<CalendarDay> {
    window(days, served)
}

/// The law track's calendar ending at `served`.
#[must_use]
pub fn law(
    days: &BTreeSet<StudyDay>,
    _skips: &BTreeSet<StudyDay>,
    served: StudyDay,
) -> Vec<CalendarDay> {
    window(days, served)
}
