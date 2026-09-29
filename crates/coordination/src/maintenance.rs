//! The database's daily upkeep (SPEC-027 R9), which the `maintenance` job runs: the port of the
//! predecessor's `database.py:GamifyStore.maintenance` at `27ee2bc`.
//!
//! It deletes the `cron_fires` rows whose fire date is more than [`CRON_FIRES_RETENTION_DAYS`] days
//! old, so the ledger cannot grow without bound, then runs `PRAGMA optimize` to refresh the
//! planner's statistics and `PRAGMA wal_checkpoint(TRUNCATE)` to bound the write-ahead log. The
//! checkpoint runs last, so it truncates the log the prune itself wrote. A checkpoint that meets a
//! reader (Litestream's read lock) completes partially and reports busy, which is not an error.

use deck_streak_agent::runs::{AgentRuns, RETENTION_DAYS};
use deck_streak_kernel::{Db, KernelError, UtcMillis};

use crate::jobs::FireDate;
use crate::runner::{Done, Fire, Reason, Work};

/// How many days a ledger row is kept, counted from its fire date
/// (`database.py:CRON_FIRES_RETENTION_DAYS`).
pub const CRON_FIRES_RETENTION_DAYS: i64 = 90;

/// Milliseconds in one day, the unit `agent_runs.created_at` counts in.
const MILLIS_PER_DAY: i64 = 86_400_000;

/// What one upkeep did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Upkeep {
    /// The ledger rows deleted.
    pub pruned: u64,
    /// The `agent_runs` rows deleted for being older than the agent's retention.
    pub agent_runs_pruned: u64,
    /// Whether the checkpoint met a reader and completed only in part.
    pub checkpoint_busy: bool,
    /// The frames left in the write-ahead log: zero once a TRUNCATE checkpoint has reset the log,
    /// and what a reader held back when it met one.
    pub log_frames: i64,
    /// The frames the checkpoint moved into the database, on the same terms.
    pub checkpointed_frames: i64,
}

/// Runs the upkeep on `db`, `today` being the maintenance fire's date.
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn upkeep(db: &Db, today: FireDate) -> Result<Upkeep, KernelError> {
    let cutoff = today.epoch_day() - CRON_FIRES_RETENTION_DAYS;
    let mut write = db.write().await?;
    let pruned = sqlx::query!("DELETE FROM cron_fires WHERE fire_date < ?1", cutoff)
        .execute(&mut *write)
        .await?
        .rows_affected();
    // On the connection that just used the ledger, so the planner knows which tables to describe.
    sqlx::query("PRAGMA optimize").execute(&mut *write).await?;
    write.commit().await?;
    // The agent's runs are kept as long as privacy.json declares; its repository does the delete.
    let runs_cutoff = (today.epoch_day() - RETENTION_DAYS) * MILLIS_PER_DAY;
    let agent_runs_pruned = AgentRuns::new(db.clone())
        .prune_before(UtcMillis::from_epoch_millis(runs_cutoff))
        .await?;
    // Outside any transaction: a checkpoint cannot run inside one.
    let (busy, log_frames, checkpointed_frames): (i64, i64, i64) =
        sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(db.reader())
            .await?;
    Ok(Upkeep {
        pruned,
        agent_runs_pruned,
        checkpoint_busy: busy != 0,
        log_frames,
        checkpointed_frames,
    })
}

/// The `maintenance` job's work: one upkeep.
pub struct MaintenanceWork<'a> {
    db: &'a Db,
}

impl<'a> MaintenanceWork<'a> {
    /// The upkeep of `db`.
    #[must_use]
    pub const fn new(db: &'a Db) -> Self {
        Self { db }
    }
}

impl Work for MaintenanceWork<'_> {
    async fn perform(&self, fire: &Fire) -> Result<Done, Reason> {
        let done = upkeep(self.db, fire.fire_date)
            .await
            .map_err(|_| Reason::new("maintenance_failed"))?;
        tracing::info!(
            pruned = done.pruned,
            agent_runs_pruned = done.agent_runs_pruned,
            checkpoint_busy = done.checkpoint_busy,
            log_frames = done.log_frames,
            checkpointed_frames = done.checkpointed_frames,
            "the database's upkeep ran"
        );
        Ok(Done::Done)
    }
}
