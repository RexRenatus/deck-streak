//! Generate a study day's readings (SPEC-046 R1 to R16): the use case that has each topic's
//! persona write its reading, holds it to every gate, repairs it once, and stores it.
//!
//! It holds none of the readings' rules. The resolution, the note texts and the vault copy come
//! through this module's ports, the persona and the runner through the agent's, and every rule of
//! form, coverage and repair through the readings' pure modules.

use std::fmt::Write as _;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use deck_streak_agent::Frontmatter;
use deck_streak_agent::compose::{Parts, compose};
use deck_streak_agent::duty::DutySpec;
use deck_streak_agent::gate::{GateOutcome, OutputGate};
use deck_streak_agent::persona::Duty;
use deck_streak_agent::roster::Roster;
use deck_streak_agent::route::AiRoute;
use deck_streak_agent::runner::Runner;
use deck_streak_agent::verdict::Cause;
use deck_streak_kernel::{Clock, KernelError, StudyDay, StudyDayRule};
use deck_streak_readings::attempts::{AttemptOutcome, AttemptRecord, AttemptTelemetry};
use deck_streak_readings::coverage::{Document, PackFailure, check_own, first_failure};
use deck_streak_readings::day_set::{ActiveTopic, TopicEnd};
use deck_streak_readings::form::{Form, word_target};
use deck_streak_readings::reading::{ReadingId, minutes, word_count};
use deck_streak_readings::repair::{self, MAX_ATTEMPTS, Step};
use deck_streak_readings::seed::{Seed, SeedNote, Track, screen};
use deck_streak_readings::state::{AgentCause, FailedReason, ReadingGate, RunOutcome, TopicState};
use deck_streak_readings::store::{
    DaySetRecord, NewReading, ReadingRun, RunId, RunTrigger, SqliteReadings, StoreError, TopicDay,
    VaultStatus,
};
use deck_streak_readings::taxonomy::Taxonomy;
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
    /// The readings' record could not be read back.
    #[error("the readings' record could not be read")]
    Read(#[from] StoreError),
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
    if !parts.route.is_configured() {
        return route_absent(parts, trigger).await;
    }
    let resolved = parts.resolver.resolve(trigger).await?;
    let study_day = parts.rule.study_day(parts.clock.now());
    let mut topics = Vec::new();
    for resolution in &resolved.resolution.topics {
        let state = match &resolution.end {
            TopicEnd::Ended(state) => *state,
            TopicEnd::DaySet(active) => {
                let state = generate_topic(parts, resolved.run, study_day, active).await?;
                let day = TopicDay {
                    study_day,
                    topic: active.topic.clone(),
                    state,
                    day_set: Some(DaySetRecord {
                        digest: active.digest.clone(),
                        card_ids: active.card_ids.clone(),
                        note_ids: active.note_ids.clone(),
                    }),
                };
                parts
                    .store
                    .record_topic_day(resolved.run, &day, parts.clock.now())
                    .await?;
                state
            }
        };
        topics.push((resolution.topic.clone(), state));
    }
    Ok(Generated {
        run: resolved.run,
        outcome: resolved.resolution.outcome,
        topics,
    })
}

/// R16: with no AI route, every topic ends `ai_route_absent` before any day set is resolved.
async fn route_absent(
    parts: &GenerateParts<'_>,
    trigger: RunTrigger,
) -> Result<Generated, GenerateError> {
    let now = parts.clock.now();
    let study_day = parts.rule.study_day(now);
    let run = ReadingRun {
        trigger,
        study_day,
        started_at: now,
        finished_at: now,
        outcome: RunOutcome::AiRouteAbsent,
        unmapped_decks: 0,
    };
    let id = parts.store.record_run(&run).await?;
    let mut universe: Vec<TopicKey> = parts.store.known_topics().await?;
    if let Some(path) = &parts.taxonomy
        && let Ok(taxonomy) = Taxonomy::load(path)
    {
        {
            universe.extend(
                taxonomy.languages().iter().filter_map(|language| {
                    TopicKey::parse(&format!("language/{}", language.code()))
                }),
            );
        }
    }
    universe.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    universe.dedup();
    let mut topics = Vec::new();
    for topic in universe {
        let day = TopicDay {
            study_day,
            topic: topic.clone(),
            state: TopicState::AiRouteAbsent,
            day_set: None,
        };
        parts.store.record_topic_day(id, &day, now).await?;
        topics.push((topic, TopicState::AiRouteAbsent));
    }
    Ok(Generated {
        run: id,
        outcome: RunOutcome::AiRouteAbsent,
        topics,
    })
}

