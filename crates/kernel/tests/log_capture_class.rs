//! A test's log capture cannot lose a line to another thread's cached callsite interest
//! (SPEC-024, the 2026-09-30 amendment; issue #461).
//!
//! The class: a test that captures log lines receives every line its code emits, whichever
//! thread first reached that line's callsite, and whatever the other tests in its binary did
//! first. `tracing` caches, per callsite, whether any dispatcher is interested. While one
//! dispatcher is registered it asks only the reaching thread's default about a callsite, so a
//! thread with no subscriber that reaches a line first caches it as never enabled, and a capture
//! made on another thread then never sees it.
//!
//! Two tests, each derived rather than listed. One runs the scenario in a child process of this
//! binary, whose dispatcher registry is empty at its start, so the loss does not depend on the
//! other tests' timing. The other walks `crates/` and counts every capturing call.

// An integration test is test code: its helpers panic on a failed read.
#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, PoisonError};

use tracing::span::{Attributes, Record};
use tracing::{Event, Id, Metadata, Subscriber};

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

/// The variable that makes this binary run one scenario in the child, and nothing else.
const CHILD: &str = "DECK_STREAK_LOG_CAPTURE_CHILD";

/// A subscriber that keeps one line per event.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<String>>>);

impl Captured {
    fn lines(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Subscriber for Captured {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event.metadata().target().to_owned());
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

/// The one callsite the scenario reaches, first on a thread with no subscriber.
fn reach() {
    tracing::info!(target: "log_capture_class", "reached");
}

/// How a scenario makes its capture: each entry the helper offers.
type Scenario = fn(&Captured);

/// The capture is made with the scoped entry: the body runs inside it.
fn scoped(captured: &Captured) {
    log_capture::with_capture(captured.clone(), || {
        std::thread::spawn(reach)
            .join()
            .expect("the other thread reached the line");
        reach();
    });
}

/// The capture is made with the held entry: the guard lives across the body.
fn held(captured: &Captured) {
    let _guard = log_capture::hold_capture(captured.clone());
    std::thread::spawn(reach)
        .join()
        .expect("the other thread reached the line");
    reach();
}

const SCENARIOS: [(&str, Scenario); 2] = [("scoped", scoped), ("held", held)];

#[test]
fn a_capture_keeps_a_line_another_thread_reached_first() {
    if let Some(name) = std::env::var_os(CHILD) {
        let name = name.to_string_lossy().into_owned();
        let (_, scenario) = SCENARIOS
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .expect("a scenario the child was asked for");
        let captured = Captured::default();
        scenario(&captured);
        let lines = captured.lines();
        assert_eq!(
            lines,
            ["log_capture_class"],
            "scenario {name}: the capture lost the line another thread reached first"
        );
        return;
    }

    let exe = std::env::current_exe().expect("this test binary's path");
    let mut lost = Vec::new();
    for (name, _) in SCENARIOS {
        let output = Command::new(&exe)
            .args([
                "--exact",
                "a_capture_keeps_a_line_another_thread_reached_first",
                "--test-threads=1",
            ])
            .env(CHILD, name)
            .output()
            .expect("the child run");
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(
            text.contains("1 passed") || !output.status.success(),
            "the child of scenario {name} ran no test: {text}"
        );
        if !output.status.success() {
            lost.push(name);
        }
    }
    eprintln!(
        "log capture scenarios: {} run, {} lost the line",
        SCENARIOS.len(),
        lost.len()
    );
    assert!(
        lost.is_empty(),
        "{} of {} scenarios lost a line another thread reached first: {lost:?}",
        lost.len(),
        SCENARIOS.len()
    );
}

/// Every `.rs` file under `dir`, skipping build output.
fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

/// A file's code with line comments and all whitespace removed, so a call split across lines by
/// the formatter is still one needle.
fn code(path: &Path) -> String {
    let text = fs::read_to_string(path).expect("a readable source");
    let mut code = String::new();
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or_default();
        code.extend(line.split_whitespace());
    }
    code
}

#[test]
fn every_capture_in_the_workspace_goes_through_the_helper() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crates directory")
        .to_path_buf();
    let mut files = Vec::new();
    sources(&crates, &mut files);
    files.retain(|path| {
        path.file_name()
            .is_some_and(|name| name != "log_capture_class.rs")
    });

    let scoped = ["subscriber::with_default(", "subscriber::set_default("];
    let global = "subscriber::set_global_default(";
    let mut raw = Vec::new();
    let mut globals = Vec::new();
    let mut routed = 0;
    for path in &files {
        let code = code(path);
        let name = path
            .strip_prefix(&crates)
            .expect("under crates")
            .display()
            .to_string();
        for needle in scoped {
            for _ in code.matches(needle) {
                raw.push(name.clone());
            }
        }
        for _ in code.matches(global) {
            globals.push(name.clone());
        }
        routed += code.matches("log_capture::with_capture(").count()
            + code.matches("log_capture::hold_capture(").count();
    }
    let mut report = String::new();
    let _ = write!(
        report,
        "capture population: {} file(s) read; {} raw capture(s), {} routed, {} global default(s)",
        files.len(),
        raw.len(),
        routed,
        globals.len()
    );
    eprintln!("{report}");

    assert!(files.len() > 100, "the walk read too few files: {report}");
    assert_eq!(
        globals,
        ["kernel/src/logging.rs"],
        "the one production global default is the only one: {report}"
    );
    assert_eq!(routed, 13, "{report}");
    assert!(
        raw.is_empty(),
        "{} capture(s) bypass the helper: {raw:#?}\n{report}",
        raw.len()
    );
}
