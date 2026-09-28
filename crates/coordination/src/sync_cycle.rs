//! The sync cycle (SPEC-022 R11, SPEC-023 R12): the one use case that syncs the private copy, then
//! either recomputes from it or records that nothing needed recomputing.
//!
//! The scheduler's daily `sync` job and the owner's `/sync` call it (SPEC-027, SPEC-026); no other
//! job syncs (ADR-037). In order, it reads the record as it stands (the gate's run-history term is
//! the run before this cycle's), syncs (the syncer owns the run's guards, retries and record),
//! collects every registered obligation's deadlines, and asks the change gate. Then it either reads
//! the window and recomputes, or leaves the skip the gate recorded. At W0 the recompute has no domain
//! consumer: it reads the window and writes the anchor. The cycle holds no rule of any context: each
//! step is its owner's.

use std::sync::Arc;

use deck_streak_ingest::engine::AnkiEngine;
use deck_streak_ingest::gate::{ChangeGate, CycleFacts, Decision, GateError, RunReason};
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::sync::{SyncError, SyncReport, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, Trigger};
use deck_streak_ingest::window::{WindowError, read_window};
use deck_streak_kernel::{Clock, KernelError};

use crate::obligations::Obligations;

/// What one cycle needs: the syncer, the reader of its copy, the gate over the service's database,
/// the registered obligations and the clock. The runner's port for the `sync` job
/// (`runner::SyncCycle`) is implemented over them in the composition root.
pub struct CycleParts<E> {
    syncer: Syncer<E, SqliteSyncRuns>,
    reader: CollectionReader,
    gate: ChangeGate,
    obligations: Obligations,
    clock: Arc<dyn Clock>,
}

impl<E: AnkiEngine + Sync> CycleParts<E> {
    /// A cycle over these parts. The syncer's record and the gate's are the same database.
    #[must_use]
    pub fn new(
        syncer: Syncer<E, SqliteSyncRuns>,
        reader: CollectionReader,
        gate: ChangeGate,
        obligations: Obligations,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            syncer,
            reader,
            gate,
            obligations,
            clock,
        }
    }

    /// The change gate this cycle asks.
    #[must_use]
    pub const fn gate(&self) -> &ChangeGate {
        &self.gate
    }

    /// The obligations this cycle collects.
    #[must_use]
    pub const fn obligations(&self) -> &Obligations {
        &self.obligations
    }
}

/// What the cycle did after the sync.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recompute {
    /// The gate ran the recompute, for this reason; the read held these reviews and cards.
    Ran {
        /// Why the gate ran it.
        reason: RunReason,
        /// The study reviews inside the window.
        reviews: usize,
        /// The cards in scope.
        cards: usize,
    },
    /// Nothing the recompute could observe changed: the gate recorded a `skipped` row.
    Skipped,
}

/// One cycle's account.
#[derive(Clone, Debug, PartialEq)]
pub struct CycleReport {
    /// What the sync did.
    pub sync: SyncReport,
    /// What followed it.
    pub recompute: Recompute,
}

/// Why a cycle could not be run to its end.
#[derive(Debug, thiserror::Error)]
pub enum CycleError {
    /// The record could not be read before the sync.
    #[error("the sync record could not be read")]
    History(#[source] KernelError),
    /// The sync's record could not be read or written.
    #[error(transparent)]
    Sync(#[from] SyncError),
    /// A registered obligation's deadlines could not be read.
    #[error("an obligation's deadlines could not be read")]
    Obligations(#[source] KernelError),
    /// The gate could not probe, decide or record.
    #[error(transparent)]
    Gate(#[from] GateError),
    /// The window could not be read.
    #[error(transparent)]
    Window(#[from] WindowError),
}

/// Runs one cycle for `trigger` (R12): sync, then the gate, then the recompute or the skip.
///
/// # Errors
///
/// [`CycleError`] when a step's record, the copy or an obligation cannot be read or written. A
/// failed sync is not an error: it is a recorded run, and the gate never skips after one.
pub async fn sync_cycle<E>(
    cycle: &CycleParts<E>,
    trigger: Trigger,
) -> Result<CycleReport, CycleError>
where
    E: AnkiEngine + Sync,
{
    let history = cycle.gate.history().await.map_err(CycleError::History)?;
    let sync = cycle.syncer.sync(trigger).await?;
    let sync_ok = match &sync {
        SyncReport::Ran { run, .. } => run.outcome.is_ok(),
        // A refusal or a debounce made no request: the copy is the last sync's, and the record's
        // last run says whether that one failed.
        SyncReport::RefusedToday | SyncReport::Debounced { .. } => true,
    };
    let deadlines = cycle
        .obligations
        .collect(cycle.clock.now())
        .await
        .map_err(CycleError::Obligations)?;
    let facts = CycleFacts {
        trigger,
        sync_ok,
        history,
    };
    let checked = cycle.gate.check(&cycle.reader, &facts, &deadlines).await?;
    let recompute = match checked.decision {
        Decision::Skip => Recompute::Skipped,
        Decision::Run(reason) => {
            let window = read_window(&cycle.reader, cycle.gate.state(), checked.now).await?;
            // The recompute's domain consumers arrive with their waves (#66); at W0 it leaves the
            // anchor its cycle decided on.
            cycle.gate.recomputed(&checked).await?;
            Recompute::Ran {
                reason,
                reviews: window.data.reviews.len(),
                cards: window.data.cards.len(),
            }
        }
    };
    Ok(CycleReport { sync, recompute })
}
