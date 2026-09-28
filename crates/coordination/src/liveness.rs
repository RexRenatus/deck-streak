//! The dead-man watch and the drift check (SPEC-027 R8), which the hourly `liveness` job runs.
//!
//! The watch pages when no successful sync is newer than [`DEAD_MAN_WINDOW_SECS`]: one study day
//! plus the catch-up window, a recorded divergence from the predecessor's
//! `max(LIVENESS_STALE_INTERVAL_MULTIPLE * interval, LIVENESS_MIN_STALE_SECS)`, whose multiple was
//! sized for a 15-minute interval and would wait days at one sync a day; or, when no sync has ever
//! succeeded, when the first recorded sync attempt is older than [`LIVENESS_BOOT_GRACE_SECS`]. The
//! drift check pages when the maintenance job's last fire, read at the configured offset, lies more
//! than [`ROLLOVER_DRIFT_TOLERANCE_MIN`] minutes from its slot, by the predecessor's
//! `timebase.py:signed_skew_minutes` and `rollover_skew_minutes` at `27ee2bc`, proved by
//! `goldens/signed_skew.json` and `goldens/rollover_skew.json`.
//!
//! Each pages once (R7): the first check that finds the sync dead, and the first check that sees a
//! maintenance fire off its slot. Both are read from instants the ledger and `sync_runs` already
//! hold, so a check that runs every hour pages once per episode.

use deck_streak_ingest::sync_runs::SqliteSyncRuns;
use deck_streak_kernel::{Db, KernelError, StudyDayRule, UtcMillis};

use crate::ledger::CronLedger;
use crate::runner::{Done, Fire, Reason, Work};

/// No dead-man page within this long of the first recorded sync attempt, when no sync has ever
/// succeeded (`constants.py:LIVENESS_BOOT_GRACE_SECS`).
pub const LIVENESS_BOOT_GRACE_SECS: i64 = 0;
/// How far a maintenance fire may lie from its slot, in minutes, before it pages
/// (`constants.py:ROLLOVER_DRIFT_TOLERANCE_MIN`).
pub const ROLLOVER_DRIFT_TOLERANCE_MIN: i64 = 0;
/// How long the sync may go without a success before the watch pages, in seconds: one study day
/// (ADR-037) plus the catch-up window.
pub const DEAD_MAN_WINDOW_SECS: i64 = 0;

/// The minute of the local day `hour`:`minute` is: the port of `timebase.py:minute_of_day`.
#[must_use]
pub const fn minute_of_day(hour: i64, minute: i64) -> i64 {
    let _ = (hour, minute);
    0
}

/// The shortest signed distance from `b_min` to `a_min` around the 1440-minute day, in
/// `[-720, 720)`: the port of `timebase.py:signed_skew_minutes`.
#[must_use]
pub const fn signed_skew_minutes(a_min: i64, b_min: i64) -> i64 {
    let _ = (a_min, b_min);
    0
}

/// The signed skew between a daily fire at `cron_hour` on a clock `scheduler_utc_offset_min` east
/// of UTC and a rollover at `collection_rollover_hour` on one `collection_utc_offset_min` east,
/// both as UTC minutes of the day: the port of `timebase.py:rollover_skew_minutes`.
#[must_use]
pub const fn rollover_skew_minutes(
    cron_hour: i64,
    scheduler_utc_offset_min: i64,
    collection_rollover_hour: i64,
    collection_utc_offset_min: i64,
) -> i64 {
    let _ = (
        cron_hour,
        scheduler_utc_offset_min,
        collection_rollover_hour,
        collection_utc_offset_min,
    );
    0
}

/// How far, in minutes, a maintenance fire at `fired_at` lies from the maintenance slot under
/// `rule`: negative when early.
#[must_use]
pub fn maintenance_drift_minutes(fired_at: UtcMillis, rule: StudyDayRule) -> i64 {
    let _ = (fired_at, rule);
    0
}

/// What the watch knows of the sync: its last success, and its first recorded attempt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncHistory {
    /// When the last successful sync finished.
    pub last_success_at: Option<UtcMillis>,
    /// When the first recorded sync attempt started.
    pub first_attempt_at: Option<UtcMillis>,
}

impl SyncHistory {
    /// Reads the history from `sync_runs` in `db`.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when a read fails.
    pub async fn read(sync_runs: &SqliteSyncRuns, db: &Db) -> Result<Self, KernelError> {
        let _ = (sync_runs, db);
        Ok(Self::default())
    }

    /// The instant after which the sync counts as dead, or `None` while nothing was ever
    /// attempted.
    #[must_use]
    pub const fn dead_after(&self) -> Option<UtcMillis> {
        None
    }
}

/// One check of the watch at `now`: the pages it owes, given when the previous check ran, the
/// sync's history, and when the maintenance job last fired.
#[must_use]
pub fn check(
    now: UtcMillis,
    previous_check: Option<UtcMillis>,
    history: SyncHistory,
    maintenance_fired_at: Option<UtcMillis>,
    rule: StudyDayRule,
) -> Vec<Reason> {
    let _ = (now, previous_check, history, maintenance_fired_at, rule);
    Vec::new()
}

/// The `liveness` job's work: one check of the watch.
pub struct LivenessWork<'a, L> {
    ledger: &'a L,
    sync_runs: &'a SqliteSyncRuns,
    db: &'a Db,
    rule: StudyDayRule,
}

impl<'a, L> LivenessWork<'a, L> {
    /// The check over `ledger`, `sync_runs` and `db`, under `rule`.
    #[must_use]
    pub const fn new(
        ledger: &'a L,
        sync_runs: &'a SqliteSyncRuns,
        db: &'a Db,
        rule: StudyDayRule,
    ) -> Self {
        Self {
            ledger,
            sync_runs,
            db,
            rule,
        }
    }
}

impl<L: CronLedger> Work for LivenessWork<'_, L> {
    async fn perform(&self, fire: &Fire) -> Result<Done, Reason> {
        let _ = (fire, self.ledger, self.sync_runs, self.db, self.rule);
        Ok(Done::Done)
    }
}