/// Strips a frontmatter block the model wrote itself; the engine writes the real one.
fn strip_frontmatter(body: &str) -> &str {
    let trimmed = body.trim_start();
    trimmed
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n").map(|(_, after)| after))
        .unwrap_or(trimmed)
}

/// A daily reading reads no memory; the empty list is still an input, checked like the cards.
const NO_MEMORY: &str = "[]";

fn agent_cause(cause: Cause) -> AgentCause {
    AgentCause::parse(cause.as_str()).unwrap_or(AgentCause::RunFailed)
}

/// One topic's reading: carry it, or write it with at most one repair.
#[allow(clippy::too_many_lines)] // one attempt loop; its steps read best in order
async fn generate_topic(
    parts: &GenerateParts<'_>,
    run: RunId,
    study_day: StudyDay,
    active: &ActiveTopic,
) -> Result<TopicState, GenerateError> {
    let topic = &active.topic;
    if let Some(latest) = parts.store.latest_reading(topic).await?
        && latest.reading.digest == active.digest
    {
        parts.store.carry_reading(&latest.reading.id).await?;
        return Ok(TopicState::Ready);
    }
    let Some(key) = deck_streak_agent::TopicKey::parse(topic.as_str()) else {
        return Ok(TopicState::Failed(FailedReason::FormUnregistered));
    };
    let Ok(persona) = parts.roster.persona(&key, Duty::DailyReading) else {
        return Ok(TopicState::Failed(FailedReason::FormUnregistered));
    };
    let track = if topic.as_str().starts_with("language/") {
        Track::Language
    } else {
        Track::Law
    };
    let Ok(notes) = parts.notes.texts(&active.note_ids).await else {
        return Ok(TopicState::Failed(FailedReason::SeedEmpty));
    };
    let new_words = if track == Track::Language {
        parts
            .notes
            .new_words(topic, &active.card_ids)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let seed = Seed {
        card_ids: active.card_ids.clone(),
        notes,
        new_words,
    };
    if let Err(reason) = screen(&seed, track) {
        return Ok(TopicState::Failed(reason));
    }
    let form = Form::of(track);
    let mut cards = String::new();
    for note in &seed.notes {
        let _ = writeln!(cards, "{}: {}", Seed::key_of(note.id), note.text);
    }
    // A new word is a card's term field: card text, so it is fenced with the notes, never trusted.
    for (index, word) in seed.new_words.iter().enumerate() {
        let _ = writeln!(cards, "new word {}: {word}", index + 1);
    }
    // SPEC-043 R11: each untrusted input is checked before it is fenced; a refusal calls nothing.
    for untrusted in [NO_MEMORY, cards.as_str()] {
        if let GateOutcome::Failed { .. } = parts.gate.check_input(untrusted).await {
            return Ok(TopicState::Failed(FailedReason::GateFailed(
                ReadingGate::Contract,
            )));
        }
    }
    let instruction = form.instruction(&seed);
    let target = word_target(seed.new_cards()).to_string();
    let spec = DutySpec::daily_reading();
    let frontmatter = Frontmatter::new(&persona, persona.band(), &[]).lines();
    let mut repair_text = String::new();
    let mut repair_gate: Option<ReadingGate> = None;
    for attempt in 1..=MAX_ATTEMPTS {
        let Ok(prompt) = compose(&Parts {
            rules: parts.prompt.rules,
            policy: parts.prompt.policy,
            template: parts.prompt.template,
            persona: persona.text(),
            duty: parts.prompt.duty,
            memory: NO_MEMORY,
            cards: &cards,
            form: &instruction,
            word_target: &target,
            repair: &repair_text,
        }) else {
            return Ok(TopicState::Failed(FailedReason::AgentUnavailable(
                AgentCause::RefusedShape,
            )));
        };
        // The runner enforces the duty's wall clock itself.
        let ran = parts.runner.run(&prompt, &spec.caps).await;
        let record = |outcome, telemetry| AttemptRecord {
            run,
            topic: topic.clone(),
            study_day,
            attempt: u8::try_from(attempt).unwrap_or(u8::MAX),
            repair_gate,
            outcome,
            telemetry,
            at: parts.clock.now(),
        };
        let reply = match ran {
            Ok(reply) => reply,
            Err(cause) => {
                let cause = agent_cause(cause);
                parts
                    .store
                    .record_attempt(&record(
                        AttemptOutcome::Unavailable(cause),
                        AttemptTelemetry::default(),
                    ))
                    .await?;
                return Ok(TopicState::Failed(FailedReason::AgentUnavailable(cause)));
            }
        };
        let telemetry = AttemptTelemetry {
            turns: reply.telemetry.turns,
            input_tokens: reply.telemetry.input_tokens,
            output_tokens: reply.telemetry.output_tokens,
            cost_micro_usd: reply.telemetry.cost_micro_usd,
            duration_ms: reply.telemetry.duration_ms,
        };
        let document = format!(
            "---\n{frontmatter}{}---\n{}",
            form.frontmatter_extra(&seed),
            strip_frontmatter(&reply.result)
        );
        let pack = match parts
            .gate
            .check(&document, persona.template().as_str())
            .await
        {
            GateOutcome::Passed => None,
            GateOutcome::Failed { class, findings } => Some(PackFailure { class, findings }),
        };
        let own = check_own(&form, &seed, &Document::parse(&document));
        let Some(failure) = first_failure(&own, pack.as_ref()) else {
            parts
                .store
                .record_attempt(&record(AttemptOutcome::Passed, telemetry))
                .await?;
            return finish(parts, study_day, active, &persona, &document).await;
        };
        let class = pack
            .as_ref()
            .map_or_else(|| failure.gate.as_str().to_owned(), |p| p.class.clone());
        parts
            .store
            .record_attempt(&record(
                AttemptOutcome::GateFailed {
                    gate: failure.gate,
                    class,
                },
                telemetry,
            ))
            .await?;
        match repair::next(attempt, &failure, &document) {
            Step::Repair(text) => {
                repair_text = text;
                repair_gate = Some(failure.gate);
            }
            Step::Fail(gate) => {
                return Ok(TopicState::Failed(FailedReason::GateFailed(gate)));
            }
        }
    }
    Ok(TopicState::Failed(FailedReason::GateFailed(
        repair_gate.unwrap_or(ReadingGate::Contract),
    )))
}

/// Stores a reading that passed every gate, and writes its vault copy.
async fn finish(
    parts: &GenerateParts<'_>,
    study_day: StudyDay,
    active: &ActiveTopic,
    persona: &deck_streak_agent::Persona,
    document: &str,
) -> Result<TopicState, GenerateError> {
    let topic = &active.topic;
    let vault = match parts
        .vault
        .write(study_day, topic, &active.digest, document)
        .await
    {
        Ok(path) => VaultStatus::Written(path),
        Err(VaultWriteFailed) => VaultStatus::Failed,
    };
    let reading = NewReading {
        id: ReadingId::of(topic.as_str(), study_day.epoch_day(), &active.digest),
        topic: topic.clone(),
        study_day,
        digest: active.digest.clone(),
        persona: persona.template().as_str().to_owned(),
        text: document.to_owned(),
        word_count: word_count(document),
        minutes: minutes(document, persona.lang()),
        card_ids: active.card_ids.clone(),
        note_count: u32::try_from(active.note_ids.len()).unwrap_or(u32::MAX),
        generated_at: parts.clock.now(),
        vault,
    };
    parts.store.store_reading(&reading).await?;
    Ok(TopicState::Ready)
}
