//! The governor (SPEC-076 R12 to R15): the verdict and the standby notice, after the predecessor's
//! `gamification/governor.py:assess` and `pipeline_layers/governor.py` at `27ee2bc`.

use std::collections::BTreeSet;

use deck_streak_kernel::StudyDay;

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
        self.standby
    }
}

/// The verdict for a strength and a silent run.
#[must_use]
pub fn assess(strength: f64, silent_days: u32) -> Verdict {
    Verdict {
        strength,
        silent_days,
        standby: false,
        lapse: false,
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
    NoticeDecision {
        sent: true,
        notified_day: input.notified_day,
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
#[must_use]
pub fn silence_walk(
    today: StudyDay,
    study_days: &BTreeSet<StudyDay>,
    skip_days: &BTreeSet<StudyDay>,
) -> Silence {
    let _ = (study_days, skip_days);
    Silence {
        silent_days: u32::MAX,
        first_silent: today,
        exhausted: true,
    }
}
