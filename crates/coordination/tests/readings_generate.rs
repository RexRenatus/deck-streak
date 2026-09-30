//! The readings' generation (SPEC-046 A7, A8, A10 to A15, A18, A19): each topic's persona writes its
//! reading through a fake runner, the reading is held to every gate, repaired once, and stored only
//! when it passed.
//!
//! Nothing reaches a model or a network: the runner, the gate, the notes, the resolution and the
//! vault are fakes, and every note text and reading here is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(
    clippy::expect_used,
    clippy::format_push_string,
    clippy::panic,
    clippy::unwrap_used
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use deck_streak_agent::duty::DutyCaps;
use deck_streak_agent::gate::{GateClass, GateFuture, GateOutcome, OutputGate};
use deck_streak_agent::persona::TemplateSet;
use deck_streak_agent::roster::Roster;
use deck_streak_agent::route::AiRoute;
use deck_streak_agent::runner::{RunFuture, RunReply, Runner};
use deck_streak_agent::verdict::{Cause, Telemetry};
use deck_streak_coordination::readings::generate::{
    GenerateParts, NoteTexts, NoteTextsError, PortFuture, PromptTexts, ReadingVault,
    StudyDayResolver, VaultWriteFailed, generate_readings,
};
use deck_streak_coordination::readings::resolve::{ResolveError, Resolved};
use deck_streak_kernel::{Db, Hour, ManualClock, StudyDay, StudyDayRule, UtcMillis, UtcOffset};
use deck_streak_readings::attempts::AttemptOutcome;
use deck_streak_readings::day_set::{ActiveTopic, StudyDayResolution, TopicEnd, TopicResolution};
use deck_streak_readings::seed::SeedNote;
use deck_streak_readings::state::{AgentCause, FailedReason, ReadingGate, RunOutcome, TopicState};
use deck_streak_readings::store::{
    ReadingRun, RunTrigger, SqliteReadings, StoredReading, VaultStatus,
};
use deck_streak_readings::topic::TopicKey;
use serde_json::json;

const DAY_MS: i64 = 86_400_000;
const START: i64 = 20_000 * DAY_MS + 5 * 3_600_000;
const ZEBRA: &str = "The distinctive rejected line about zebras crossing the synthetic record.";
const TAXONOMY: &str = r#"{
  "schema": "deckstreak.readings.taxonomy.v1",
  "law": {"roots": ["Casebook"], "bands": []},
  "languages": [
    {"deck": "Default", "code": "qaa", "display": "Alpha", "term_field": "Headword"}
  ],
  "writing_roots": []
}"#;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn topic(key: &str) -> TopicKey {
    TopicKey::parse(key).expect("a synthetic topic")
}

/// The synthetic text of note `id`: long enough to be an anchor, short enough to be quoted whole.
fn note_text(id: i64) -> String {
    format!("Synthetic rule number {id} says the proof stands")
}

/// A synthetic law reading citing and quoting every note in `ids`, with `extra` in its prose.
fn body_for(ids: &[i64], extra: &str) -> String {
    let mut quoted = String::new();
    for id in ids {
        quoted.push_str(&format!("{} [@n{id}]. ", note_text(*id)));
    }
    let mut text = String::from("# A synthetic reading\n\n");
    for (name, prose) in [
        ("reading", format!("{quoted}{extra}")),
        (
            "issue",
            "A plain question of the synthetic kind.".to_owned(),
        ),
        (
            "rule",
            "A plain statement of the synthetic rule.".to_owned(),
        ),
        (
            "application",
            "A plain application to synthetic facts.".to_owned(),
        ),
        (
            "conclusion",
            "A plain conclusion of the synthetic reading.".to_owned(),
        ),
    ] {
        text.push_str(&format!("## {name} <!-- section:{name} -->\n\n{prose}\n\n"));
    }
    text.push_str("## retrieval <!-- section:retrieval -->\n\n- What does the rule say?\n");
    text
}

/// The note ids a prompt asks the reading to cite, read from its form instruction.
fn ids_in(prompt: &str) -> Vec<i64> {
    prompt
        .lines()
        .filter_map(|line| line.strip_prefix("- `[@n")?.strip_suffix("]`"))
        .filter_map(|id| id.trim_end_matches('`').parse().ok())
        .collect()
}

