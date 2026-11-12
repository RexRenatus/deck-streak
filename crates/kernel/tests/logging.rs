//! Every event leaves the process as one JSON line that opens with its journal priority, with each
//! registered secret and every token shape replaced by the marker the predecessor's filter writes
//! (SPEC-020 A13 to A15, R12, R13), and so does a panic's message, which the logging's panic hook
//! logs as one event instead of the default hook's plain text on stderr (SPEC-031 A8, R1).
//!
//! What leaves the process is measured on a child process: this test binary run again, running
//! only an ignored test that installs the kernel's logging and emits events, so the global
//! subscriber, the panic hook and the real stdout and stderr are the ones a role of the daemon
//! would have.

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
/// A credential's synthetic value that a panic's message carries.
const PANICKED: &str = "amber-kettle-signal";
/// The name of the thread the panicking child panics on.
const PANICKING_THREAD: &str = "probe-a8";

/// Runs this test binary again, running only the ignored test `child` with `RUST_LOG` at `level`,
/// checks that it passed, and returns what it wrote to stdout and to stderr.
fn run_child(child: &str, level: &str) -> (String, String) {
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
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "the child {child} failed:\n{stdout}\n{stderr}"
    );
    (stdout, stderr)
}

/// Runs this test binary again, running only the ignored test `child` with `RUST_LOG` at `level`,
/// and returns every event line it wrote to stdout.
fn events_of(child: &str, level: &str) -> Vec<String> {
    event_lines(&run_child(child, level).0)
}

/// Every event line of a child's stdout. The test harness prints its own lines around the child's;
/// an event is a JSON object, with its priority prefix when it has one.
fn event_lines(stdout: &str) -> Vec<String> {
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
    // The probe's levels the build under test compiles in, most severe first: the build's own
    // static maximum decides it, so the shipped build (a dependency's `release_max_level_debug`
    // compiles `trace!` out of it, ADR-330) is held to exactly what it can emit, and the dev
    // build still expects all five.
    let static_max = tracing::level_filters::STATIC_MAX_LEVEL;
    let expected: Vec<&str> = [
        (tracing::Level::ERROR, "ERROR"),
        (tracing::Level::WARN, "WARN"),
        (tracing::Level::INFO, "INFO"),
        (tracing::Level::DEBUG, "DEBUG"),
        (tracing::Level::TRACE, "TRACE"),
    ]
    .into_iter()
    .filter(|(level, _)| *level <= static_max)
    .map(|(_, name)| name)
    .collect();
    assert!(
        expected.len() >= 4,
        "the build compiles out more than TRACE: {static_max}"
    );
    assert_eq!(levels, expected, "{lines:#?}");
    assert_eq!(
        levels.iter().any(|level| level == "TRACE"),
        tracing::Level::TRACE <= static_max,
        "the probe's TRACE line is present exactly when the build compiles TRACE in"
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

#[test]
#[ignore = "a child process: a_panic_logs_one_redacted_json_event_and_never_the_value runs it"]
fn child_panics_with_a_registered_secret() {
    let redactor = Redactor::new();
    logging::install(&redactor).expect("the first install in this process");
    redactor.register(PANICKED);
    // A thread of its own panics, as a role's task would, so the child itself passes and its parent
    // reads everything the panic left on either stream.
    let joined = std::thread::Builder::new()
        .name(PANICKING_THREAD.to_owned())
        .spawn(|| panic!("the credential {PANICKED} reached a panic"))
        .expect("the thread starts")
        .join();
    assert!(joined.is_err(), "the thread did not panic");
}

#[test]
fn a_panic_logs_one_redacted_json_event_and_never_the_value() {
    let (stdout, stderr) = run_child("child_panics_with_a_registered_secret", "info");
    let lines = event_lines(&stdout);
    let panics: Vec<(Option<&str>, Value)> = lines
        .iter()
        .map(|line| parse(line))
        .filter(|(_, event)| event["thread"] == PANICKING_THREAD)
        .collect();
    assert_eq!(
        panics.len(),
        1,
        "one event must log the panic:\n{stdout}\nstderr:\n{stderr}"
    );
    let (prefix, event) = &panics[0];
    // An ERROR event, which journald files at priority 3, so the alert quotes it.
    assert_eq!(*prefix, Some("<3>"), "{event}");
    assert_eq!(event["level"], "ERROR", "{event}");
    assert_eq!(
        event["panic"],
        format!("the credential {REDACTED} reached a panic"),
        "{event}"
    );
    assert!(
        event["location"]
            .as_str()
            .is_some_and(|location| location.contains("logging.rs")),
        "the event names no location: {event}"
    );
    // The value never leaves the process, on either stream, and nothing reaches stderr, where no
    // redactor reads: the default hook's plain text is replaced, never chained.
    for text in [&stdout, &stderr] {
        assert!(
            !text.contains(PANICKED),
            "a secret left the process:\n{text}"
        );
    }
    assert!(
        !stderr.contains("panicked at"),
        "the default panic hook wrote to stderr:\n{stderr}"
    );
}

/// The `RUST_LOG` values the passkey library's silence is measured under (SPEC-359 R13): everything
/// at `trace`; each of the library's crates named at `trace`; a module under each named at `trace`,
/// a longer target, which `EnvFilter` ranks above a shorter one; and a span the events sit in named
/// at `trace`, which `EnvFilter` reads before any target. Each keeps `info` for the controls.
const SILENCE_FILTERS: [&str; 5] = [
    "trace",
    "info,webauthn_rs_core=trace",
    "info,webauthn_rs=trace",
    "info,webauthn_rs_core::core=trace,webauthn_rs::interface=trace",
    "info,[ceremony]=trace",
];

#[test]
#[ignore = "a child process: the_passkey_library_writes_nothing_whatever_rust_log_says runs it"]
fn child_logs_under_the_passkey_librarys_targets() {
    logging::install(&Redactor::new()).expect("the first install in this process");
    let ceremony = tracing::info_span!("ceremony");
    let _entered = ceremony.enter();
    tracing::info!(target: "logging_control", probe = "r13", "the positive control");
    tracing::info!(target: "webauthn_rsx", probe = "r13", "a crate the silence does not name");
    tracing::error!(target: "webauthn_rs", probe = "r13", "the library's root");
    tracing::error!(target: "webauthn_rs::interface", probe = "r13", "a module of the library");
    tracing::error!(target: "webauthn_rs_core", probe = "r13", "the core's root");
    tracing::error!(target: "webauthn_rs_core::core", probe = "r13", "a module of the core");
}

/// SPEC-359 R13: an `error` event under the passkey library's targets writes nothing, whatever
/// `RUST_LOG` says, while a control at `info` under another target writes its line, and so does a
/// target that only begins with the library's name.
#[test]
fn the_passkey_library_writes_nothing_whatever_rust_log_says() {
    println!("examined {} RUST_LOG value(s)", SILENCE_FILTERS.len());
    for filter in SILENCE_FILTERS {
        let lines = events_of("child_logs_under_the_passkey_librarys_targets", filter);
        let targets: Vec<String> = probed(&lines, "r13")
            .iter()
            .map(|event| event["target"].as_str().unwrap_or("no target").to_owned())
            .collect();
        assert_eq!(
            targets,
            ["logging_control", "webauthn_rsx"],
            "under RUST_LOG={filter}: {lines:#?}"
        );
    }
}
