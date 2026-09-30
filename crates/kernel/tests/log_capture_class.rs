//! A test's log capture cannot lose a line to another thread's cached callsite interest
//! (SPEC-024, the 2026-09-30 amendment; issue #461).
//!
//! The class: a test that captures log lines receives every line its thread emits outside a
//! dispatcher's own call, whichever thread first reached that line's callsite and however that
//! thread's registration of it interleaves with the capture's, in a binary where nothing but the
//! helper registers a dispatcher or a callsite.
//! `tracing` caches, per callsite, whether any dispatcher is interested. While at most one
//! dispatcher was registered at the last registration it asks only the reaching thread's default
//! about a callsite, and stores that answer without a lock, so a thread with no subscriber that
//! reaches a line first, or whose first registration of it straddles a capture's registration,
//! caches it as never enabled, and a capture made on another thread then never sees it.
//!
//! Two tests, each derived rather than listed. One runs each scenario in a child process of this
//! binary, whose dispatcher registry is empty at its start, so the loss does not depend on the
//! other tests' timing: another thread reaches the line before the capture, and another thread's
//! first registration of the line is in flight while the capture is made. Each child also checks
//! that the floor was the global default before its capture registered: a capture that registers
//! first leaves a window in which another thread's registration asks no default, answers `never`
//! and stores it after the capture's answer, an order no scenario here needs to reach. The other
//! walks `crates/`, `tools/` and every file they bring in as code, doctests included, and counts
//! every name that installs a subscriber or registers a dispatcher or a callsite, in every
//! spelling.

// An integration test is test code: its helpers panic on a failed read.
#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{JoinHandle, ThreadId};

use tracing::callsite::{Callsite, Identifier};
use tracing::field::{Field, FieldSet};
use tracing::level_filters::LevelFilter;
use tracing::metadata::Kind;
use tracing::span::{Attributes, Record};
use tracing::subscriber::{Interest, NoSubscriber};
use tracing::{Event, Id, Level, Metadata, Subscriber, Value};

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

/// The variable that makes this binary run one scenario in the child, and nothing else.
const CHILD: &str = "DECK_STREAK_LOG_CAPTURE_CHILD";

/// A subscriber that keeps one line per event, and counts how often the registry asked its level
/// hint while no global default was set.
#[derive(Clone, Default)]
struct Captured {
    lines: Arc<Mutex<Vec<String>>>,
    asked: Arc<AtomicUsize>,
    before_the_floor: Arc<AtomicUsize>,
}

impl Captured {
    fn lines(&self) -> Vec<String> {
        self.lines
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// How often the registry asked this capture's hint, and how many of those asks found no
    /// global default yet.
    fn asks(&self) -> (usize, usize) {
        (
            self.asked.load(Ordering::SeqCst),
            self.before_the_floor.load(Ordering::SeqCst),
        )
    }
}

impl Subscriber for Captured {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    /// The registry asks every live dispatcher's hint whenever a dispatcher registers, this one's
    /// own registration included, so the first ask is made while this capture registers. An ask
    /// that finds no global default is a capture that registered before the floor, the order in
    /// which the floor cannot keep a line: another thread's registration made during the capture's
    /// can still ask no default, answer `never` and store it after the capture's answer. A thread
    /// of the ask's own has no default of its own, so it reaches the global default or none; a
    /// thread default anywhere else does not count as the floor.
    fn max_level_hint(&self) -> Option<LevelFilter> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        let global = std::thread::scope(|scope| {
            scope
                .spawn(|| tracing::dispatcher::get_default(|current| !current.is::<NoSubscriber>()))
                .join()
                .expect("the thread that reads the global default")
        });
        if !global {
            self.before_the_floor.fetch_add(1, Ordering::SeqCst);
        }
        None
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        self.lines
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
        let (asked, before_the_floor) = captured.asks();
        assert_eq!(
            lines,
            ["log_capture_class"],
            "scenario {name}: the capture lost the line another thread reached first"
        );
        assert!(
            asked > 0 && before_the_floor == 0,
            "scenario {name}: the capture registered before the floor was the global default \
             ({before_the_floor} of {asked} registry asks found no global default)"
        );
        return;
    }

