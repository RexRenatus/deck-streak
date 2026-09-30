//! A test's log capture cannot lose a line to another thread's cached callsite interest
//! (SPEC-024, the 2026-09-30 amendment; issue #461).
//!
//! The class: a test that captures log lines receives every line its code emits, whichever
//! thread first reached that line's callsite, and whatever the other tests in its binary did
//! first. `tracing` caches, per callsite, whether any dispatcher is interested. While at most one
//! dispatcher was registered at the last registration it asks only the reaching thread's default
//! about a callsite, and stores that answer without a lock, so a thread with no subscriber that
//! reaches a line first, or whose first registration of it straddles a capture's registration,
//! caches it as never enabled, and a capture made on another thread then never sees it.
//!
//! Two tests, each derived rather than listed. One runs each scenario in a child process of this
//! binary, whose dispatcher registry is empty at its start, so the loss does not depend on the
//! other tests' timing: another thread reaches the line before the capture, and another thread's
//! first registration of the line is in flight while the capture is made. The other walks
//! `crates/` and counts every capturing call, in every spelling.

// An integration test is test code: its helpers panic on a failed read.
#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{JoinHandle, ThreadId};

use tracing::callsite::{Callsite, Identifier};
use tracing::field::{Field, FieldSet};
use tracing::level_filters::LevelFilter;
use tracing::metadata::Kind;
use tracing::span::{Attributes, Record};
use tracing::subscriber::Interest;
use tracing::{Event, Id, Level, Metadata, Subscriber, Value};

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

/// A callsite whose `metadata()` holds the one thread armed for it, once, until released. The
/// registry calls `metadata()` after it chose its rebuilder (`JustOne` while at most one dispatcher
/// was registered at the last registration) and before it asks and stores the answer, so the hold
/// places a capture's registration exactly between the two.
struct Straddled {
    interest: AtomicU8,
}

static STRADDLED: Straddled = Straddled {
    interest: AtomicU8::new(0),
};

static STRADDLED_META: Metadata<'static> = Metadata::new(
    "straddled",
    "log_capture_class",
    Level::INFO,
    None,
    None,
    None,
    FieldSet::new(&[], Identifier(&STRADDLED)),
    Kind::EVENT,
);

/// The thread `STRADDLED` holds, the end it signals the hold on, and the end it waits on.
type Hold = (ThreadId, SyncSender<()>, Receiver<()>);

static HOLD: Mutex<Option<Hold>> = Mutex::new(None);

impl Callsite for Straddled {
    fn set_interest(&self, interest: Interest) {
        let value = if interest.is_never() {
            1
        } else if interest.is_always() {
            3
        } else {
            2
        };
        self.interest.store(value, Ordering::SeqCst);
    }

    fn metadata(&self) -> &Metadata<'_> {
        let mut hold = HOLD.lock().unwrap_or_else(PoisonError::into_inner);
        let armed = hold
            .as_ref()
            .is_some_and(|(thread, _, _)| *thread == std::thread::current().id());
        if armed {
            let (_, paused, release) = hold.take().expect("the armed hold");
            drop(hold);
            paused.send(()).expect("the scenario hears the hold");
            release.recv().expect("the scenario releases the hold");
        }
        &STRADDLED_META
    }
}

/// Emits one event through `STRADDLED` as the `tracing` macros do: the level gate, the cached
/// interest, then the current default's `enabled`.
fn reach_straddled() {
    if Level::INFO > LevelFilter::current() {
        return;
    }
    let enabled = match STRADDLED.interest.load(Ordering::SeqCst) {
        1 => false,
        3 => true,
        _ => tracing::dispatcher::get_default(|current| current.enabled(&STRADDLED_META)),
    };
    if enabled {
        let values: [(&Field, Option<&dyn Value>); 0] = [];
        Event::dispatch(&STRADDLED_META, &STRADDLED_META.fields().value_set(&values));
    }
}

