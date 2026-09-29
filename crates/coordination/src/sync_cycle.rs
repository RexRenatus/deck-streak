//! The sync cycle (SPEC-022 R11, SPEC-023 R12): the one use case that syncs the private copy, then
//! either recomputes from it or records that nothing needed recomputing.
//!
//! The scheduler's daily `sync` job and the owner's `/sync` call it (SPEC-027, SPEC-026); no other
//! job syncs (ADR-037). In order, it reads the record as it stands (the gate's run-history term is
//! the run before this cycle's), syncs (the syncer owns the run's guards, retries and record),
//! flushes the notification router when the sync ran and succeeded and the parts carry a router
//! (SPEC-041 R7), collects every registered obligation's deadlines, and asks the change gate. Then it either reads
//! the window and recomputes, or leaves the skip the gate recorded. The recompute runs the fold over
//! the window it read (SPEC-071 R15; [`crate::recompute`]), with the study day in which the latest
//! successful sync started, so a day closed before that sync is settled and every later day stays
//! owed; then it writes the anchor. A fold that fails leaves the anchor as it was, so the next
//! cycle recomputes again. A cycle given the fold also collects the settle a closed day is owed as
//! an obligation ([`OWED_SETTLE`]), so the day's first successful sync after a sync that ran across
//! the rollover still recomputes, even when nothing in the collection changed. The cycle holds no
//! rule of any context: each step is its owner's.

use std::sync::Arc;

use deck_streak_ingest::engine::AnkiEngine;
use deck_streak_ingest::gate::{ChangeGate, CycleFacts, Deadline, Decision, GateError, RunReason};
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::sync::{SyncError, SyncReport, Syncer};
use deck_streak_ingest::sync_runs::{SqliteSyncRuns, SyncRunStore, Trigger};
use deck_streak_ingest::window::{WindowError, read_window};
use deck_streak_kernel::{Clock, Db, KernelError, PortFuture, StudyDayRule, UtcMillis};
use deck_streak_notifications::Router;

use crate::instruments::Instruments;
use crate::obligations::{ObligationSource, Obligations};
use crate::recompute::{Fold, FoldInput};

/// The name of the settle a closed study day is owed, as an obligation (SPEC-071 R15): the source's
/// name, and the label of its deadline, which the gate's reason and the log carry.
pub const OWED_SETTLE: &str = "owed_settle";

/// The settle a closed study day is owed, as an obligation of every cycle that runs the fold
/// (SPEC-071 R15, SPEC-023 R10). Its one deadline is the start of the current study day's first
/// successful sync: a sync that started after the close of every day before it, so the recompute
/// that follows it may settle them. After a sync that ran across the rollover left the day that
/// closed owed, the day's next successful sync comes due for it even when nothing in the collection
/// changed; a recompute that ran after that start has served it, so it holds the gate open for
/// no other cycle.
struct OwedSettle {
    runs: SqliteSyncRuns,
    rule: StudyDayRule,
}

impl OwedSettle {
    /// The deadline at `now`: the start of the first successful sync of `now`'s study day, if one
    /// has started.
    async fn due(&self, now: UtcMillis) -> Result<Vec<Deadline>, KernelError> {
        let first = self.runs.first_success_in(self.rule.study_day(now)).await?;
        Ok(first
            .into_iter()
            .map(|at| Deadline {
                label: self.name(),
                at,
            })
            .collect())
    }
}

impl ObligationSource for OwedSettle {
    fn name(&self) -> &'static str {
        OWED_SETTLE
    }

    fn deadlines(&self, now: UtcMillis) -> PortFuture<'_, Vec<Deadline>> {
        Box::pin(self.due(now))
    }
}

/// What one cycle needs: the syncer, the reader of its copy, the gate over the service's database,
/// the registered obligations, the clock, and the notification router it flushes, when it has one.
/// The runner's port for the `sync` job (`runner::SyncCycle`) is implemented over them in the
/// composition root.
pub struct CycleParts<E> {
    syncer: Syncer<E, SqliteSyncRuns>,
    reader: CollectionReader,
    gate: ChangeGate,
    obligations: Obligations,
    clock: Arc<dyn Clock>,
    router: Option<Arc<Router>>,
    fold: Option<CycleFold>,
    instruments: Option<Arc<Instruments>>,
}