type Responder = Box<dyn Fn(usize, &str) -> Result<String, Cause> + Send + Sync>;

struct FakeRunner {
    respond: Responder,
    prompts: Mutex<Vec<String>>,
}

impl FakeRunner {
    fn new(respond: Responder) -> Self {
        Self {
            respond,
            prompts: Mutex::new(Vec::new()),
        }
    }
    fn calls(&self) -> usize {
        self.prompts.lock().unwrap().len()
    }
    fn prompt(&self, at: usize) -> String {
        self.prompts.lock().unwrap()[at].clone()
    }
}

impl Runner for FakeRunner {
    fn run<'a>(&'a self, prompt: &'a str, _caps: &'a DutyCaps) -> RunFuture<'a> {
        let attempt = {
            let mut prompts = self.prompts.lock().unwrap();
            prompts.push(prompt.to_owned());
            prompts.len()
        };
        let reply = (self.respond)(attempt, prompt).map(|result| RunReply {
            result,
            telemetry: Telemetry {
                turns: 3,
                input_tokens: 100,
                output_tokens: 50,
                cost_micro_usd: 12_000,
                duration_ms: 900,
            },
        });
        Box::pin(async move { reply })
    }
}

/// A gate that stands in for the packs' probes: it refuses the markers its findings name.
struct FakeGate;

impl OutputGate for FakeGate {
    fn check<'a>(&'a self, output: &'a str, _template: &'a str) -> GateFuture<'a> {
        let refusal = if output.contains("TOOSHORT") {
            Some((GateClass::ReadingLength, "the primer prose holds 12 words"))
        } else if output.contains("COUNTDOWN") {
            Some((GateClass::NoDates, "the reading counts down to a date"))
        } else {
            None
        };
        let outcome = refusal.map_or(GateOutcome::Passed, |(class, finding)| {
            GateOutcome::Failed {
                class,
                findings: vec![finding.to_owned()],
            }
        });
        Box::pin(async move { outcome })
    }
    fn check_input<'a>(&'a self, _input: &'a str) -> GateFuture<'a> {
        Box::pin(async { GateOutcome::Passed })
    }
}

struct FakeNotes(BTreeMap<i64, String>, AtomicUsize);

impl NoteTexts for FakeNotes {
    fn new_words<'a>(
        &'a self,
        topic: &'a TopicKey,
        card_ids: &'a [i64],
    ) -> PortFuture<'a, Result<Vec<String>, NoteTextsError>> {
        let _ = (topic, card_ids);
        self.1.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(Vec::new()) })
    }

    fn texts<'a>(
        &'a self,
        note_ids: &'a [i64],
    ) -> PortFuture<'a, Result<Vec<SeedNote>, NoteTextsError>> {
        let notes = note_ids
            .iter()
            .filter_map(|id| {
                self.0.get(id).map(|text| SeedNote {
                    id: *id,
                    text: text.clone(),
                })
            })
            .collect();
        Box::pin(async move { Ok(notes) })
    }
}

#[derive(Default)]
struct FakeVault {
    fail: bool,
    written: Mutex<Vec<(i64, String, String)>>,
}

impl ReadingVault for FakeVault {
    fn write<'a>(
        &'a self,
        day: StudyDay,
        topic: &'a TopicKey,
        _digest: &'a str,
        body: &'a str,
    ) -> PortFuture<'a, Result<String, VaultWriteFailed>> {
        let result = if self.fail {
            Err(VaultWriteFailed)
        } else {
            self.written.lock().unwrap().push((
                day.epoch_day(),
                topic.as_str().to_owned(),
                body.to_owned(),
            ));
            Ok(format!("readings/{}.md", topic.as_str()))
        };
        Box::pin(async move { result })
    }
}

/// A resolver that records its run as the real one does, and counts its calls.
struct FakeResolver {
    store: SqliteReadings,
    topics: Vec<(TopicKey, Vec<i64>, Vec<i64>)>,
    calls: AtomicUsize,
}