/// Starts another thread's first registration of `STRADDLED` and returns once that thread is
/// held inside it, having chosen its rebuilder before any capture exists.
fn straddle() -> (SyncSender<()>, JoinHandle<()>) {
    let (held, on_held) = sync_channel(1);
    let (release, on_release) = sync_channel(1);
    let other = std::thread::spawn(move || {
        *HOLD.lock().unwrap_or_else(PoisonError::into_inner) =
            Some((std::thread::current().id(), held, on_release));
        tracing::callsite::register(&STRADDLED);
    });
    on_held
        .recv()
        .expect("the other thread is held inside its registration");
    (release, other)
}

/// Releases the held registration, waits until it stored its answer, and reaches the line.
fn finish((release, other): (SyncSender<()>, JoinHandle<()>)) {
    release.send(()).expect("the other thread is released");
    other.join().expect("the other thread registered the line");
    reach_straddled();
}

/// The scoped entry, made while another thread's registration of the line is in flight.
fn scoped_straddled(captured: &Captured) {
    let other = straddle();
    log_capture::with_capture(captured.clone(), || finish(other));
}

/// The held entry, made while another thread's registration of the line is in flight.
fn held_straddled(captured: &Captured) {
    let other = straddle();
    let _guard = log_capture::hold_capture(captured.clone());
    finish(other);
}

const SCENARIOS: [(&str, Scenario); 4] = [
    ("scoped", scoped),
    ("held", held),
    ("scoped-straddled", scoped_straddled),
    ("held-straddled", held_straddled),
];

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

/// Prints how many items a check examined and refuses zero: a walk that stopped matching must
/// fail, never pass over the empty set (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    eprintln!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Every `.rs` file under `dir`, skipping cargo's build output.
fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            // Cargo marks its build output with CACHEDIR.TAG; a source directory that happens to
            // be named `target` is still read.
            if path.file_name().is_some_and(|name| name == "target")
                && path.join("CACHEDIR.TAG").exists()
            {
                continue;
            }
            sources(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

/// The names that make a subscriber or a dispatcher a default, scoped or global, in `tracing`
/// (`subscriber::` and `dispatcher::` alike) and `tracing-subscriber` (`SubscriberInitExt` and
/// `fmt::init`), and the future adapter that installs one while it is polled. A path, a `use`, a
/// rename, a method call, a turbofish, a function value and a macro argument all spell one of
/// these as a token, so the census counts tokens rather than one path's text.
const INSTALLS: [&str; 6] = [
    "set_default",
    "with_default",
    "set_global_default",
    "init",
    "try_init",
    "with_subscriber",
];

/// The production global default: the one install the workspace keeps outside the helper.
const GLOBAL_DEFAULT: &str = "kernel/src/logging.rs";

/// This killer, excluded by its full path so a file of the same name elsewhere is still read.
const KILLER: &str = "kernel/tests/log_capture_class.rs";

/// A file's code with every comment, string literal and character literal blanked and every line
/// break kept, so a name inside a comment or a string never counts, and a `//` inside a string
/// hides nothing after it.
fn code(path: &Path) -> String {
    let text: Vec<char> = fs::read_to_string(path)
        .expect("a readable source")
        .chars()
        .collect();
    let mut code = String::with_capacity(text.len());
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        let len = if rest.starts_with(&['/', '/']) {
            rest.iter().position(|&c| c == '\n').unwrap_or(rest.len())
        } else if rest.starts_with(&['/', '*']) {
            block_comment(rest)
        } else if let Some(len) = raw_string(&text, at) {
            len
        } else if rest[0] == '"' {
            string(rest)
        } else if let Some(len) = character(rest) {
            len
        } else {
            code.push(rest[0]);
            at += 1;
            continue;
        };
        code.extend(
            rest[..len]
                .iter()
                .map(|&c| if c == '\n' { '\n' } else { ' ' }),
        );
        at += len;
    }
    code
}

