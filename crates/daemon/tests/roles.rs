//! The binary runs a role by the name its first argument gives and refuses any other with code 2,
//! and two roles that open one fresh database at the same moment both start (SPEC-025 A14, A15,
//! R1, R11).

// An integration test is test code: its helpers panic on a failed child, and the examined count
// is printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::ffi::OsStr;
use std::process::{Command, Output};
use std::sync::Arc;

use deck_streak_daemon::wiring::{StateDirectory, open_database};
use deck_streak_kernel::{Offload, OffloadWorkers, SystemClock};
use serde_json::Value;
use tokio::sync::Barrier;

/// Fresh databases the race test opens, two roles at a time. Opened with no lock, as at the
/// red-first commit, one role failed with `SQLITE_BUSY` in 29 of 30 runs of 24 rounds, as late as
/// the fifteenth round, and one run passed all 24; 64 rounds leave a regression no real chance.
const ROUNDS: usize = 64;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Runs the built binary with `arguments` and only the variables of `environment`.
fn deckstreakd(arguments: &[&str], environment: &[(&str, &OsStr)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deckstreakd"))
        .args(arguments)
        .env_clear()
        .envs(environment.iter().copied())
        .output()
        .expect("the binary runs")
}

/// Every line the binary wrote to stdout, as its journal priority and its JSON event; a line that
/// is not one is returned with no priority and `Null`.
fn events(output: &Output) -> Vec<(Option<String>, Value)> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| match line.find('{') {
            Some(3) => (
                Some(line[..3].to_owned()),
                serde_json::from_str(&line[3..]).unwrap_or(Value::Null),
            ),
            _ => (None, Value::Null),
        })
        .collect()
}

fn describe(output: &Output) -> String {
    format!(
        "{}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn the_binary_runs_a_role_by_name_and_refuses_an_unknown_one() {
    // An unknown role, no role, and a known one with an argument it does not take: each exits 2,
    // and its first line is an ERROR event whose usage names every role the binary knows.
    let refusals: [&[&str]; 3] = [&["frobnicate"], &[], &["api", "extra"]];
    for arguments in examined("refused invocation(s)", refusals.to_vec()) {
        let output = deckstreakd(arguments, &[]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{arguments:?}: {}",
            describe(&output)
        );
        let lines = events(&output);
        let first = lines.first().cloned().unwrap_or((None, Value::Null));
        assert_eq!(
            first.0.as_deref(),
            Some("<3>"),
            "{arguments:?}: {}",
            describe(&output)
        );
        let usage = first.1["message"].as_str().unwrap_or_default().to_owned();
        assert!(usage.starts_with("usage: deckstreakd <role>"), "{usage}");
        assert!(usage.ends_with("the roles are: api"), "{usage}");
    }

    // A known role runs: the api role refuses to start without its listen address, naming the
    // setting from its first line, as a JSON event with its priority, and exits 1, not 2.
    let directory = tempfile::tempdir().expect("a temporary directory");
    let output = deckstreakd(
        &["api"],
        &[("STATE_DIRECTORY", directory.path().as_os_str())],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let lines = events(&output);
    let first = lines.first().cloned().unwrap_or((None, Value::Null));
    assert_eq!(first.0.as_deref(), Some("<3>"), "{}", describe(&output));
    let refusal = first.1.to_string();
    assert!(
        refusal.contains("DECKSTREAK_API_LISTEN"),
        "the refusal does not name the setting: {refusal}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_roles_opening_one_fresh_database_at_once_both_start() {
    for round in examined("round(s)", (1..=ROUNDS).collect()) {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let state = StateDirectory::new(directory.path()).expect("an absolute path");
        let workers = OffloadWorkers::new(2).expect("two workers is in range");
        let offload = Offload::new(workers, Arc::new(SystemClock));
        // Both roles start opening at the same moment, as two units systemd starts together.
        let together = Arc::new(Barrier::new(2));
        let roles: Vec<_> = (0..2)
            .map(|_| {
                let (state, offload, together) =
                    (state.clone(), offload.clone(), Arc::clone(&together));
                tokio::spawn(async move {
                    together.wait().await;
                    open_database(&offload, &state).await
                })
            })
            .collect();
        for (role, handle) in roles.into_iter().enumerate() {
            let opened = handle.await.expect("the role's task completes");
            assert!(
                opened.is_ok(),
                "round {round}: role {role} failed to start: {opened:?}"
            );
            let database = opened.expect("the database opened");
            // Each role reads the kernel's table: the migrations were applied, once.
            let generation = database.settings_generation().await;
            assert_eq!(generation.ok(), Some(0), "round {round}: role {role}");
            database.close().await;
        }
    }
}