impl StudyDayResolver for FakeResolver {
    fn resolve(&self, trigger: RunTrigger) -> PortFuture<'_, Result<Resolved, ResolveError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            let resolution = StudyDayResolution {
                outcome: RunOutcome::Resolved,
                topics: self
                    .topics
                    .iter()
                    .map(|(topic, cards, notes)| TopicResolution {
                        topic: topic.clone(),
                        end: TopicEnd::DaySet(ActiveTopic {
                            topic: topic.clone(),
                            deck_ids: vec![1],
                            card_ids: cards.clone(),
                            note_ids: notes.clone(),
                            digest: format!("{:0>64}", cards.iter().sum::<i64>()),
                        }),
                    })
                    .collect(),
                unmapped: Vec::new(),
            };
            let run = ReadingRun {
                trigger,
                study_day: StudyDay::from_epoch_day(20_000),
                started_at: UtcMillis::from_epoch_millis(START),
                finished_at: UtcMillis::from_epoch_millis(START),
                outcome: RunOutcome::Resolved,
                unmapped_decks: 0,
            };
            let id = self
                .store
                .record(&run, &resolution)
                .await
                .expect("the run is recorded");
            Ok(Resolved {
                run: id,
                resolution,
            })
        })
    }
}

struct Rig {
    _scratch: tempfile::TempDir,
    store: SqliteReadings,
    roster: Roster,
    runner: FakeRunner,
    notes: FakeNotes,
    vault: FakeVault,
    resolver: FakeResolver,
    taxonomy: PathBuf,
    texts: [String; 4],
}

impl Rig {
    async fn new(topics: Vec<(TopicKey, Vec<i64>, Vec<i64>)>, respond: Responder) -> Self {
        let scratch = tempfile::tempdir().expect("a scratch directory");
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .expect("the database");
        let store = SqliteReadings::new(db);
        let mut bound = serde_json::Map::new();
        for (topic, _, _) in &topics {
            if topic.as_str() != "law/unbound" {
                bound.insert(
                    topic.as_str().to_owned(),
                    json!({"template": "law-evidence"}),
                );
            }
        }
        let roster = Roster::parse(
            &json!({
                "schema": "deckstreak.agent.roster.v1",
                "personas": {"law-evidence": {
                    "name": "Professor Quill Testwell",
                    "bio": "A synthetic bio.",
                    "voice": "A synthetic voice.",
                    "personality": "A synthetic personality."
                }},
                "topics": bound
            })
            .to_string(),
            TemplateSet::public().expect("the public templates"),
        )
        .expect("a synthetic roster");
        let mut all = BTreeMap::new();
        for (_, _, ids) in &topics {
            for id in ids {
                all.insert(*id, note_text(*id));
            }
        }
        let taxonomy = scratch.path().join("taxonomy.json");
        fs::write(&taxonomy, TAXONOMY).expect("the taxonomy");
        let template = fs::read_to_string(repo().join("agent/prompts/daily-reading.prompt.md"))
            .expect("the prompt template");
        Self {
            resolver: FakeResolver {
                store: store.clone(),
                topics,
                calls: AtomicUsize::new(0),
            },
            _scratch: scratch,
            store,
            roster,
            runner: FakeRunner::new(respond),
            notes: FakeNotes(all, AtomicUsize::new(0)),
            vault: FakeVault::default(),
            taxonomy,
            texts: [
                "RULES".to_owned(),
                "POLICY".to_owned(),
                template,
                "DUTY".to_owned(),
            ],
        }
    }

    async fn generate(
        &self,
        route: AiRoute,
    ) -> deck_streak_coordination::readings::generate::Generated {
        self.generate_under(route, StudyDayRule::default()).await
    }

    async fn generate_under(
        &self,
        route: AiRoute,
        rule: StudyDayRule,
    ) -> deck_streak_coordination::readings::generate::Generated {
        let parts = GenerateParts {
            route,
            roster: &self.roster,
            runner: &self.runner,
            gate: &FakeGate,
            resolver: &self.resolver,
            notes: &self.notes,
            vault: &self.vault,
            store: self.store.clone(),
            clock: Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START))),
            rule,
            taxonomy: Some(self.taxonomy.clone()),
            prompt: PromptTexts {
                rules: &self.texts[0],
                policy: &self.texts[1],
                template: &self.texts[2],
                duty: &self.texts[3],
            },
        };
        generate_readings(&parts, RunTrigger::Owner)
            .await
            .expect("the generation is recorded")
    }
}

fn good() -> Responder {
    Box::new(|_, prompt| Ok(body_for(&ids_in(prompt), "")))
}

fn one_topic() -> Vec<(TopicKey, Vec<i64>, Vec<i64>)> {
    vec![(topic("law/evidence"), vec![11, 12], vec![401, 402])]
}

