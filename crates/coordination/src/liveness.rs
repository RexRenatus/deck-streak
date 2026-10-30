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

use crate::jobs::{CATCHUP_MAX_LATE_MIN, MAINTENANCE, SYNC_CADENCE_SECS};
use crate::ledger::CronLedger;
use crate::runner::{Done, Fire, Reason, Work};

const MILLIS_PER_SEC: i64 = 1_000;
const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
const MINUTES_PER_HOUR: i64 = 60;
const MINUTES_PER_DAY: i64 = 1_440;
const HALF_DAY_MIN: i64 = MINUTES_PER_DAY / 2;

/// No dead-man page within this long of the first recorded sync attempt, when no sync has ever
/// succeeded (`constants.py:LIVENESS_BOOT_GRACE_SECS`).
pub const LIVENESS_BOOT_GRACE_SECS: i64 = 900;
/// How far a maintenance fire may lie from its slot, in minutes, before it pages
/// (`constants.py:ROLLOVER_DRIFT_TOLERANCE_MIN`).
pub const ROLLOVER_DRIFT_TOLERANCE_MIN: i64 = 30;
/// How long the sync may go without a success before the watch pages, in seconds: one study day
/// (ADR-037) plus the catch-up window.
pub const DEAD_MAN_WINDOW_SECS: i64 = SYNC_CADENCE_SECS + CATCHUP_MAX_LATE_MIN * 60;

/// The minute of the local day `hour`:`minute` is: the port of `timebase.py:minute_of_day`.
#[must_use]
pub const fn minute_of_day(hour: i64, minute: i64) -> i64 {
    hour * MINUTES_PER_HOUR + minute
}

/// The shortest signed distance from `b_min` to `a_min` around the 1440-minute day, in
/// `[-720, 720)`: the port of `timebase.py:signed_skew_minutes`.
#[must_use]
pub const fn signed_skew_minutes(a_min: i64, b_min: i64) -> i64 {
    (a_min - b_min + HALF_DAY_MIN).rem_euclid(MINUTES_PER_DAY) - HALF_DAY_MIN
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
    let scheduler_utc =
        (minute_of_day(cron_hour, 0) - scheduler_utc_offset_min).rem_euclid(MINUTES_PER_DAY);
    let collection_utc = (minute_of_day(collection_rollover_hour, 0) - collection_utc_offset_min)
        .rem_euclid(MINUTES_PER_DAY);
    signed_skew_minutes(scheduler_utc, collection_utc)
}

/// How far, in minutes, a maintenance fire at `fired_at` lies from the maintenance slot under
/// `rule`: negative when early.
///
/// The offset that puts the fire on its slot is the zone the timer fired in; it is compared with
/// the configured offset as the predecessor compared its scheduler's zone with the collection's
/// rollover, so a timer rendered in the wrong zone reads as the skew between the two zones.
#[must_use]
pub fn maintenance_drift_minutes(fired_at: UtcMillis, rule: StudyDayRule) -> i64 {
    let hour = i64::from(rule.rollover_hour().get());
    let minute = i64::from(MAINTENANCE.schedule.minute());
    let fired_utc = fired_at.epoch_millis().rem_euclid(DAY_MS) / MINUTE_MS;
    let timer_offset = signed_skew_minutes(minute_of_day(hour, minute), fired_utc);
    rollover_skew_minutes(
        hour,
        timer_offset,
        hour,
        i64::from(rule.utc_offset().minutes()),
    )
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
        let last_success_at = sync_runs.last_success_at().await?;
        let first = sqlx::query_scalar!(r#"SELECT min(started_at) AS "first: i64" FROM sync_runs"#)
            .fetch_one(db.reader())
            .await?;
        Ok(Self {
            last_success_at,
            first_attempt_at: first.map(UtcMillis::from_epoch_millis),
        })
    }

    /// The instant after which the sync counts as dead, or `None` while nothing was ever
    /// attempted.
    #[must_use]
    pub const fn dead_after(&self) -> Option<UtcMillis> {
        let (since, window_secs) = match (self.last_success_at, self.first_attempt_at) {
            (Some(success), _) => (success, DEAD_MAN_WINDOW_SECS),
            (None, Some(first)) => (first, LIVENESS_BOOT_GRACE_SECS),
            (None, None) => return None,
        };
        Some(UtcMillis::from_epoch_millis(
            since
                .epoch_millis()
                .saturating_add(window_secs * MILLIS_PER_SEC),
        ))
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
    let mut pages = Vec::new();
    if let Some(dead_after) = history.dead_after() {
        // The first check to find it dead: the previous one ran before the sync died.
        let first_to_find = previous_check.is_none_or(|previous| previous <= dead_after);
        if now > dead_after && first_to_find {
            let silent = |since: UtcMillis| {
                now.epoch_millis().saturating_sub(since.epoch_millis()) / MILLIS_PER_SEC
            };
            pages.push(match (history.last_success_at, history.first_attempt_at) {
                (Some(success), _) => Reason::new("sync_dead").with("silent_secs", silent(success)),
                (None, first) => Reason::new("sync_never_succeeded")
                    .with("since_first_attempt_secs", first.map_or(0, silent)),
            });
        }
    }
    if let Some(fired) = maintenance_fired_at {
        let skew = maintenance_drift_minutes(fired, rule);
        // The first check to see this fire: the previous one ran before it.
        let first_to_see = previous_check.is_none_or(|previous| fired > previous);
        if skew.abs() > ROLLOVER_DRIFT_TOLERANCE_MIN && first_to_see {
            pages.push(Reason::new("maintenance_drift").with("skew_min", skew));
        }
    }
    pages
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
        // The watch's own previous check, read before this run records its outcome.
        let previous_check = self
            .ledger
            .latest(fire.job.id)
            .await
            .map_err(unreadable)?
            .and_then(|row| row.last_fire_at);
        let history = SyncHistory::read(self.sync_runs, self.db)
            .await
            .map_err(unreadable)?;
        let maintenance_fired_at = self
            .ledger
            .latest(MAINTENANCE.id)
            .await
            .map_err(unreadable)?
            .and_then(|row| row.last_fire_at);
        let pages = check(
            fire.started_at,
            previous_check,
            history,
            maintenance_fired_at,
            self.rule,
        );
        Ok(if pages.is_empty() {
            Done::Done
        } else {
            Done::Page(pages)
        })
    }
}

/// The watch's failure when what it reads cannot be read: a reason code, never the error's text.
fn unreadable(_: KernelError) -> Reason {
    Reason::new("liveness_unreadable")
}
