//! Resolve a study day's reading topics and record them (SPEC-045 R4, R8, R9, R11): the use case
//! that runs the readings' resolution for the study day the clock names.
//!
//! It holds none of the readings' rules. It reads the last sync from ingest's record, loads the
//! taxonomy from its configured file, has ingest read the private copy read-only for the
//! resolution's deck names, cards and reviews, runs the readings' resolution with the readings'
//! queue port over ingest's engine, and records the run and every topic that ended the day, in one
//! write. The read comes before the gates because a refused night still names its topics; the
//! throwaway copy the queue needs is the collection work the gates hold back. Each topic with a day
//! set is returned for the generation (SPEC-046).
//!
//! The decks the learner keeps away from AI are read first (SPEC-381 R3), and every card of one is
//! held back from the day sets; a failed read of them ends the resolution with its named error and
//! records nothing, as a failed read of the sync record does.

use std::path::PathBuf;
use std::sync::Arc;

use deck_streak_ingest::engine::AnkiEngine;
use deck_streak_ingest::reader::CollectionReader;
use deck_streak_ingest::sensitive::SqliteSensitiveDecks;
use deck_streak_ingest::sync_runs::SqliteSyncRuns;
use deck_streak_kernel::{Clock, KernelError, StudyDayRule};
use deck_streak_readings::day_set::{
    self, EngineQueue, ReadFailure, ResolveInputs, StudyDayResolution,
};
use deck_streak_readings::gates::{self, LastSync};
use deck_streak_readings::store::{ReadingRun, RunId, RunTrigger, SqliteReadings};
use deck_streak_readings::taxonomy::Taxonomy;

/// What the use case reads, asks and writes.
pub struct ResolveParts<E> {
    runs: SqliteSyncRuns,
    reader: CollectionReader,
    queue: EngineQueue<E>,
    store: SqliteReadings,
    taxonomy: Option<PathBuf>,
    clock: Arc<dyn Clock>,
    rule: StudyDayRule,
}

impl<E> ResolveParts<E> {
    /// The use case over ingest's record `runs` and its `reader` (reading every deck, so every
    /// queued card's decks are read), the readings' `queue` and `store`, the taxonomy file at
    /// `taxonomy` when one is configured, the kernel's `clock` and its study-day `rule`.
    #[must_use]
    pub fn new(
        runs: SqliteSyncRuns,
        reader: CollectionReader,
        queue: EngineQueue<E>,
        store: SqliteReadings,
        taxonomy: Option<PathBuf>,
        clock: Arc<dyn Clock>,
        rule: StudyDayRule,
    ) -> Self {
        Self {
            runs,
            reader,
            queue,
            store,
            taxonomy,
            clock,
            rule,
        }
    }
}

/// A recorded resolution: its run's id, and the resolution itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The run's id in `reading_runs`.
    pub run: RunId,
    /// The resolution: the run's outcome, every topic's end, and the unmapped decks.
    pub resolution: StudyDayResolution,
}

/// Why a resolution was not recorded: the service's own database failed. Every failure of the
/// collection, the sync or the taxonomy is a could-not-tell state, recorded, never an error.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// Ingest's record of its syncs could not be read.
    #[error("the sync record could not be read")]
    Ledger(#[source] KernelError),
    /// The readings' record could not be written.
    #[error("the readings' record could not be written")]
    Record(#[source] KernelError),
    /// The decks kept away from AI could not be read (SPEC-381 R3).
    #[error("the decks kept away from AI could not be read")]
    Marks(#[source] KernelError),
}

/// Resolves the study day the clock names, for `trigger`, holding back every card of a deck `marks`
/// keeps away from AI, and records it.
///
/// # Errors
///
/// [`ResolveError::Marks`] when the marks cannot be read, [`ResolveError::Ledger`] when ingest's
/// record cannot be read, and [`ResolveError::Record`] when the run cannot be written; nothing is
/// recorded then.
pub async fn resolve_study_day<E>(
    parts: &ResolveParts<E>,
    marks: &SqliteSensitiveDecks,
    trigger: RunTrigger,
) -> Result<Resolved, ResolveError>
where
    E: AnkiEngine + Clone + Send + Sync + 'static,
{
    let marked = marks.read_marked().await.unwrap_or_default();
    let started_at = parts.clock.now();
    let today = parts.rule.study_day(started_at);
    let history = parts.runs.history().await.map_err(ResolveError::Ledger)?;
    let taxonomy = match parts.taxonomy.as_deref().map(Taxonomy::load) {
        Some(Ok(taxonomy)) => Some(taxonomy),
        Some(Err(refusal)) => {
            tracing::warn!(error = %refusal, "the readings' taxonomy file was refused");
            None
        }
        None => None,
    };
    let read = parts.reader.read(gates::review_floor(started_at)).await;
    let inputs = ResolveInputs {
        today,
        rule: parts.rule,
        last_sync: LastSync::from_last_run(history.last),
        taxonomy: taxonomy.as_ref(),
        read: read.as_ref().map_err(ReadFailure::from),
    };
    let resolution = day_set::resolve_holding_back(inputs, &marked, &parts.queue).await;
    let run = ReadingRun {
        trigger,
        study_day: today,
        started_at,
        finished_at: parts.clock.now(),
        outcome: resolution.outcome,
        unmapped_decks: u32::try_from(resolution.unmapped.len()).unwrap_or(u32::MAX),
    };
    let id = parts
        .store
        .record(&run, &resolution)
        .await
        .map_err(ResolveError::Record)?;
    Ok(Resolved {
        run: id,
        resolution,
    })
}
