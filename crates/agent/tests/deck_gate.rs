//! The deck gate in the duty run (SPEC-381 A4, A5, A6; R4): a run whose scope holds a kept-away
//! deck, whose gate cannot read the marks, or whose cards carry no scope ends `withheld` with its
//! class, raises the one alert `withhold` raises, records it, and never starts the runner.
//!
//! The engine is built as the shipped duty tests build it, with a deck gate whose verdict is fixed.
#![allow(clippy::expect_used)]

mod support;

use deck_streak_agent::compose::Parts;
use deck_streak_agent::deck_gate::{CardDecks, DeckScope, DeckVerdict};
use deck_streak_agent::duty::{AgentAlert, AlertKind, DutyEngine, DutyInput, DutySpec};
use deck_streak_agent::gate::GateOutcome;
use deck_streak_agent::route::AiRoute;
use deck_streak_agent::runs::AgentRuns;
use deck_streak_agent::verdict::{Verdict, Withheld};
use deck_streak_kernel::{Db, ManualClock, UtcMillis};
use support::{FixedDeckGate, FixedGate, FixedRunner, RecordedAlerts, RecordedVault};

const TEMPLATE: &str = "{{persona}} {{duty_rules}} {{memory|json}} {{cards|json}}";

/// Two cards: one in its home deck, one a filtered deck borrowed.
const TWO_CARDS: [CardDecks; 2] = [
    CardDecks {
        home: 11,
        current: 11,
    },
    CardDecks {
        home: 12,
        current: 40,
    },
];

/// A run's input over `scope`, its cards' text `cards`.
fn input<'a>(cards: &'a str, scope: &'a [CardDecks]) -> DutyInput<'a> {
    DutyInput {
        template: "law-professor",
        subject: "law/evidence",
        parts: Parts {
            rules: "r",
            policy: "p",
            template: TEMPLATE,
            persona: "x",
            duty: "d",
            memory: "m",
            cards,
        },
        scope: DeckScope { cards: scope },
    }
}

/// One row of the record: its verdict, cause, class and turns.
type Row = (String, Option<String>, Option<String>, Option<i64>);

/// What one run left behind: its verdict, the runner's launches, the alerts, the deliveries, the
/// scopes the gate was asked to judge, and the record's rows.
struct Ended {
    verdict: Verdict,
    launches: u32,
    alerts: Vec<AgentAlert>,
    delivered: usize,
    asked: Vec<Vec<CardDecks>>,
    rows: Vec<Row>,
}

/// Runs the daily reading once over `input`, with a deck gate that answers `verdict`, a runner
/// that would reply, and an output gate that would pass.
async fn run(input: &DutyInput<'_>, verdict: DeckVerdict) -> Ended {
    let dir = tempfile::tempdir().expect("a directory");
    let db = Db::open(&dir.path().join("t.db"))
        .await
        .expect("a database");
    let runs = AgentRuns::new(db.clone());
    let clock = ManualClock::new(UtcMillis::from_epoch_millis(5_000));
    let runner = FixedRunner::replying("a reading");
    let deck_gate = FixedDeckGate::answering(verdict);
    let gate = FixedGate(GateOutcome::Passed);
    let alerts = RecordedAlerts::default();
    let vault = RecordedVault::default();
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
    let verdict = engine
        .run(&DutySpec::daily_reading(), input)
        .await
        .expect("the run is recorded");
    let rows = sqlx::query_as("SELECT verdict, cause, class, turns FROM agent_runs ORDER BY id")
        .fetch_all(db.reader())
        .await
        .expect("rows");
    db.close().await;
    Ended {
        verdict,
        launches: *runner.launches.lock().expect("launches"),
        alerts: alerts.0.lock().expect("alerts").clone(),
        delivered: vault.delivered.lock().expect("vault").len(),
        asked: deck_gate.asked.lock().expect("asked").clone(),
        rows,
    }
}

/// The verdict, alert and row a refusal of `class` with `finding` leaves.
fn assert_withheld(ended: &Ended, class: &str, finding: &str) {
    assert_eq!(
        ended.verdict,
        Verdict::Withheld(Withheld {
            class: class.to_owned(),
            findings: vec![finding.to_owned()],
        })
    );
    assert_eq!(ended.launches, 0, "the runner never started");
    assert_eq!(ended.delivered, 0, "nothing was delivered");
    assert_eq!(
        ended.alerts,
        [AgentAlert {
            duty: "daily-reading".to_owned(),
            kind: AlertKind::Withheld(class.to_owned()),
        }],
        "the one alert withhold raises"
    );
    assert_eq!(
        ended.rows,
        [("withheld".to_owned(), None, Some(class.to_owned()), None)],
        "the refusal is recorded, never skipped"
    );
}

#[tokio::test]
async fn a_kept_away_deck_is_withheld_before_the_runner_starts() {
    let ended = run(
        &input("two cards", &TWO_CARDS),
        DeckVerdict::KeptAway { cards: 1 },
    )
    .await;
    assert_withheld(&ended, "deck-sensitive", "1 card(s) kept away");
    assert_eq!(
        ended.asked,
        [TWO_CARDS.to_vec()],
        "the gate judged the scope"
    );
}

#[tokio::test]
async fn an_unreadable_gate_is_withheld_before_the_runner_starts() {
    let ended = run(
        &input("two cards", &TWO_CARDS),
        DeckVerdict::Unreadable { cards: 2 },
    )
    .await;
    assert_withheld(&ended, "deck-unreadable", "2 card(s) not judged");
    assert_eq!(
        ended.asked,
        [TWO_CARDS.to_vec()],
        "the gate judged the scope"
    );
}

#[tokio::test]
async fn cards_with_no_deck_scope_are_withheld_before_the_runner_starts() {
    // The gate would admit: the refusal is the engine's own, because cards with no scope cannot be
    // judged.
    let ended = run(&input("two cards", &[]), DeckVerdict::Admitted).await;
    assert_withheld(&ended, "deck-unreadable", "cards carry no deck scope");
    // A run with no cards and no scope is not refused by this rule: the control.
    let control = run(&input("", &[]), DeckVerdict::Admitted).await;
    assert_eq!(
        control.launches, 1,
        "a run with no cards reaches the runner"
    );
}
