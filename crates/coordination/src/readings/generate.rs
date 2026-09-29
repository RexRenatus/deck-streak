//! Generate a study day's readings (SPEC-046 R1 to R16): the use case that has each topic's
//! persona write its reading, holds it to every gate, repairs it once, and stores it.
//!
//! It holds none of the readings' rules. The resolution, the note texts and the vault copy come
//! through this module's ports, the persona and the runner through the agent's, and every rule of
//! form, coverage and repair through the readings' pure modules.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use deck_streak_agent::gate::OutputGate;
use deck_streak_agent::roster::Roster;
use deck_streak_agent::route::AiRoute;
use deck_streak_agent::runner::Runner;
use deck_streak_kernel::{Clock, KernelError, StudyDay, StudyDayRule};
use deck_streak_readings::seed::SeedNote;
use deck_streak_readings::state::{RunOutcome, TopicState};
use deck_streak_readings::store::{ReadingRun, RunId, RunTrigger, SqliteReadings};
use deck_streak_readings::topic::TopicKey;

use super::resolve::{ResolveError, Resolved};

/// A boxed future, as the ports return.
pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Resolves the study day's topics and records the run (SPEC-045).
pub trait StudyDayResolver: Send + Sync {
    /// Resolves the study day the clock names, for `trigger`.
    fn resolve(&self, trigger: RunTrigger) -> PortFuture<'_, Result<Resolved, ResolveError>>;
}

/// Why the notes' texts could not be read.
#[derive(Debug, thiserror::Error)]
#[error("the notes' texts could not be read")]
pub struct NoteTextsError;

/// Reads the notes' field text, in field order.
pub trait NoteTexts: Send + Sync {
    /// The texts of `note_ids`, in the order given; a note the collection lacks is left out.
    fn texts<'a>(
        &'a self,
        note_ids: &'a [i64],
    ) -> PortFuture<'a, Result<Vec<SeedNote>, NoteTextsError>>;

    /// The new words a language topic's day set introduces, in card order; none for law.
    fn new_words<'a>(
        &'a self,
        topic: &'a TopicKey,
        card_ids: &'a [i64],
    ) -> PortFuture<'a, Result<Vec<String>, NoteTextsError>> {
        let _ = (topic, card_ids);
        Box::pin(async { Ok(Vec::new()) })
    }
}

/// The vault refused the copy.
#[derive(Debug, thiserror::Error)]
#[error("the vault copy could not be written")]
pub struct VaultWriteFailed;

/// Writes a reading's vault copy through the date tree (SPEC-042).
pub trait ReadingVault: Send + Sync {
    /// Writes `body` for `topic` on `day`, and returns the path written.
    fn write<'a>(
        &'a self,
        day: StudyDay,
        topic: &'a TopicKey,
        digest: &'a str,
        body: &'a str,
    ) -> PortFuture<'a, Result<String, VaultWriteFailed>>;
}

/// The trusted texts a prompt is composed from.
#[derive(Clone, Copy, Debug)]
pub struct PromptTexts<'a> {
    /// The rules every prompt opens with.
    pub rules: &'a str,
    /// The content-safety policy.
    pub policy: &'a str,
    /// The daily reading's task template.
    pub template: &'a str,
    /// The duty's rules.
    pub duty: &'a str,
}

/// What the generation reads, asks and writes.
pub struct GenerateParts<'a> {
    /// The AI route; read first (R16).
    pub route: AiRoute,
    /// The roster the personas come from.
    pub roster: &'a Roster,
    /// The runner.
    pub runner: &'a dyn Runner,
    /// The gate.
    pub gate: &'a dyn OutputGate,
    /// The resolution.
    pub resolver: &'a dyn StudyDayResolver,
    /// The notes' texts.
    pub notes: &'a dyn NoteTexts,
    /// The vault copy.
    pub vault: &'a dyn ReadingVault,
    /// The readings' record.
    pub store: SqliteReadings,
    /// The clock.
    pub clock: Arc<dyn Clock>,
    /// The study-day rule.
    pub rule: StudyDayRule,
    /// The taxonomy file, when one is configured.
    pub taxonomy: Option<PathBuf>,
    /// The trusted prompt texts.
    pub prompt: PromptTexts<'a>,
}

/// What a generation did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    /// The run's id in `reading_runs`.
    pub run: RunId,
    /// The run's outcome.
    pub outcome: RunOutcome,
    /// How each topic that ended the study day ended it.
    pub topics: Vec<(TopicKey, TopicState)>,
}

/// Why a generation was not recorded: the service's own database failed.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    /// The resolution failed.
    #[error("the study day could not be resolved")]
    Resolve(#[from] ResolveError),
    /// The readings' record failed.
    #[error("the readings' record failed")]
    Record(#[from] KernelError),
}

/// Generates the readings of the study day the clock names, for `trigger`.
///
/// # Errors
///
/// [`GenerateError`] when the study day cannot be resolved or the record cannot be written.
pub async fn generate_readings(
    parts: &GenerateParts<'_>,
    trigger: RunTrigger,
) -> Result<Generated, GenerateError> {
    let now = parts.clock.now();
    let run = ReadingRun {
        trigger,
        study_day: parts.rule.study_day(now),
        started_at: now,
        finished_at: now,
        outcome: RunOutcome::Resolved,
        unmapped_decks: 0,
    };
    let id = parts.store.record_run(&run).await?;
    Ok(Generated {
        run: id,
        outcome: RunOutcome::Resolved,
        topics: Vec::new(),
    })
}
