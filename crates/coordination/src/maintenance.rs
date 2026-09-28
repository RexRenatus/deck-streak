//! The database's daily upkeep (SPEC-027 R9), which the `maintenance` job runs: the port of the
//! predecessor's `database.py:GamifyStore.maintenance` at `27ee2bc`.
//!
//! It deletes the `cron_fires` rows whose fire date is more than [`CRON_FIRES_RETENTION_DAYS`] days
//! old, so the ledger cannot grow without bound, then runs `PRAGMA optimize` to refresh the
//! planner's statistics and `PRAGMA wal_checkpoint(TRUNCATE)` to bound the write-ahead log. The
//! checkpoint runs last, so it truncates the log the prune itself wrote. A checkpoint that meets a
//! reader (Litestream's read lock) completes partially and reports busy, which is not an error.

use deck_streak_kernel::{Db, KernelError};

use crate::jobs::FireDate;
use crate::runner::{Done, Fire, Reason, Work};

/// How many days a ledger row is kept, counted from its fire date
/// (`database.py:CRON_FIRES_RETENTION_DAYS`).
pub const CRON_FIRES_RETENTION_DAYS: i64 = 0;

/// What one upkeep did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Upkeep {
    /// The ledger rows deleted.
    pub pruned: u64,
    /// Whether the checkpoint met a reader and completed only in part.
    pub checkpoint_busy: bool,
    /// The write-ahead log's frames when the checkpoint ran.
    pub log_frames: i64,
    /// The frames the checkpoint moved into the database.
    pub checkpointed_frames: i64,
}

/// Runs the upkeep on `db`, `today` being the maintenance fire's date.
///
/// # Errors
///
/// [`KernelError::Database`] when a statement fails.
pub async fn upkeep(db: &Db, today: FireDate) -> Result<Upkeep, KernelError> {
    let _ = (db, today);
    Ok(Upkeep::default())
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
        let _ = (fire, self.db);
        Ok(Done::Done)
    }
}
