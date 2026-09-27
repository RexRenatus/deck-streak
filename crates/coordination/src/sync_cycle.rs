//! The sync cycle (SPEC-022 R11): the one use case that runs a sync and records it.
//!
//! The scheduler's daily `sync` job and the owner's `/sync` call it (SPEC-027, SPEC-026); no other
//! job syncs (ADR-037). The syncer owns the run's guards, retries and record; the change gate
//! joins this cycle after the sync (SPEC-023).

use deck_streak_ingest::engine::AnkiEngine;
use deck_streak_ingest::sync::{SyncError, SyncReport, Syncer};
use deck_streak_ingest::sync_runs::{SyncRunStore, Trigger};

/// Runs one sync for `trigger` and records it.
///
/// # Errors
///
/// [`SyncError::Store`] when the run's record cannot be read or written. A failed sync is a
/// recorded run with its reason code, not an error.
pub async fn sync_cycle<E, S>(
    syncer: &Syncer<E, S>,
    trigger: Trigger,
) -> Result<SyncReport, SyncError>
where
    E: AnkiEngine + Sync,
    S: SyncRunStore,
{
    syncer.sync(trigger).await
}