fn state_of(
    generated: &deck_streak_coordination::readings::generate::Generated,
    key: &str,
) -> TopicState {
    generated
        .topics
        .iter()
        .find(|(topic, _)| topic.as_str() == key)
        .map(|(_, state)| *state)
        .expect("the topic ended the day")
}

async fn readings(rig: &Rig) -> Vec<StoredReading> {
    rig.store.readings().await.expect("the readings")
}

#[tokio::test]
async fn a_gate_failure_is_repaired_once_naming_the_gate() {
    let rig = Rig::new(
        one_topic(),
        Box::new(|attempt, prompt| {
            let extra = if attempt == 1 {
                format!("TOOSHORT {ZEBRA}")
            } else {
                String::new()
            };
            Ok(body_for(&ids_in(prompt), &extra))
        }),
    )
    .await;
    let generated = rig.generate(AiRoute::Proxy).await;
    assert_eq!(state_of(&generated, "law/evidence"), TopicState::Ready);
    assert_eq!(rig.runner.calls(), 2);
    let second = rig.runner.prompt(1);
    assert!(second.contains("band"), "the repair names the gate");
    assert!(second.contains("the reading-length check refused the reading"));
    assert!(
        !second.contains("the primer prose holds 12 words"),
        "a pack's finding line never reaches the repair"
    );
    assert!(
        !second.contains("zebras"),
        "the rejected text is never quoted"
    );
    assert!(
        !rig.runner.prompt(0).contains("band"),
        "attempt one carries no repair"
    );
    assert_eq!(readings(&rig).await.len(), 1);
}

#[tokio::test]
async fn a_second_failure_writes_nothing_and_records_its_reason() {
    let rig = Rig::new(
        one_topic(),
        Box::new(|_, prompt| Ok(body_for(&ids_in(prompt), "TOOSHORT"))),
    )
    .await;
    let generated = rig.generate(AiRoute::Proxy).await;
    assert_eq!(
        state_of(&generated, "law/evidence"),
        TopicState::Failed(FailedReason::GateFailed(ReadingGate::Band))
    );
    assert_eq!(rig.runner.calls(), 2, "one repair, never a third attempt");
    assert!(readings(&rig).await.is_empty());
    assert!(rig.vault.written.lock().unwrap().is_empty());
    let days = rig
        .store
        .topic_days(StudyDay::from_epoch_day(20_000))
        .await
        .expect("days");
    assert_eq!(
        days[0].day.state,
        TopicState::Failed(FailedReason::GateFailed(ReadingGate::Band))
    );
}

#[tokio::test]
async fn every_attempt_is_recorded_with_tokens_latency_and_verdict() {
    let rig = Rig::new(
        one_topic(),
        Box::new(|attempt, prompt| {
            let extra = if attempt == 1 { "TOOSHORT" } else { "" };
            Ok(body_for(&ids_in(prompt), extra))
        }),
    )
    .await;
    let generated = rig.generate(AiRoute::Proxy).await;
    let attempts = rig
        .store
        .attempts(generated.run)
        .await
        .expect("the attempts");
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].attempt, 1);
    assert_eq!(attempts[0].repair_gate, None);
    assert_eq!(
        attempts[0].outcome,
        AttemptOutcome::GateFailed {
            gate: ReadingGate::Band,
            class: "reading-length".to_owned()
        }
    );
    assert_eq!(attempts[1].attempt, 2);
    assert_eq!(attempts[1].repair_gate, Some(ReadingGate::Band));
    assert_eq!(attempts[1].outcome, AttemptOutcome::Passed);
    for attempt in &attempts {
        assert_eq!(attempt.telemetry.turns, 3);
        assert_eq!(attempt.telemetry.input_tokens, 100);
        assert_eq!(attempt.telemetry.output_tokens, 50);
        assert_eq!(attempt.telemetry.cost_micro_usd, 12_000);
        assert_eq!(attempt.telemetry.duration_ms, 900);
        assert_eq!(attempt.topic.as_str(), "law/evidence");
    }
}

