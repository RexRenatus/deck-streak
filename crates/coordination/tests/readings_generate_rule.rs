//! Generation reads the configured study day (SPEC-047 A15): the run and each reading are dated by
//! the configured rule, never the default, over sixteen generated rules.
//!
//! This file carries its own small copy of `readings_generate.rs`'s fakes so that the stacked
//! change to that file (SPEC-046) and this one never edit the same lines. Nothing reaches a model
//! or a network, and every note text and reading here is synthetic.

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
use deck_streak_agent::gate::{GateFuture, GateOutcome, OutputGate};
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
use deck_streak_readings::day_set::{ActiveTopic, StudyDayResolution, TopicEnd, TopicResolution};
use deck_streak_readings::seed::SeedNote;
use deck_streak_readings::state::RunOutcome;
use deck_streak_readings::store::{ReadingRun, RunTrigger, SqliteReadings, StoredReading};
use deck_streak_readings::topic::TopicKey;
use serde_json::json;

const DAY_MS: i64 = 86_400_000;
const START: i64 = 20_000 * DAY_MS + 5 * 3_600_000;
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
            Some(("reading-length", "the primer prose holds 12 words"))
        } else if output.contains("COUNTDOWN") {
            Some(("no-dates", "the reading counts down to a date"))
        } else {
            None
        };
        let outcome = refusal.map_or(GateOutcome::Passed, |(class, finding)| {
            GateOutcome::Failed {
                class: class.to_owned(),
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

    async fn generate_at(
        &self,
        route: AiRoute,
        rule: StudyDayRule,
        instant: i64,
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
            clock: Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(instant))),
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

async fn readings(rig: &Rig) -> Vec<StoredReading> {
    rig.store.readings().await.expect("the readings")
}

const HOUR_MS: i64 = 3_600_000;
/// The rollover hours and the offsets, in minutes east of UTC, the rule population is the product of.
const HOURS: [i64; 2] = [0, 4];
const OFFSETS: [i64; 8] = [-720, -300, -210, 0, 330, 345, 540, 840];

/// The UTC instant study day `day` begins at, written from the definition.
fn begins(day: i64, hour: i64, offset: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS - offset * 60_000
}

#[tokio::test]
async fn the_generation_dates_its_run_and_readings_in_the_configured_study_day() {
    let mut examined = 0;
    let mut instants_examined = 0;
    for offset in OFFSETS {
        for hour in HOURS {
            let rule = StudyDayRule::new(
                Hour::new(u8::try_from(hour).expect("an hour")).expect("an hour"),
                UtcOffset::from_minutes(i16::try_from(offset).expect("minutes"))
                    .expect("an offset"),
            );
            let mut instants = std::collections::BTreeSet::new();
            for day in [20_001, 20_002] {
                let own = begins(day, hour, offset);
                let default = day * DAY_MS + 4 * HOUR_MS;
                instants.extend([own - 1, own, default - 1, default]);
            }
            for instant in instants {
                let day = (instant + offset * 60_000 - hour * HOUR_MS).div_euclid(DAY_MS);
                let configured = StudyDay::from_epoch_day(day);
                let rig = Rig::new(one_topic(), good()).await;
                rig.generate_at(AiRoute::Proxy, rule, instant).await;
                let stored = readings(&rig).await;
                assert_eq!(stored.len(), 1, "a reading");
                assert_eq!(
                    stored[0].reading.study_day, configured,
                    "offset {offset}, hour {hour}, instant {instant}: the reading is dated by the configured day"
                );
                let days = rig.store.topic_days(configured).await.expect("topic days");
                assert!(
                    !days.is_empty(),
                    "offset {offset}, hour {hour}, instant {instant}: the topic day is dated by the configured day"
                );
                let absent = Rig::new(one_topic(), good()).await;
                absent.generate_at(AiRoute::Absent, rule, instant).await;
                let runs = absent.store.runs().await.expect("runs");
                assert_eq!(
                    runs.last().expect("a run").1.study_day,
                    configured,
                    "offset {offset}, hour {hour}, instant {instant}: the absent-route run is dated by the configured day"
                );
                instants_examined += 1;
            }
            examined += 1;
        }
    }
    println!("examined {examined} configured rule(s), {instants_examined} instant(s)");
    assert_eq!(examined, OFFSETS.len() * HOURS.len());
    assert_eq!(examined, 16, "eight offsets by two rollover hours");
}