/// The fold a cycle's recompute runs (SPEC-071 R15): the fold with its registered steps, the
/// database they write, the study-day rule, and the digest of the owner's courses that every
/// day's fingerprint carries.
struct CycleFold {
    fold: Arc<Fold>,
    db: Db,
    rule: StudyDayRule,
    courses_digest: Option<String>,
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
            router: None,
            fold: None,
            instruments: None,
        }
    }

    /// These parts, flushing `router` after every sync that ran and succeeded (SPEC-041 R7).
    #[must_use]
    pub fn with_flush(mut self, router: Arc<Router>) -> Self {
        self.router = Some(router);
        self
    }

    /// This cycle, running `fold` after every recompute's read (SPEC-071 R15): its steps write
    /// `db`, the gate's own database, study days are decided by `rule`, and every day's fingerprint
    /// carries `courses_digest`. A role builds its fold once, at start, and shares it with every
    /// cycle it runs. The cycle collects the settle a closed day is owed among its obligations
    /// ([`OWED_SETTLE`]), so no cycle that runs the fold can leave it out.
    #[must_use]
    pub fn with_fold(
        mut self,
        fold: Arc<Fold>,
        db: Db,
        rule: StudyDayRule,
        courses_digest: Option<String>,
    ) -> Self {
        let owed = OwedSettle {
            runs: SqliteSyncRuns::new(db.clone()),
            rule,
        };
        self.obligations.register(owed);
        self.fold = Some(CycleFold {
            fold,
            db,
            rule,
            courses_digest,
        });
        self
    }

    /// This cycle, running `instruments` after every sync's recompute (SPEC-094 R7). A role builds
    /// them once, at start, and shares them with every cycle it runs.
    #[must_use]
    pub fn with_instruments(mut self, instruments: Arc<Instruments>) -> Self {
        self.instruments = Some(instruments);
        self
    }

    /// The fold this cycle's recompute runs, when it has one.
    #[must_use]
    pub fn fold(&self) -> Option<&Fold> {
        self.fold.as_ref().map(|fold| fold.fold.as_ref())
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
    /// The recompute's fold, or the record of the last successful sync it reads, could not read or
    /// write; the anchor was left as it was, so the next cycle recomputes again.
    #[error("the recompute's fold could not run to its end")]
    Recompute(#[source] KernelError),
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
    if let (Some(router), SyncReport::Ran { run, .. }) = (&cycle.router, &sync)
        && run.outcome.is_ok()
    {
        flush(router).await;
    }
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
            if let Some(fold) = &cycle.fold {
                // A closed day is settled only after a successful sync that started after its
                // close (R15): the fold reads the study day of the latest one on record.
                let synced_in = cycle
                    .gate
                    .runs()
                    .last_success()
                    .await
                    .map_err(CycleError::Recompute)?
                    .map(|run| run.study_day);
                let input = FoldInput {
                    data: &window.data,
                    rule: fold.rule,
                    now: checked.now,
                    synced_in,
                    courses_digest: fold.courses_digest.as_deref(),
                };
                fold.fold
                    .run(&fold.db, &input)
                    .await
                    .map_err(CycleError::Recompute)?;
            }
            cycle.gate.recomputed(&checked).await?;
            Recompute::Ran {
                reason,
                reviews: window.data.reviews.len(),
                cards: window.data.cards.len(),
            }
        }
    };
    if let Some(instruments) = &cycle.instruments {
        run_instruments(instruments).await;
    }
    Ok(CycleReport { sync, recompute })
}

/// The instruments step, after a sync's recompute (SPEC-094 R7). A step that cannot run is logged
/// and never fails the sync it follows: every instrument stays due for the next.
async fn run_instruments(instruments: &Instruments) {
    match instruments.step().await {
        Ok(step) => tracing::info!(?step, "the instruments step ran"),
        Err(error) => tracing::error!(%error, "the instruments step could not run"),
    }
}

/// The router's flush, after a sync that ran and succeeded (SPEC-041 R7). A flush that cannot run is
/// logged and never fails the sync it follows: the queue keeps its holds for the next.
async fn flush(router: &Router) {
    match router.flush().await {
        Ok(flushed) => tracing::info!(?flushed, "the notification router flushed"),
        Err(error) => tracing::error!(%error, "the notification router could not flush"),
    }
}