#[tokio::test]
async fn every_topic_with_new_cards_gets_a_reading_with_no_daily_cap() {
    let topics: Vec<_> = (0..12)
        .map(|n| {
            (
                topic(&format!("law/topic-{n}")),
                vec![100 + n],
                vec![500 + n],
            )
        })
        .collect();
    let rig = Rig::new(topics, good()).await;
    let generated = rig.generate(AiRoute::Proxy).await;
    assert_eq!(generated.topics.len(), 12);
    assert!(
        generated
            .topics
            .iter()
            .all(|(_, state)| *state == TopicState::Ready)
    );
    assert_eq!(readings(&rig).await.len(), 12);
    assert_eq!(rig.vault.written.lock().unwrap().len(), 12);
    assert_eq!(rig.runner.calls(), 12);
}

#[tokio::test]
async fn an_unusable_seed_fails_before_any_model_call() {
    let topics = vec![
        (topic("law/unbound"), vec![1], vec![401]),
        (topic("law/empty"), vec![2], vec![9_999]),
        (topic("law/short"), vec![3], vec![403]),
    ];
    let mut rig = Rig::new(topics, good()).await;
    rig.notes.0.insert(403, "short".to_owned());
    rig.notes.0.remove(&9_999);
    let generated = rig.generate(AiRoute::Proxy).await;
    assert_eq!(
        state_of(&generated, "law/unbound"),
        TopicState::Failed(FailedReason::FormUnregistered)
    );
    assert_eq!(
        state_of(&generated, "law/empty"),
        TopicState::Failed(FailedReason::SeedEmpty)
    );
    assert_eq!(
        state_of(&generated, "law/short"),
        TopicState::Failed(FailedReason::AnchorUnusableAll)
    );
    assert_eq!(rig.runner.calls(), 0);
    assert!(
        rig.store
            .attempts(generated.run)
            .await
            .expect("attempts")
            .is_empty()
    );
}

#[tokio::test]
async fn no_reading_is_stored_or_written_unless_every_gate_passed() {
    // The reading omits note 402's words, so the anchors gate refuses it twice.
    let rig = Rig::new(one_topic(), Box::new(|_, _| Ok(body_for(&[401], "")))).await;
    let generated = rig.generate(AiRoute::Proxy).await;
    assert_eq!(
        state_of(&generated, "law/evidence"),
        TopicState::Failed(FailedReason::GateFailed(ReadingGate::Roster))
    );
    assert!(readings(&rig).await.is_empty());
    assert!(rig.vault.written.lock().unwrap().is_empty());
    // A reading that passed is stored with exactly what the vault was given.
    let rig = Rig::new(one_topic(), good()).await;
    rig.generate(AiRoute::Proxy).await;
    let stored = readings(&rig).await;
    let written = rig.vault.written.lock().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(written.len(), 1);
    assert_eq!(stored[0].reading.text, written[0].2);
    assert!(stored[0].reading.text.starts_with("---\n"));
    assert!(stored[0].reading.text.contains("x-new-cards: 2"));
}

#[tokio::test]
async fn an_unchanged_day_set_carries_its_reading_without_a_model_call() {
    let rig = Rig::new(one_topic(), good()).await;
    rig.generate(AiRoute::Proxy).await;
    assert_eq!(rig.runner.calls(), 1);
    let again = rig.generate(AiRoute::Proxy).await;
    assert_eq!(rig.runner.calls(), 1, "no second model call");
    assert_eq!(state_of(&again, "law/evidence"), TopicState::Ready);
    let stored = readings(&rig).await;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].carried_nights, 1);
    assert_eq!(rig.vault.written.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn a_vault_write_failure_keeps_the_reading_and_records_it() {
    let mut rig = Rig::new(one_topic(), good()).await;
    rig.vault.fail = true;
    let generated = rig.generate(AiRoute::Proxy).await;
    assert_eq!(state_of(&generated, "law/evidence"), TopicState::Ready);
    let stored = readings(&rig).await;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].reading.vault, VaultStatus::Failed);
}

#[tokio::test]
async fn a_reading_with_a_date_or_countdown_is_never_delivered() {
    let rig = Rig::new(
        one_topic(),
        Box::new(|_, prompt| Ok(body_for(&ids_in(prompt), "COUNTDOWN"))),
    )
    .await;
    let generated = rig.generate(AiRoute::Proxy).await;
    assert_eq!(
        state_of(&generated, "law/evidence"),
        TopicState::Failed(FailedReason::GateFailed(ReadingGate::Contract))
    );
    assert_eq!(rig.runner.calls(), 2, "repaired once, then refused");
    assert!(readings(&rig).await.is_empty());
    assert!(rig.vault.written.lock().unwrap().is_empty());
}