/// The length of a block comment, nested ones included.
fn block_comment(rest: &[char]) -> usize {
    let (mut depth, mut at) = (1, 2);
    while at < rest.len() && depth > 0 {
        if rest[at..].starts_with(&['/', '*']) {
            depth += 1;
            at += 2;
        } else if rest[at..].starts_with(&['*', '/']) {
            depth -= 1;
            at += 2;
        } else {
            at += 1;
        }
    }
    at
}

/// The length of a raw string (`r"…"`, `r#"…"#`, `br"…"`) starting at `at`, if one does.
fn raw_string(text: &[char], at: usize) -> Option<usize> {
    let starts_word = at == 0 || !(text[at - 1].is_alphanumeric() || text[at - 1] == '_');
    let prefix = if text[at] == 'r' {
        1
    } else if text[at] == 'b' && text.get(at + 1) == Some(&'r') {
        2
    } else {
        return None;
    };
    if !starts_word {
        return None;
    }
    let hashes = text[at + prefix..]
        .iter()
        .take_while(|&&c| c == '#')
        .count();
    if text.get(at + prefix + hashes) != Some(&'"') {
        return None;
    }
    let body = at + prefix + hashes + 1;
    let mut end = body;
    while end < text.len() {
        if text[end] == '"'
            && text[end + 1..]
                .iter()
                .take(hashes)
                .filter(|&&c| c == '#')
                .count()
                == hashes
        {
            return Some(end + 1 + hashes - at);
        }
        end += 1;
    }
    Some(text.len() - at)
}

/// The length of a string literal starting at its opening quote.
fn string(rest: &[char]) -> usize {
    let mut at = 1;
    while at < rest.len() && rest[at] != '"' {
        at += if rest[at] == '\\' { 2 } else { 1 };
    }
    (at + 1).min(rest.len())
}

/// The length of a character literal starting at `'`, or `None` for a lifetime.
fn character(rest: &[char]) -> Option<usize> {
    if rest.first() != Some(&'\'') || rest.len() < 3 {
        return None;
    }
    if rest[1] == '\\' {
        let close = rest[3..].iter().position(|&c| c == '\'')?;
        return Some(close + 4);
    }
    (rest[2] == '\'').then_some(3)
}

/// Every install token in `code`, with its line.
fn installs(code: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    for (index, line) in code.lines().enumerate() {
        for token in line.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
            if INSTALLS.contains(&token) {
                found.push((index + 1, token));
            }
        }
    }
    found
}

#[test]
fn every_capture_in_the_workspace_goes_through_the_helper() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crates directory")
        .to_path_buf();
    let mut files = Vec::new();
    sources(&crates, &mut files);
    let files = examined("Rust source file(s) under crates/", files);

    let mut raw = Vec::new();
    let mut globals = Vec::new();
    let mut routed = 0;
    let mut read = 0;
    for path in &files {
        let name = path
            .strip_prefix(&crates)
            .expect("under crates")
            .display()
            .to_string();
        if name == KILLER {
            continue;
        }
        read += 1;
        let code = code(path);
        for (line, token) in installs(&code) {
            if token == "set_global_default" && name == GLOBAL_DEFAULT {
                globals.push(name.clone());
            } else {
                raw.push(format!("{name}:{line}: {token}"));
            }
        }
        let code: String = code.split_whitespace().collect();
        routed += code.matches("log_capture::with_capture(").count()
            + code.matches("log_capture::hold_capture(").count();
    }
    let mut report = String::new();
    let _ = write!(
        report,
        "capture population: {read} file(s) read; {} raw capture(s), {routed} routed, {} global default(s)",
        raw.len(),
        globals.len()
    );
    eprintln!("{report}");

    assert!(read > 100, "the walk read too few files: {report}");
    assert_eq!(
        globals,
        [GLOBAL_DEFAULT],
        "the one production global default is the only one: {report}"
    );
    assert_eq!(routed, 13, "{report}");
    assert!(
        raw.is_empty(),
        "{} capture(s) bypass the helper: {raw:#?}\n{report}",
        raw.len()
    );
}
