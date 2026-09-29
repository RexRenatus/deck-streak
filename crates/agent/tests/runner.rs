//! The process runner (SPEC-043 A1 to A3, A7, A8): each exit of the shell runner maps to its cause,
//! and a run past a cap delivers nothing. The scripts are fakes; no proxy and no model is reached.
#![allow(clippy::expect_used)]

mod support;

use std::time::Duration;

use deck_streak_agent::duty::DutyCaps;
use deck_streak_agent::runner::{ProcessRunner, Runner};
use deck_streak_agent::verdict::Cause;
use support::script;

fn caps(seconds: u64) -> DutyCaps {
    DutyCaps {
        wall_clock: Duration::from_secs(seconds),
        ..DutyCaps::default()
    }
}

async fn run_script(body: &str, seconds: u64, grace: Duration) -> Result<String, Cause> {
    let dir = tempfile::tempdir().expect("a directory");
    let path = script(dir.path(), "runner.sh", body);
    let runner = ProcessRunner::new(path, dir.path().to_path_buf()).with_grace(grace);
    runner
        .run("a prompt", &caps(seconds))
        .await
        .map(|reply| reply.result)
}

const OK_JSON: &str = r#"{"subtype":"success","is_error":false,"num_turns":4,"total_cost_usd":0.0123,"usage":{"input_tokens":10,"output_tokens":20},"result":"a reading"}"#;

#[tokio::test]
async fn a_successful_run_returns_its_reply_and_telemetry() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = script(dir.path(), "runner.sh", &format!("echo '{OK_JSON}'"));
    let reply = ProcessRunner::new(path, dir.path().to_path_buf())
        .run("a prompt", &caps(5))
        .await
        .expect("a reply");
    assert_eq!(reply.result, "a reading");
    assert_eq!(
        (
            reply.telemetry.turns,
            reply.telemetry.input_tokens,
            reply.telemetry.output_tokens,
            reply.telemetry.cost_micro_usd
        ),
        (4, 10, 20, 12_300)
    );
}

#[tokio::test]
async fn every_runner_exit_maps_to_its_closed_cause() {
    for (code, cause) in [
        (1, Cause::KeyMissing),
        (2, Cause::RefusedShape),
        (3, Cause::KeyRejected),
        (4, Cause::CapacityExhausted),
        (5, Cause::ProxyUnreachable),
        (6, Cause::RunFailed),
        (9, Cause::RunFailed),
    ] {
        let got = run_script(&format!("exit {code}"), 5, Duration::from_secs(5)).await;
        assert_eq!(got, Err(cause), "exit {code}");
    }
}

#[tokio::test]
async fn an_unreachable_proxy_is_unavailable_with_its_cause() {
    let got = run_script(
        "echo 'REFUSE: the proxy is unreachable' >&2; exit 5",
        5,
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(got, Err(Cause::ProxyUnreachable));
}

#[tokio::test]
async fn a_run_past_its_turn_cap_delivers_nothing() {
    let body = r#"echo '{"subtype":"error_max_turns","is_error":true,"result":"partial"}'; exit 6"#;
    assert_eq!(
        run_script(body, 5, Duration::from_secs(5)).await,
        Err(Cause::TurnCap)
    );
}

#[tokio::test]
async fn a_run_past_its_budget_cap_delivers_nothing() {
    let body =
        r#"echo '{"subtype":"error_max_budget_usd","is_error":true,"result":"partial"}'; exit 6"#;
    assert_eq!(
        run_script(body, 5, Duration::from_secs(5)).await,
        Err(Cause::BudgetCap)
    );
}

#[tokio::test]
async fn a_run_past_its_wall_clock_delivers_nothing() {
    // The script's own refusal names the wall clock ...
    let body = "echo 'REFUSE: the run passed its wall clock' >&2; exit 6";
    assert_eq!(
        run_script(body, 5, Duration::from_secs(5)).await,
        Err(Cause::TimeCap)
    );
    // ... and a script that never returns is stopped from this side.
    assert_eq!(
        run_script("sleep 30", 1, Duration::from_millis(200)).await,
        Err(Cause::TimeCap)
    );
}

#[tokio::test]
async fn the_prompt_is_left_only_on_a_private_file_that_is_removed() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = script(
        dir.path(),
        "runner.sh",
        &format!(
            "stat -c %a \"$1\" > \"{0}/mode\"; echo '{OK_JSON}'",
            dir.path().display()
        ),
    );
    ProcessRunner::new(path, dir.path().to_path_buf())
        .run("secret prompt", &caps(5))
        .await
        .expect("a reply");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("mode"))
            .expect("a mode")
            .trim(),
        "600"
    );
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .expect("a listing")
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("prompt-"))
        .collect();
    assert!(left.is_empty(), "the prompt file is removed after the run");
}