    let exe = std::env::current_exe().expect("this test binary's path");
    let mut failed = Vec::new();
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
            let marker = format!("scenario {name}:");
            let why = text
                .lines()
                .find(|line| line.contains(&marker))
                .unwrap_or("the child failed")
                .trim()
                .to_owned();
            failed.push(why);
        }
    }
    eprintln!(
        "log capture scenarios: {} run, {} failed",
        SCENARIOS.len(),
        failed.len()
    );
    assert!(
        failed.is_empty(),
        "{} of {} scenarios failed: {failed:#?}",
        failed.len(),
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

/// Every `.rs` file under `dir`. Nothing is skipped: the workspace builds into the root `target/`,
/// never under `crates/` or `tools/`, so every directory here is source, whatever it is named and
/// whatever marker file it holds.
fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
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

/// The names that register a dispatcher or a callsite, or rebuild the registry's cached answers,
/// outside an install. The floor holds only for an answer computed after it is the global default:
/// an answer another thread computed before it, with no default of its own, is `never`, and it can
/// be stored after a capture registered. Nothing registers before the floor while nothing but the
/// helper names these, because the global level filter stays `OFF` until the first capture
/// registers, and no macro registers a callsite while it is `OFF`.
const REGISTERS: [&str; 4] = [
    "Dispatch",
    "callsite",
    "DefaultCallsite",
    "rebuild_interest_cache",
];

/// Names that install out of the census's sight: macros that build an identifier from pieces, so a
/// call they spell is no token, and the attributes of crates that install a subscriber for a test.
const HIDDEN: [&str; 5] = [
    "paste",
    "pastey",
    "concat_idents",
    "traced_test",
    "test_log",
];

/// The production global default: the one install the workspace keeps outside the helper.
const GLOBAL_DEFAULT: &str = "crates/kernel/src/logging.rs";

/// The helper itself: the one file that installs and registers for every capture.
const HELPER: &str = "tools/log-capture/capture.rs";

/// This killer. It is read like every other file, and it alone may name the callsite registry: its
/// straddled scenarios register a callsite by hand, each in a child process of its own.
const KILLER: &str = "crates/kernel/tests/log_capture_class.rs";

/// A file's code with every comment, string literal and character literal blanked and every
/// character's position kept, so a name inside a comment or a string never counts, and a `//`
/// inside a string hides nothing after it.
fn code(text: &[char]) -> Vec<char> {
    let mut code = Vec::with_capacity(text.len());
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        let len = if rest.starts_with(&['/', '/']) {
            rest.iter().position(|&c| c == '\n').unwrap_or(rest.len())
        } else if rest.starts_with(&['/', '*']) {
            block_comment(rest)
        } else if let Some(len) = raw_string(text, at) {
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

/// The length of a raw string (`r"…"`, `r#"…"#`, `br"…"`, `cr"…"`) starting at `at`, if one does.
fn raw_string(text: &[char], at: usize) -> Option<usize> {
    let starts_word = at == 0 || !(text[at - 1].is_alphanumeric() || text[at - 1] == '_');
    let prefix = if text[at] == 'r' {
        1
    } else if matches!(text[at], 'b' | 'c') && text.get(at + 1) == Some(&'r') {
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

/// The code rustdoc runs as doctests: every line inside a fenced block of a doc comment (`///`,
/// `//!`, `/** */`, `/*! */`) whose info string names no other language, placed at its own line
/// so a token's line is the file's line.
fn doctests(text: &str) -> String {
    let mut out = String::new();
    // `None` outside a fence; `Some(true)` inside a Rust fence, `Some(false)` inside another one.
    let mut fence: Option<bool> = None;
    let mut in_block_doc = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let doc = if let Some(rest) = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"))
        {
            Some(rest)
        } else if in_block_doc || trimmed.starts_with("/**") || trimmed.starts_with("/*!") {
            in_block_doc = !trimmed.contains("*/");
            Some(trimmed.trim_start_matches("/**").trim_start_matches("/*!"))
        } else {
            None
        };
        let mut kept = "";
        if let Some(doc) = doc {
            let doc = doc.trim_start().trim_start_matches('*').trim_start();
            if let Some(info) = doc.strip_prefix("```") {
                fence = match fence {
                    Some(_) => None,
                    None => Some(
                        info.split(',')
                            .map(str::trim)
                            .all(|word| RUST_FENCE.contains(&word)),
                    ),
                };
            } else if fence == Some(true) {
                kept = doc;
            }
        } else {
            fence = None;
        }
        out.push_str(kept);
        out.push('\n');
    }
    out
}

/// The info-string words of a fence rustdoc compiles as Rust; any other word names a language.
const RUST_FENCE: [&str; 11] = [
    "",
    "rust",
    "ignore",
    "should_panic",
    "no_run",
    "compile_fail",
    "edition2015",
    "edition2018",
    "edition2021",
    "edition2024",
    "standalone_crate",
];

/// Every identifier in `code`, with its line and its position.
fn tokens(code: &[char]) -> Vec<(usize, usize, String)> {
    let mut found = Vec::new();
    let (mut line, mut at) = (1, 0);
    while at < code.len() {
        let c = code[at];
        if c == '\n' {
            line += 1;
            at += 1;
        } else if c.is_alphabetic() || c == '_' {
            let start = at;
            while at < code.len() && (code[at].is_alphanumeric() || code[at] == '_') {
                at += 1;
            }
            found.push((line, start, code[start..at].iter().collect()));
        } else {
            at += 1;
        }
    }
    found
}

/// The next character after `at` that is not whitespace, and its position.
fn next_char(code: &[char], mut at: usize) -> Option<(usize, char)> {
    while at < code.len() && code[at].is_whitespace() {
        at += 1;
    }
    code.get(at).map(|&c| (at, c))
}

/// The previous character before `at` that is not whitespace, and its position.
fn previous_char(code: &[char], mut at: usize) -> Option<(usize, char)> {
    while at > 0 {
        at -= 1;
        if !code[at].is_whitespace() {
            return Some((at, code[at]));
        }
    }
    None
}

/// The string literal the source holds at `at` (blanked in `code`), if one starts there and holds
/// no escape: a file name spelled with an escape is refused rather than guessed.
fn literal(text: &[char], at: usize) -> Option<String> {
    if text.get(at) != Some(&'"') {
        return None;
    }
    let len = string(&text[at..]);
    let body: String = text[at + 1..at + len - 1].iter().collect();
    (!body.contains('\\')).then_some(body)
}

/// Whether the token at `at` sits inside an attribute (`#[…]` or `#![…]`), however deep in its
/// parentheses (`#[cfg_attr(test, path = "…")]`).
fn in_attribute(code: &[char], at: usize) -> bool {
    let mut depth = 0_usize;
    let mut index = at;
    while index > 0 {
        index -= 1;
        match code[index] {
            ']' | ')' => depth += 1,
            '(' | '[' if depth > 0 => depth -= 1,
            '[' => {
                return previous_char(code, index).is_some_and(|(before, c)| {
                    c == '#'
                        || (c == '!' && previous_char(code, before).is_some_and(|(_, c)| c == '#'))
                });
            }
            ';' | '{' | '}' => return false,
            _ => {}
        }
    }
    false
}

/// The files a source brings into its crate as code: each `#[path = "…"]` module, each
/// `include!("…")`, and each `#[doc = include_str!("…")]` (whose fenced blocks rustdoc runs), each
/// resolved against the including file's directory. One the census cannot follow is returned as a
/// finding instead.
fn brought_in(path: &Path, text: &[char], code: &[char]) -> (Vec<(PathBuf, bool)>, Vec<usize>) {
    let dir = path.parent().expect("a file's directory");
    let (mut files, mut lost) = (Vec::new(), Vec::new());
    let found = tokens(code);
    for (index, (line, start, token)) in found.iter().enumerate() {
        let end = start + token.len();
        let target = match token.as_str() {
            "path" => match next_char(code, end) {
                Some((equals, '=')) if in_attribute(code, *start) => next_char(text, equals + 1)
                    .and_then(|(at, _)| literal(text, at))
                    .map(|name| (name, false)),
                _ => continue,
            },
            "include" | "include_str" => {
                let Some((bang, '!')) = next_char(code, end) else {
                    continue;
                };
                let Some((open, '(')) = next_char(code, bang + 1) else {
                    continue;
                };
                let doc = token == "include_str"
                    && previous_char(code, *start).is_some_and(|(_, c)| c == '=')
                    && index > 0
                    && found[index - 1].2 == "doc";
                if token == "include_str" && !doc {
                    continue;
                }
                next_char(text, open + 1)
                    .and_then(|(at, _)| literal(text, at).map(|name| (at, name)))
                    .and_then(|(at, name)| {
                        let len = string(&text[at..]);
                        let closes = next_char(code, at + len).is_some_and(|(_, c)| c == ')');
                        closes.then_some((name, doc))
                    })
            }
            _ => continue,
        };
        let resolved = target.and_then(|(name, doc)| {
            let stem = path.file_stem().expect("a file name");
            [dir.join(&name), dir.join(stem).join(&name)]
                .into_iter()
                .find(|candidate| candidate.is_file())
                .map(|found| (found, doc))
        });
        match resolved {
            Some(file) => files.push(file),
            None => lost.push(*line),
        }
    }
    (files, lost)
}

/// A file's census name: its path from the workspace root, or its full path outside it.
fn name_of(root: &Path, path: &Path) -> String {
    let path = path.canonicalize().expect("a readable source");
    path.strip_prefix(root).map_or_else(
        |_| path.display().to_string(),
        |name| name.display().to_string(),
    )
}

/// Every workspace crate that declares itself a proc-macro crate: one can build an install from
/// strings, which no token census reads.
fn proc_macro_crates(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for manifest in fs::read_dir(root.join("crates")).expect("the crates directory") {
        let manifest = manifest
            .expect("a crate directory")
            .path()
            .join("Cargo.toml");
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let line: String = line
                .split('#')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .collect();
            if line == "proc-macro=true" || line == "proc_macro=true" {
                found.push(format!(
                    "{}:{}: a proc-macro crate",
                    name_of(root, &manifest),
                    index + 1
                ));
            }
        }
    }
    found
}

#[test]
fn every_capture_in_the_workspace_goes_through_the_helper() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root");
    let mut files = Vec::new();
    sources(&root.join("crates"), &mut files);
    sources(&root.join("tools"), &mut files);
    let mut queue: Vec<(PathBuf, bool)> =
        examined("Rust source file(s) under crates/ and tools/", files)
            .into_iter()
            .map(|path| (path, false))
            .collect();

    let mut seen = std::collections::BTreeSet::new();
    let mut raw = Vec::new();
    let mut globals = Vec::new();
    let mut routed = 0;
    while let Some((path, doc_only)) = queue.pop() {
        let name = name_of(&root, &path);
        if !seen.insert((name.clone(), doc_only)) {
            continue;
        }
        let text = fs::read_to_string(&path).expect("a readable source");
        let chars: Vec<char> = text.chars().collect();
        let blanked = if doc_only { Vec::new() } else { code(&chars) };
        let doc: Vec<char> = if doc_only {
            let fenced = format!("///{}", text.replace('\n', "\n///"));
            doctests(&fenced).chars().collect()
        } else {
            doctests(&text).chars().collect()
        };
        let (more, lost) = brought_in(&path, &chars, &blanked);
        queue.extend(more);
        for line in lost {
            raw.push(format!(
                "{name}:{line}: an include the census cannot follow"
            ));
        }
        for (line, _, token) in tokens(&blanked).into_iter().chain(tokens(&code(&doc))) {
            let token = token.as_str();
            let allowed = name == HELPER
                || (token == "set_global_default" && name == GLOBAL_DEFAULT)
                || (token == "callsite" && name == KILLER);
            if allowed
                || !(INSTALLS.contains(&token)
                    || REGISTERS.contains(&token)
                    || HIDDEN.contains(&token))
            {
                if token == "set_global_default" && name == GLOBAL_DEFAULT {
                    globals.push(name.clone());
                }
                continue;
            }
            raw.push(format!("{name}:{line}: {token}"));
        }
        if name != KILLER {
            let squashed: String = blanked.iter().filter(|c| !c.is_whitespace()).collect();
            routed += squashed.matches("log_capture::with_capture(").count()
                + squashed.matches("log_capture::hold_capture(").count();
        }
    }
    raw.extend(proc_macro_crates(&root));
    let read = seen.len();
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