#[tokio::test]
async fn an_unavailable_route_ends_the_topic_with_no_retry() {
    let rig = Rig::new(one_topic(), Box::new(|_, _| Err(Cause::CapacityExhausted))).await;
    let generated = rig.generate(AiRoute::Proxy).await;
    assert_eq!(
        state_of(&generated, "law/evidence"),
        TopicState::Failed(FailedReason::AgentUnavailable(
            AgentCause::CapacityExhausted
        ))
    );
    assert_eq!(rig.runner.calls(), 1);
}

#[tokio::test]
async fn an_absent_route_ends_every_topic_ai_route_absent_with_no_attempt() {
    let rig = Rig::new(one_topic(), good()).await;
    rig.generate(AiRoute::Proxy).await;
    let before = readings(&rig).await;
    let calls = rig.runner.calls();
    let generated = rig.generate(AiRoute::Absent).await;
    assert_eq!(
        rig.resolver.calls.load(Ordering::SeqCst),
        1,
        "no day set is resolved"
    );
    assert_eq!(rig.runner.calls(), calls);
    assert_eq!(generated.outcome, RunOutcome::AiRouteAbsent);
    assert_eq!(
        state_of(&generated, "law/evidence"),
        TopicState::AiRouteAbsent
    );
    assert_eq!(
        state_of(&generated, "language/qaa"),
        TopicState::AiRouteAbsent
    );
    assert!(
        rig.store
            .attempts(generated.run)
            .await
            .expect("attempts")
            .is_empty()
    );
    assert_eq!(readings(&rig).await, before);
    assert_eq!(rig.vault.written.lock().unwrap().len(), 1);
    let runs = rig.store.runs().await.expect("runs");
    assert_eq!(
        runs.last().expect("a run").1.outcome,
        RunOutcome::AiRouteAbsent
    );
}

#[tokio::test]
async fn an_absent_route_ends_every_topic_on_the_configured_study_day() {
    let rig = Rig::new(one_topic(), good()).await;
    let at = UtcMillis::from_epoch_millis(START);
    let rule = StudyDayRule::new(Hour::new(6).expect("an hour"), UtcOffset::UTC);
    let configured = rule.study_day(at);
    let other = StudyDayRule::default().study_day(at);
    assert_ne!(
        configured, other,
        "the configured rule moves the day at START"
    );
    let generated = rig.generate_under(AiRoute::Absent, rule).await;
    let days = rig.store.topic_days(configured).await.expect("topic days");
    for (topic, _) in &generated.topics {
        let day = days
            .iter()
            .find(|stored| stored.day.topic == *topic)
            .unwrap_or_else(|| panic!("{} did not end the configured study day", topic.as_str()));
        assert_eq!(day.day.state, TopicState::AiRouteAbsent);
    }
    assert_eq!(days.len(), generated.topics.len());
    assert!(
        rig.store
            .topic_days(other)
            .await
            .expect("topic days")
            .is_empty()
    );
    let runs = rig.store.runs().await.expect("runs");
    assert_eq!(runs.last().expect("a run").1.study_day, configured);
}

#[tokio::test]
async fn a_law_topic_never_asks_for_new_words() {
    let rig = Rig::new(one_topic(), good()).await;
    let generated = rig.generate(AiRoute::Proxy).await;
    assert_eq!(state_of(&generated, "law/evidence"), TopicState::Ready);
    assert_eq!(
        rig.notes.1.load(Ordering::SeqCst),
        0,
        "new words are fetched only for a language topic"
    );
}

/// A gate whose input check refuses an invisible character, and counts its checks (SPEC-043 R11).
struct InputRefusingGate(AtomicUsize);

impl OutputGate for InputRefusingGate {
    fn check<'a>(&'a self, output: &'a str, template: &'a str) -> GateFuture<'a> {
        FakeGate.check(output, template)
    }
    fn check_input<'a>(&'a self, input: &'a str) -> GateFuture<'a> {
        self.0.fetch_add(1, Ordering::SeqCst);
        let outcome = if input.contains('\u{200b}') {
            GateOutcome::Failed {
                class: GateClass::OutputInvisible,
                findings: vec!["an invisible character".to_owned()],
            }
        } else {
            GateOutcome::Passed
        };
        Box::pin(async move { outcome })
    }
}

