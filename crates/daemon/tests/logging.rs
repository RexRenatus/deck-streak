//! Every role of `deckstreakd` installs the kernel's logging before it reads a setting, so every line
//! it writes, from its first, is a JSON event that opens with its journal priority, a refusal to
//! start included (SPEC-031 A1, R1).
//!
//! The roles and the jobs are read from the binary's own usage line, so a role added later is judged
//! without a change here. Each runs with an empty environment, so each refuses start on the first
//! setting it reads: the earliest line a role can write.

// An integration test is test code: its helpers panic on a failed child, and the examined count is
// printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::process::{Command, Output};

use serde_json::Value;

/// What a refused start says of the setting it did not find (SPEC-020's settings errors).
const MISSING_SETTING: &str = "is required and is not set";

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Runs the built binary with `arguments` and an empty environment.
fn deckstreakd(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deckstreakd"))
        .args(arguments)
        .env_clear()
        .output()
        .expect("the binary runs")
}

fn describe(output: &Output) -> String {
    format!(
        "{}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The sd-daemon(3) priority journald files a line of `level` at.
fn priority(level: &str) -> Option<&'static str> {
    match level {
        "ERROR" => Some("<3>"),
        "WARN" => Some("<4>"),
        "INFO" => Some("<6>"),
        "DEBUG" | "TRACE" => Some("<7>"),
        _ => None,
    }
}

/// Every line `output` wrote, each checked to be a JSON event that opens with its level's priority,
/// and nothing written to stderr, where no subscriber writes.
fn events(what: &str, output: &Output) -> Vec<Value> {
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8");
    let mut events = Vec::new();
    for line in stdout.lines() {
        let (prefix, json) = line.split_at(line.find('{').unwrap_or(0));
        let event: Value = serde_json::from_str(json)
            .unwrap_or_else(|_| panic!("{what}: a line is not a JSON event: {line}"));
        let level = event["level"].as_str().unwrap_or_default();
        assert_eq!(
            Some(prefix),
            priority(level),
            "{what}: a line does not open with its level's priority: {line}"
        );
        events.push(event);
    }
    assert!(
        output.stderr.is_empty(),
        "{what}: wrote to stderr, past the subscriber: {}",
        describe(output)
    );
    assert!(
        !events.is_empty(),
        "{what}: wrote no line: {}",
        describe(output)
    );
    events
}

/// A list the usage line names after `label`, up to the next `;`.
fn named(message: &str, label: &str) -> Vec<String> {
    let Some(at) = message.find(label) else {
        panic!("the usage line names no `{label}`: {message}");
    };
    message[at + label.len()..]
        .split(';')
        .next()
        .unwrap_or_default()
        .split(',')
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn every_role_logs_json_with_its_priority_from_its_first_line() {
    // With no role the binary refuses with its usage line, and that refusal is its first event.
    let usage = deckstreakd(&[]);
    assert_eq!(usage.status.code(), Some(2), "{}", describe(&usage));
    let first = events("no role", &usage).remove(0);
    assert_eq!(first["level"], "ERROR", "{first}");
    let message = first["message"]
        .as_str()
        .expect("a usage message")
        .to_owned();
    let roles = named(&message, "the roles are: ");
    let jobs = named(&message, "the jobs are: ");

    // Every role, and for the job role every job of the table, each run with no setting at all.
    let mut invocations = Vec::new();
    for role in examined("role(s) the usage line names", roles) {
        if role == "job" {
            for job in examined("job(s) the usage line names", jobs.clone()) {
                invocations.push(vec![role.clone(), job]);
            }
        } else if role == "data" {
            // The data role takes a command; `export` reads the settings every role reads first.
            invocations.push(vec![role, "export".to_owned()]);
        } else if role == "preset" {
            // The preset role takes a command; `list` reads the settings every role reads first.
            invocations.push(vec![role, "list".to_owned()]);
        } else {
            invocations.push(vec![role]);
        }
    }
    for invocation in examined("role invocation(s)", invocations) {
        let arguments: Vec<&str> = invocation.iter().map(String::as_str).collect();
        let what = arguments.join(" ");
        let output = deckstreakd(&arguments);
        // A refused start, not a usage error: the role ran, and read a setting it did not find.
        assert_eq!(
            output.status.code(),
            Some(1),
            "{what}: {}",
            describe(&output)
        );
        let first = events(&what, &output).remove(0);
        assert_eq!(first["level"], "ERROR", "{what}: {first}");
        assert!(
            first.to_string().contains(MISSING_SETTING),
            "{what}: the first line does not name the setting it refused on: {first}"
        );
    }
}
