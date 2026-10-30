//! The duty engine (SPEC-043 A7 to A10, A12): degradation, the gate's refusals and the record.
#![allow(clippy::expect_used)]

mod support;

use deck_streak_agent::compose::Parts;
use deck_streak_agent::deck_gate::{CardDecks, DeckScope};
use deck_streak_agent::duty::{AlertKind, DutyEngine, DutyInput, DutySpec};
use deck_streak_agent::gate::GateOutcome;
use deck_streak_agent::route::{AI_ROUTE, AiRoute};
use deck_streak_agent::runs::AgentRuns;
use deck_streak_agent::verdict::{Cause, Verdict};
use deck_streak_kernel::{Db, Environment, ManualClock, UtcMillis};
use support::{FixedDeckGate, FixedGate, FixedRunner, RecordedAlerts, RecordedVault};

const TEMPLATE: &str = "{{persona}} {{duty_rules}} {{memory|json}} {{cards|json}}";

fn input() -> DutyInput<'static> {
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
            cards: "c",
        },
        scope: DeckScope {
            cards: &[CardDecks {
                home: 1,
                current: 1,
            }],
        },
    }
}

struct Rig {
    db: Db,
    alerts: RecordedAlerts,
    vault: RecordedVault,
    _dir: tempfile::TempDir,
}

async fn rig() -> Rig {
    let dir = tempfile::tempdir().expect("a directory");
    let db = Db::open(&dir.path().join("t.db"))
        .await
        .expect("a database");
    Rig {
        db,
        alerts: RecordedAlerts::default(),
        vault: RecordedVault::default(),
        _dir: dir,
    }
}

async fn run(rig: &Rig, route: AiRoute, runner: &FixedRunner, gate: &FixedGate) -> Verdict {
    let runs = AgentRuns::new(rig.db.clone());
    let clock = ManualClock::new(UtcMillis::from_epoch_millis(5_000));
    let deck_gate = FixedDeckGate::admitting();
    let engine = DutyEngine {
        route,
        runner,
        deck_gate: &deck_gate,
        gate,
        alerts: &rig.alerts,
        vault: &rig.vault,
        runs: &runs,
        clock: &clock,
    };
    engine
        .run(&DutySpec::daily_reading(), &input())
        .await
        .expect("the run is recorded")
}

async fn rows(rig: &Rig) -> Vec<(String, Option<String>, Option<String>, Option<i64>)> {
    sqlx::query_as("SELECT verdict, cause, class, turns FROM agent_runs ORDER BY id")
        .fetch_all(rig.db.reader())
        .await
        .expect("rows")
}

#[tokio::test]
async fn an_absent_route_records_ai_route_absent_and_alerts_nothing() {
    let rig = rig().await;
    let runner = FixedRunner::replying("never");
    let verdict = run(
        &rig,
        AiRoute::Absent,
        &runner,
        &FixedGate(GateOutcome::Passed),
    )
    .await;
    assert_eq!(verdict, Verdict::AiRouteAbsent);
    assert_eq!(
        *runner.launches.lock().expect("l"),
        0,
        "nothing was launched"
    );
    assert!(rig.alerts.0.lock().expect("a").is_empty(), "no alert");
    assert!(rig.vault.delivered.lock().expect("v").is_empty());
    assert_eq!(
        rows(&rig).await,
        [("ai_route_absent".to_owned(), None, None, None)]
    );
}

#[test]
fn the_route_is_absent_unless_the_setting_says_proxy() {
    let env = |v: &[(&str, &str)]| Environment::from_vars(v.iter().copied());
    assert_eq!(
        AiRoute::from_env(&env(&[])).expect("unset"),
        AiRoute::Absent
    );
    assert_eq!(
        AiRoute::from_env(&env(&[(AI_ROUTE, "proxy")])).expect("proxy"),
        AiRoute::Proxy
    );
    assert!(
        AiRoute::from_env(&env(&[(AI_ROUTE, "openai")])).is_err(),
        "an unknown route refuses start"
    );
    assert!(AiRoute::Proxy.is_configured() && !AiRoute::Absent.is_configured());
}

#[tokio::test]
async fn a_failed_run_is_unavailable_with_its_cause_and_one_alert() {
    for cause in Cause::ALL {
        let rig = rig().await;
        let runner = FixedRunner::failing(cause);
        let verdict = run(
            &rig,
            AiRoute::Proxy,
            &runner,
            &FixedGate(GateOutcome::Passed),
        )
        .await;
        assert_eq!(verdict, Verdict::Unavailable(cause));
        let alerts = rig.alerts.0.lock().expect("a").clone();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, AlertKind::Unavailable(cause));
        assert!(
            rig.vault.delivered.lock().expect("v").is_empty(),
            "nothing is delivered"
        );
        assert_eq!(
            rows(&rig).await,
            [(
                "unavailable".to_owned(),
                Some(cause.as_str().to_owned()),
                None,
                Some(0)
            )]
        );
    }
}