#[tokio::test]
async fn an_untrusted_input_is_checked_before_any_call() {
    let mut rig = Rig::new(one_topic(), good()).await;
    rig.notes.0.insert(
        401,
        "Synthetic rule number 401 says\u{200b} the proof stands".to_owned(),
    );
    let gate = InputRefusingGate(AtomicUsize::new(0));
    let parts = GenerateParts {
        route: AiRoute::Proxy,
        roster: &rig.roster,
        runner: &rig.runner,
        gate: &gate,
        resolver: &rig.resolver,
        notes: &rig.notes,
        vault: &rig.vault,
        store: rig.store.clone(),
        clock: Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START))),
        rule: StudyDayRule::default(),
        taxonomy: Some(rig.taxonomy.clone()),
        prompt: PromptTexts {
            rules: &rig.texts[0],
            policy: &rig.texts[1],
            template: &rig.texts[2],
            duty: &rig.texts[3],
        },
    };
    let generated = generate_readings(&parts, RunTrigger::Owner)
        .await
        .expect("the generation is recorded");
    assert_eq!(
        (gate.0.load(Ordering::SeqCst), rig.runner.calls()),
        (2, 0),
        "input checks made, model calls made; the topic ended {:?}",
        state_of(&generated, "law/evidence")
    );
    assert_eq!(
        state_of(&generated, "law/evidence"),
        TopicState::Failed(FailedReason::GateFailed(ReadingGate::Contract)),
        "a refused input ends the topic on the contract gate"
    );
    let attempts = rig
        .store
        .attempts(generated.run)
        .await
        .expect("the attempts");
    assert!(attempts.is_empty(), "no attempt is made: {attempts:?}");
    assert!(readings(&rig).await.is_empty());
}

/// A notes port whose language cards carry new words, as the deck's term fields would.
struct WordNotes(BTreeMap<i64, String>, Vec<String>);

impl NoteTexts for WordNotes {
    fn new_words<'a>(
        &'a self,
        topic: &'a TopicKey,
        card_ids: &'a [i64],
    ) -> PortFuture<'a, Result<Vec<String>, NoteTextsError>> {
        let _ = (topic, card_ids);
        let words = self.1.clone();
        Box::pin(async move { Ok(words) })
    }

    fn texts<'a>(
        &'a self,
        note_ids: &'a [i64],
    ) -> PortFuture<'a, Result<Vec<SeedNote>, NoteTextsError>> {
        let notes = note_ids
            .iter()
            .filter_map(|id| {
                self.0.get(id).map(|text| SeedNote {
                    id: *id,
                    text: text.clone(),
                })
            })
            .collect();
        Box::pin(async move { Ok(notes) })
    }
}

#[tokio::test]
async fn a_new_word_is_checked_before_any_call() {
    let rig = Rig::new(vec![(topic("language/qaa"), vec![21], vec![501])], good()).await;
    let notes = WordNotes(
        BTreeMap::from([(501, "A synthetic note about a house".to_owned())]),
        vec!["casa\u{200b}".to_owned()],
    );
    let gate = InputRefusingGate(AtomicUsize::new(0));
    let parts = GenerateParts {
        route: AiRoute::Proxy,
        roster: &rig.roster,
        runner: &rig.runner,
        gate: &gate,
        resolver: &rig.resolver,
        notes: &notes,
        vault: &rig.vault,
        store: rig.store.clone(),
        clock: Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(START))),
        rule: StudyDayRule::default(),
        taxonomy: Some(rig.taxonomy.clone()),
        prompt: PromptTexts {
            rules: &rig.texts[0],
            policy: &rig.texts[1],
            template: &rig.texts[2],
            duty: &rig.texts[3],
        },
    };
    let generated = generate_readings(&parts, RunTrigger::Owner)
        .await
        .expect("the generation is recorded");
    assert_eq!(
        (gate.0.load(Ordering::SeqCst), rig.runner.calls()),
        (2, 0),
        "input checks made, model calls made; the topic ended {:?}",
        state_of(&generated, "language/qaa")
    );
    assert_eq!(
        state_of(&generated, "language/qaa"),
        TopicState::Failed(FailedReason::GateFailed(ReadingGate::Contract)),
        "a refused new word ends the topic on the contract gate"
    );
}
