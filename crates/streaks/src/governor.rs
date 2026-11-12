//! The governor (SPEC-076 R12 to R15): the verdict and the standby notice, after the predecessor's
//! `gamification/governor.py:assess` and `pipeline_layers/governor.py` at `27ee2bc`.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

use crate::constants::{SILENCE_WALK_CAP_DAYS, STANDBY_NOTICE_GAP_DAYS, STRENGTH_ARM_THRESHOLD};
use crate::lapse::LAPSE_AFTER_SILENT_DAYS;

/// The governor's verdict.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Verdict {
    /// Today's strength.
    pub strength: f64,
    /// Silent study days ending today.
    pub silent_days: u32,
    /// Strength below the arm threshold.
    pub standby: bool,
    /// The silent run reached the lapse threshold.
    pub lapse: bool,
}

impl Verdict {
    /// Whether neither standby nor a lapse holds.
    #[must_use]
    pub const fn armed(&self) -> bool {
        !self.standby && !self.lapse
    }
}

/// The verdict for a strength and a silent run.
#[must_use]
pub fn assess(strength: f64, silent_days: u32) -> Verdict {
    Verdict {
        strength,
        silent_days,
        standby: strength < STRENGTH_ARM_THRESHOLD,
        lapse: silent_days >= LAPSE_AFTER_SILENT_DAYS,
    }
}

/// What the standby notice rule decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoticeDecision {
    /// Whether a notice is due now.
    pub sent: bool,
    /// The notice day to store.
    pub notified_day: Option<StudyDay>,
}

/// What the rule reads.
#[derive(Clone, Copy, Debug)]
pub struct NoticeInput {
    /// Today's verdict.
    pub verdict: Verdict,
    /// Whether the stored state was already standby.
    pub was_standby: bool,
    /// The day of the last notice.
    pub notified_day: Option<StudyDay>,
    /// Whether it is quiet hours now.
    pub quiet_hours: bool,
    /// Whether a notifier is present.
    pub notifier: bool,
    /// Today.
    pub today: StudyDay,
}

/// Whether a standby notice is due, and the notice day to store.
#[must_use]
pub fn standby_notice(input: &NoticeInput) -> NoticeDecision {
    let recently = input
        .notified_day
        .is_some_and(|d| input.today.epoch_day() - d.epoch_day() < STANDBY_NOTICE_GAP_DAYS);
    let sent = input.verdict.standby
        && !input.was_standby
        && !input.verdict.lapse
        && !recently
        && input.notifier
        && !input.quiet_hours;
    NoticeDecision {
        sent,
        notified_day: if sent {
            Some(input.today)
        } else {
            input.notified_day
        },
    }
}

/// What the silence walk found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Silence {
    /// Silent study days ending today, skip days not counted.
    pub silent_days: u32,
    /// The run's first silent day (today when none).
    pub first_silent: StudyDay,
    /// Whether the walk reached its cap without meeting a study day.
    pub exhausted: bool,
}

/// Walk back from `today` while the day holds no study, at most the cap.
///
/// Its domain is the study days an instant maps to, whose epoch day numbers lie within 2^37 of the
/// epoch. The walk steps at most `SILENCE_WALK_CAP_DAYS + 1` days below `today`, so a `today` fewer
/// days than that above the smallest epoch day is outside it (SPEC-076 R32).
#[must_use]
pub fn silence_walk(
    today: StudyDay,
    study_days: &BTreeSet<StudyDay>,
    skip_days: &BTreeSet<StudyDay>,
) -> Silence {
    let mut silent: u32 = 0;
    let mut first_silent = today;
    let mut number = today.epoch_day();
    // A bounded walk: the cap's days and one more, so no arithmetic on the counter can spin it.
    for _ in 0..=SILENCE_WALK_CAP_DAYS {
        let day = StudyDay::from_epoch_day(number);
        if study_days.contains(&day) {
            break;
        }
        if !skip_days.contains(&day) {
            silent = silent.saturating_add(1);
            first_silent = day;
        }
        number -= 1;
    }
    Silence {
        silent_days: silent,
        first_silent,
        exhausted: !study_days.contains(&StudyDay::from_epoch_day(number)),
    }
}