#[tokio::test]
async fn an_output_failing_a_blocking_class_is_withheld_and_recorded() {
    let rig = rig().await;
    let runner = FixedRunner::replying("see https://evil.example/x");
    let gate = FixedGate(GateOutcome::Failed {
        class: "output-links".to_owned(),
        findings: vec!["output-links: finding".to_owned()],
    });
    let verdict = run(&rig, AiRoute::Proxy, &runner, &gate).await;
    let Verdict::Withheld(withheld) = &verdict else {
        panic!("expected withheld, got {verdict:?}")
    };
    assert_eq!(withheld.class, "output-links");
    assert!(
        rig.vault.delivered.lock().expect("v").is_empty(),
        "nothing is delivered"
    );
    assert_eq!(
        rows(&rig).await,
        [(
            "withheld".to_owned(),
            None,
            Some("output-links".to_owned()),
            Some(3)
        )]
    );
}

#[tokio::test]
async fn a_withheld_verdict_raises_one_alert_carrying_only_the_class() {
    let rig = rig().await;
    let runner = FixedRunner::replying("SECRET CONTENT");
    let gate = FixedGate(GateOutcome::Failed {
        class: "output-links".to_owned(),
        findings: vec!["SECRET CONTENT".to_owned()],
    });
    let _verdict = run(&rig, AiRoute::Proxy, &runner, &gate).await;
    let alerts = rig.alerts.0.lock().expect("a");
    assert_eq!(alerts.len(), 1, "one alert");
    assert_eq!(
        alerts[0].kind,
        AlertKind::Withheld("output-links".to_owned())
    );
    assert!(
        !format!("{:?}", alerts[0]).contains("SECRET"),
        "never the content"
    );
}

#[tokio::test]
async fn a_passed_output_is_delivered_and_recorded_with_its_telemetry() {
    let rig = rig().await;
    let runner = FixedRunner::replying("a good reading");
    let verdict = run(
        &rig,
        AiRoute::Proxy,
        &runner,
        &FixedGate(GateOutcome::Passed),
    )
    .await;
    let Verdict::Delivered(delivered) = &verdict else {
        panic!("expected delivered, got {verdict:?}")
    };
    assert_eq!(delivered.output, "a good reading");
    assert_eq!(*rig.vault.delivered.lock().expect("v"), ["a good reading"]);
    assert!(rig.alerts.0.lock().expect("a").is_empty());
    let (turns, input, output, cost, duration, at): (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT turns, input_tokens, output_tokens, cost_micro_usd, duration_ms, created_at FROM agent_runs",
    )
    .fetch_one(rig.db.reader())
    .await
    .expect("a row");
    assert_eq!(
        (turns, input, output, cost, duration, at),
        (3, 100, 50, 12_000, 900, 5_000)
    );
}

#[tokio::test]
async fn a_vault_that_refuses_leaves_the_run_unavailable() {
    let mut rig = rig().await;
    rig.vault.refuse = true;
    let verdict = run(
        &rig,
        AiRoute::Proxy,
        &FixedRunner::replying("ok"),
        &FixedGate(GateOutcome::Passed),
    )
    .await;
    assert_eq!(verdict, Verdict::Unavailable(Cause::RunFailed));
    assert_eq!(rig.alerts.0.lock().expect("a").len(), 1);
}

#[test]
fn the_caps_and_the_causes_are_the_specified_ones() {
    let spec = DutySpec::daily_reading();
    assert_eq!(spec.name, "daily-reading");
    assert_eq!(
        (
            spec.caps.max_turns,
            spec.caps.max_budget_micro_usd,
            spec.caps.wall_clock.as_secs()
        ),
        (30, 5_000_000, 620)
    );
    let names: Vec<&str> = Cause::ALL.iter().map(|c| c.as_str()).collect();
    assert_eq!(
        names,
        [
            "key_missing",
            "refused_shape",
            "key_rejected",
            "capacity_exhausted",
            "proxy_unreachable",
            "run_failed",
            "turn_cap",
            "time_cap",
            "budget_cap"
        ]
    );
    assert_eq!(
        deck_streak_agent::duty::DutyCaps::default()
            .wall_clock
            .as_secs(),
        1800
    );
}
