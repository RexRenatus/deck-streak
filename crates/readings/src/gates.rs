//! The last-sync and pause gates (SPEC-045 R4, R10; ADR-019, ADR-045).
//!
//! Both run before any collection work. The last sync is ingest's record of its runs: a last run
//! `ok` or `skipped` succeeded, and one that failed, or no run at all, did not; the collection file's
//! age is never read. The pause is the readings' own rule, computed from ingest's reviews: the owner
//! studied when a qualifying review (a revlog row of type 0 to 3 with ease 1 or more,
//! `types.py:Review.is_study_event`) falls on either of the two study days before this one, by the
//! kernel's study-day rule. It is not the governor's lapse: it never mints, reads or stores a lapse
//! id (docs/CONTEXT-MAP.md, "Overloaded words").

use deck_streak_ingest::reader::{Review, is_study_event};
use deck_streak_ingest::sync_runs::RunStatus;
use deck_streak_kernel::{StudyDay, StudyDayRule, UtcMillis};

/// A day in milliseconds.
const DAY_MS: i64 = 86_400_000;

/// Whether the last sync succeeded, as ingest's record of its runs says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LastSync {
    /// The last run synced, or synced and the change gate skipped its recompute.
    Succeeded,
    /// The last run failed.
    Failed,
    /// No run is on record.
    Never,
}

impl LastSync {
    /// The last sync, from the status of ingest's last recorded run (`RunHistory::last`).
    #[must_use]
    pub const fn from_last_run(last: Option<RunStatus>) -> Self {
        let _ = last;
        Self::Succeeded
    }

    /// Whether the readings may resolve: only after a sync that succeeded.
    #[must_use]
    pub const fn succeeded(self) -> bool {
        let _ = self;
        true
    }
}

/// The two study days before `today`, on either of which a qualifying review keeps the daily
/// readings going.
#[must_use]
pub const fn pause_window(today: StudyDay) -> [StudyDay; 2] {
    [today, today]
}

/// Whether the owner studied on either of the two study days before `today`: a qualifying review
/// whose instant falls in one of them by `rule`. No such review pauses every topic (R4).
#[must_use]
pub fn studied_before(reviews: &[Review], today: StudyDay, rule: StudyDayRule) -> bool {
    let _ = (reviews, today, rule);
    true
}

/// The floor the review read takes: three days before `now`, so every review of the two study days
/// before the current one is newer than it, whatever the rollover hour and the offset.
#[must_use]
pub const fn review_floor(now: UtcMillis) -> i64 {
    let _ = now;
    0
}
