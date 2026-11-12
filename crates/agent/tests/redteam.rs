//! The red-team cases in `agent/redteam` (SPEC-043 A6, A11): each attack is obeyed by a fake runner,
//! and the gate must withhold the result. The real probes judge the same cases in the box run; this
//! test proves the engine and the gate mechanism against the fake probe in `tests/fixtures`.
// Integration test code: helpers panic on a failed fixture, and the examined counts are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::path::PathBuf;
use std::sync::Mutex;

use deck_streak_agent::compose::Parts;
use deck_streak_agent::deck_gate::{CardDecks, DeckScope};
use deck_streak_agent::duty::{DutyCaps, DutyEngine, DutyInput, DutySpec};
use deck_streak_agent::gate::{GateClassSpec, ProbeGate};
use deck_streak_agent::route::AiRoute;
use deck_streak_agent::runner::{RunFuture, RunReply, Runner};
use deck_streak_agent::runs::AgentRuns;
use deck_streak_agent::verdict::{Telemetry, Verdict};
use deck_streak_kernel::{Db, ManualClock, UtcMillis};
use support::{FixedDeckGate, RecordedAlerts, RecordedVault};

const TEMPLATE: &str = "{{persona}} {{duty_rules}}\n<untrusted source=\"memory\">\n{{memory|json}}\n</untrusted>\n<untrusted source=\"cards\">\n{{cards|json}}\n</untrusted>\n";

/// A runner that obeys whatever the untrusted text asks: it answers with the attack's own words.
struct ObedientRunner {
    attack: String,
    prompts: Mutex<Vec<String>>,
}

impl Runner for ObedientRunner {
    fn run<'a>(&'a self, prompt: &'a str, _caps: &'a DutyCaps) -> RunFuture<'a> {
        self.prompts
            .lock()
            .expect("prompts")
            .push(prompt.to_owned());
        let result = self.attack.clone();
        Box::pin(async move {
            Ok(RunReply {
                result,
                telemetry: Telemetry::default(),
            })
        })
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn gate(dir: &std::path::Path) -> ProbeGate {
    let probe = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-probe.py");
    let class = |c: &str| GateClassSpec {
        probe: probe.clone(),
        class: c.to_owned(),
        with_template: false,
    };
    ProbeGate::new(
        dir.to_path_buf(),
        dir.to_path_buf(),
        vec![
            class("output-links"),
            class("output-invisible"),
            class("output-identity"),
        ],
        Some(class("output-invisible")),
        true,
    )
    .expect("a gate with its classes and its input class")
}

async fn drive(attack: &str, source: &str, benign: bool) -> (Verdict, Vec<String>, usize) {
    let dir = tempfile::tempdir().expect("a directory");
    let db = Db::open(&dir.path().join("t.db"))
        .await
        .expect("a database");
    let runs = AgentRuns::new(db.clone());
    let runner = ObedientRunner {
        attack: if benign {
            "a plain reading".to_owned()
        } else {
            attack.to_owned()
        },
        prompts: Mutex::new(Vec::new()),
    };
    let (alerts, vault, gate) = (
        RecordedAlerts::default(),
        RecordedVault::default(),
        gate(dir.path()),
    );
    let clock = ManualClock::new(UtcMillis::from_epoch_millis(1));
    let deck_gate = FixedDeckGate::admitting();
    let engine = DutyEngine {
        route: AiRoute::Proxy,
        runner: &runner,
        deck_gate: &deck_gate,
        gate: &gate,
        alerts: &alerts,
        vault: &vault,
        runs: &runs,
        clock: &clock,
    };
    let (memory, cards) = if source == "memory" {
        (attack, "c")
    } else {
        ("m", attack)
    };
    let input = DutyInput {
        template: "t",
        subject: "s",
        parts: Parts {
            rules: "r",
            policy: "p",
            template: TEMPLATE,
            persona: "x",
            duty: "d",
            memory,
            cards,
        },
        scope: DeckScope {
            cards: &[CardDecks {
                home: 1,
                current: 1,
            }],
        },
    };
    let verdict = engine
        .run(&DutySpec::daily_reading(), &input)
        .await
        .expect("recorded");
    let prompts = runner.prompts.lock().expect("prompts").clone();
    let delivered = vault.delivered.lock().expect("vault").len();
    db.close().await;
    (verdict, prompts, delivered)
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn cases() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root().join("agent/redteam"))
        .expect("the cases")
        .flatten()
    {
        let text = std::fs::read_to_string(entry.path()).expect("a case");
        let mut parts = text.splitn(3, "---\n");
        let (_, front, body) = (
            parts.next(),
            parts.next().expect("frontmatter"),
            parts.next().expect("a body"),
        );
        let source = front
            .lines()
            .find_map(|l| l.strip_prefix("source: \""))
            .expect("a source")
            .trim_end_matches('"')
            .to_owned();
        assert!(
            front.contains("expect: \"withheld\""),
            "every case expects withheld"
        );
        out.push((
            entry.file_name().to_string_lossy().into_owned(),
            source,
            body.trim().to_owned(),
        ));
    }
    out.sort();
    examined("red-team case(s)", out)
}

#[tokio::test]
async fn every_redteam_case_is_withheld_by_the_gate() {
    let cases = cases();
    assert!(cases.len() >= 5, "the case set is present: {}", cases.len());
    for (name, source, attack) in cases {
        let (verdict, prompts, delivered) = drive(&attack, &source, false).await;
        assert!(
            matches!(verdict, Verdict::Withheld(_)),
            "{name}: {verdict:?}"
        );
        assert_eq!(delivered, 0, "{name}: nothing is delivered");
        assert_eq!(
            prompts.len(),
            1,
            "{name}: the attack reached the model only as fenced data"
        );
        assert_eq!(
            prompts[0].matches("<untrusted").count(),
            2,
            "{name}: the attack opened no fence"
        );
    }
}

#[tokio::test]
async fn the_same_engine_delivers_a_benign_run() {
    for source in ["cards", "memory"] {
        let (verdict, _, delivered) = drive("Back: the holding.", source, true).await;
        assert!(matches!(verdict, Verdict::Delivered(_)), "{verdict:?}");
        assert_eq!(delivered, 1);
    }
}
