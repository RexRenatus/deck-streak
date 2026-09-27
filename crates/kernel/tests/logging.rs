//! Every event leaves the process as one JSON line that opens with its journal priority, with each
//! registered secret and every token shape replaced by the marker the predecessor's filter writes
//! (SPEC-020 A13 to A15, R12, R13).
//!
//! What leaves the process is measured on a child process: this test binary run again, running
//! only an ignored test that installs the kernel's logging and emits events, so the global
//! subscriber and the real stdout are the ones a role of the daemon would have.

// An integration test is test code: its helpers panic on a failed child, and the golden reader
// prints the examined count on purpose. clippy.toml's in-test allowances cover `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeSet;
use std::process::Command;

use deck_streak_kernel::redact::{self, REDACTED};
use deck_streak_kernel::{Redactor, logging};
use serde_json::Value;

/// A credential's synthetic value, and one holding what a JSON string escapes.
const PLAIN: &str = "velvet-orchid-lantern";
const QUOTED: &str = "quote\"and\\slash";
/// A synthetic token of the predecessor's shape; seven digits, so never the public scrub's shape.
const TOKEN: &str = "4815162:abcdefghijklmnopqrstuvwxyzABCDEF";

/// Runs this test binary again, running only the ignored test `child` with `RUST_LOG` at `level`,
/// and returns every event line it wrote to stdout.
fn events_of(child: &str, level: &str) -> Vec<String> {
    let output = Command::new(std::env::current_exe().expect("this test binary's path"))
        .args([
            "--exact",
            child,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("RUST_LOG", level)
        .output()
        .expect("the child process runs");
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(
        output.status.success(),
        "the child {child} failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The test harness prints its own lines around the child's; an event is a JSON object, with
    // its priority prefix when it has one.
    stdout
        .lines()
        .filter_map(|line| {
            let brace = line.find('{')?;
            let start = line[..brace]
                .rfind('<')
                .filter(|&at| brace - at == 3)
                .unwrap_or(brace);
            Some(line[start..].to_owned())
        })
        .collect()
}

/// An event line's priority prefix, if it opens with one, and its JSON object.
fn parse(line: &str) -> (Option<&str>, Value) {
    let (prefix, json) = match line.find('{') {
        Some(3) => (Some(&line[..3]), &line[3..]),
        _ => (None, line),
    };
    (
        prefix,
        serde_json::from_str(json).unwrap_or_else(|_| panic!("not a JSON event: {line}")),
    )
}

/// The events of `lines` a child emitted for `probe`.
fn probed(lines: &[String], probe: &str) -> Vec<Value> {
    lines
        .iter()
        .map(|line| parse(line).1)
        .filter(|event| event["probe"] == probe)
        .collect()
}

#[test]
#[ignore = "a child process: a_logged_secret_leaves_the_process_as_the_redaction_marker runs it"]
fn child_logs_registered_secrets() {
    let redactor = Redactor::new();
    logging::install(&redactor).expect("the first install in this process");
    // Registered after the logging was installed, as the credential loader registers a secret it
    // reads during start: the writer reads the registry live.
    redactor.register(PLAIN);
    redactor.register(QUOTED);
    tracing::info!(probe = "a13", value = PLAIN, "a field carries a secret");
    tracing::info!(probe = "a13", "the message carries {PLAIN}");
    tracing::info!(
        probe = "a13",
        value = QUOTED,
        "a field carries a secret JSON escapes"
    );
    tracing::info!(
        probe = "a13",
        value = TOKEN,
        "a field carries a token nobody registered"
    );
}

#[test]
fn a_logged_secret_leaves_the_process_as_the_redaction_marker() {
    let lines = events_of("child_logs_registered_secrets", "info");
    let events = probed(&lines, "a13");
    assert_eq!(events.len(), 4, "{lines:#?}");
    assert_eq!(events[0]["value"], REDACTED);
    assert_eq!(
        events[1]["message"],
        format!("the message carries {REDACTED}")
    );
    assert_eq!(events[2]["value"], REDACTED);
    assert_eq!(events[3]["value"], REDACTED);
    // No line carries a secret, as written or as JSON escapes it.
    let escaped = serde_json::to_string(QUOTED).expect("a JSON string");
    for line in &lines {
        for secret in [PLAIN, QUOTED, &escaped[1..escaped.len() - 1], TOKEN] {
            assert!(!line.contains(secret), "a secret left the process: {line}");
        }
    }
}

#[test]
fn the_redaction_matches_the_predecessors_golden() {
    let mut classes = BTreeSet::new();
    golden::each_case("redaction", |case| {
        let redactor = Redactor::new();
        for secret in case.input["secrets"].as_array().expect("a list of secrets") {
            redactor.register(secret.as_str().expect("a secret is text"));
        }
        let text = case.input["text"].as_str().expect("a text");
        assert_eq!(
            Some(redactor.redact(text).as_str()),
            case.output.as_str(),
            "{}",
            case.input
        );
        classes.extend(case.class.clone());
    });
    for class in [
        "overlap",
        "short",
        "marker",
        "token",
        "decimal-run",
        "numeric-not-decimal",
    ] {
        assert!(classes.contains(class), "no {class} case was examined");
    }
}

#[test]
fn the_redactor_reads_exactly_the_predecessors_decimal_digits() {
    // The golden holds one token per run of ten decimal digits, its id the run's digits in order,
    // and the predecessor redacted every one of them.
    let mut digits = BTreeSet::new();
    golden::each_case("redaction", |case| {
        if case.class.as_deref() == Some("decimal-run") {
            let text = case.input["text"].as_str().expect("a text");
            let (id, _) = text.split_once(':').expect("a token");
            assert_eq!(id.chars().count(), 10, "a run of ten digits: {id}");
            digits.extend(id.chars());
        }
    });
    assert!(!digits.is_empty(), "the golden holds no run of digits");
    // Every character is a digit to the port exactly when it is one of those.
    let mut examined = 0_u32;
    for character in (0..=0x10_FFFF).filter_map(char::from_u32) {
        assert_eq!(
            redact::is_decimal_digit(character),
            digits.contains(&character),
            "{character:?} (U+{:04X})",
            u32::from(character)
        );
        examined += 1;
    }
    println!(
        "examined {examined} character(s), {} decimal digit(s)",
        digits.len()
    );
}

#[test]
#[ignore = "a child process: each_json_line_opens_with_its_journal_priority runs it"]
fn child_logs_at_every_level() {
    logging::install(&Redactor::new()).expect("the first install in this process");
    tracing::error!(probe = "a15", "an error");
    tracing::warn!(probe = "a15", "a warning");
    tracing::info!(probe = "a15", "a notice");
    tracing::debug!(probe = "a15", "a detail");
    tracing::trace!(probe = "a15", "a trace");
}

#[test]
fn each_json_line_opens_with_its_journal_priority() {
    let lines = events_of("child_logs_at_every_level", "trace");
    let mut levels = Vec::new();
    for line in &lines {
        let (prefix, event) = parse(line);
        if event["probe"] != "a15" {
            continue;
        }
        let level = event["level"].as_str().expect("a level").to_owned();
        let priority = match level.as_str() {
            "ERROR" => "<3>",
            "WARN" => "<4>",
            "INFO" => "<6>",
            _ => "<7>",
        };
        assert_eq!(prefix, Some(priority), "{line}");
        levels.push(level);
    }
    assert_eq!(
        levels,
        ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"],
        "{lines:#?}"
    );
}

#[test]
#[ignore = "a child process: a_second_install_returns_an_error runs it"]
fn child_installs_twice() {
    let redactor = Redactor::new();
    logging::install(&redactor).expect("the first install in this process");
    let second = logging::install(&redactor);
    tracing::info!(
        probe = "second",
        refused = second.is_err(),
        "installed twice"
    );
}

#[test]
fn a_second_install_returns_an_error() {
    let lines = events_of("child_installs_twice", "info");
    let events = probed(&lines, "second");
    assert_eq!(events.len(), 1, "{lines:#?}");
    assert_eq!(events[0]["refused"], true);
}
