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
//! Two tests carry the class, and the rest pin the helper's refusals, its count and its doc.
//! One runs each scenario in a child process of this binary, whose dispatcher registry is empty
//! at its start, so the loss does not depend on the other tests' timing: another thread reaches
//! the line before the capture, and another thread's
//! first registration of the line is in flight while the capture is made. Each child also checks
//! that the floor was the global default before its capture registered: a capture that registers
//! first leaves a window in which another thread's registration asks no default, answers `never`
//! and stores it after the capture's answer, an order no scenario here needs to reach. The other
//! walks `crates/`, `tools/` and every file they bring in as code, doctests included, and counts
//! every name in its fixed lists, taken from the pinned crates' public items, that installs a
//! subscriber or registers a dispatcher or a callsite.

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

/// The variable that makes this binary install the production global default, then capture.
const ANOTHER_GLOBAL: &str = "DECK_STREAK_LOG_CAPTURE_ANOTHER_GLOBAL";

/// A capture in a binary whose global default is not the floor is refused by the helper. The
/// production global default is the one install the census admits outside the helper, so a test
/// can reach it, and nothing but the helper's refusal keeps a capture from registering where the
/// floor is not the global default, outside the order the class is stated for.
#[test]
fn a_capture_after_the_production_global_default_is_refused() {
    if std::env::var_os(ANOTHER_GLOBAL).is_some() {
        deck_streak_kernel::logging::install(&deck_streak_kernel::Redactor::new())
            .expect("the production global default, first in this child");
        scoped(&Captured::default());
        return;
    }
    let exe = std::env::current_exe().expect("this test binary's path");
    let output = Command::new(&exe)
        .args([
            "--exact",
            "a_capture_after_the_production_global_default_is_refused",
            "--test-threads=1",
        ])
        .env(ANOTHER_GLOBAL, "1")
        .output()
        .expect("the child run");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("installs no other global default"),
        "the helper let a capture register after another global default: {text}"
    );
    assert_eq!(
        output.status.code(),
        Some(101),
        "the refused capture ends the child with a panic"
    );
}

/// What an attempt panicked with, empty when it did not panic.
fn refusal(attempt: impl FnOnce()) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(attempt)) {
        Ok(()) => String::new(),
        Err(payload) => payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default(),
    }
}

/// A capture nested inside a capture on one thread is refused by name, through either entry, for
/// a nested capture would take every line from the outer one. The outer capture keeps its lines,
/// and a capture another thread makes while this one is held is admitted.
#[test]
fn a_capture_nested_inside_a_capture_on_one_thread_is_refused() {
    const NAME: &str = "a capture nested inside another capture on one thread is refused";
    let outer = Captured::default();
    let elsewhere = Captured::default();
    let (nested_scoped, nested_held) = log_capture::with_capture(outer.clone(), || {
        reach();
        let nested_scoped = refusal(|| log_capture::with_capture(Captured::default(), || ()));
        let nested_held = refusal(|| drop(log_capture::hold_capture(Captured::default())));
        let other = elsewhere.clone();
        std::thread::scope(|scope| {
            scope
                .spawn(move || log_capture::with_capture(other, reach))
                .join()
                .expect("the other thread's capture is admitted");
        });
        reach();
        (nested_scoped, nested_held)
    });
    assert_eq!(nested_scoped, NAME, "the scoped entry nested in a capture");
    assert_eq!(nested_held, NAME, "the holding entry nested in a capture");
    assert_eq!(
        outer.lines().len(),
        2,
        "the outer capture keeps both its lines"
    );
    assert_eq!(
        elsewhere.lines().len(),
        1,
        "a capture on another thread is admitted and receives its line"
    );
}

/// A capture made while this thread holds none and its default is not the floor is refused, through
/// either entry, by a message that names the missing floor and not a nested capture (SPEC-024 A19,
/// issue #522). A capture attempted inside a dispatcher's own call reads the default as none while
/// some thread holds a scoped default; while none does, the pinned `tracing-core` answers the global
/// default there, which is the floor. So another thread holds a capture for the attempt's span,
/// whatever the binary's other tests hold.
#[test]
fn a_capture_where_the_floor_is_not_the_default_names_the_missing_floor() {
    const NAME: &str = "a capture is refused: the floor is not this thread's default";
    let (held, on_held) = sync_channel(0);
    let (release, on_release) = sync_channel::<()>(0);
    let elsewhere = std::thread::spawn(move || {
        let _guard = log_capture::hold_capture(Captured::default());
        held.send(())
            .expect("the test hears the other capture is held");
        on_release
            .recv()
            .expect("the test releases the other capture");
    });
    on_held.recv().expect("another thread holds a capture");
    let (scoped, holding) = tracing::dispatcher::get_default(|_| {
        (
            refusal(|| log_capture::with_capture(Captured::default(), || ())),
            refusal(|| drop(log_capture::hold_capture(Captured::default()))),
        )
    });
    release.send(()).expect("the other capture is released");
    elsewhere.join().expect("the other thread's capture ends");
    assert_eq!(
        scoped, NAME,
        "the scoped entry, where the floor is not this thread's default"
    );
    assert_eq!(
        holding, NAME,
        "the holding entry, where the floor is not this thread's default"
    );
}

/// A capture made after a held capture dropped is admitted and receives its line: the count of
/// captures the helper holds on this thread is back at 0 once the held capture's guard drops
/// (SPEC-024 A19).
#[test]
fn a_capture_after_a_held_capture_dropped_is_admitted() {
    let first = Captured::default();
    let then = Captured::default();
    let guard = log_capture::hold_capture(first.clone());
    reach();
    drop(guard);
    let admitted = refusal(|| log_capture::with_capture(then.clone(), reach));
    assert_eq!(
        admitted, "",
        "a capture after a held capture dropped is admitted"
    );
    assert_eq!(
        then.lines(),
        ["log_capture_class"],
        "the admitted capture receives its line"
    );
    assert_eq!(
        first.lines(),
        ["log_capture_class"],
        "the held capture kept its own line"
    );
}

/// The doc of the helper's nesting refusal says where its reading of the default holds: outside a
/// dispatcher's own call, since inside one the default can read as none (SPEC-024 A20, issue #511's
/// wording 3).
#[test]
fn the_nesting_refusal_doc_names_a_dispatchers_own_call() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = fs::read_to_string(root.join(HELPER)).expect("the helper's source");
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.starts_with("fn refuse_nested_capture("))
        .expect("the refusal's definition in the helper");
    let mut doc: Vec<&str> = lines[..at]
        .iter()
        .rev()
        .take_while(|line| line.starts_with("///"))
        .map(|line| line.trim_start_matches('/').trim())
        .collect();
    doc.reverse();
    let doc = examined("doc line(s) of the nesting refusal", doc).join(" ");
    assert!(
        doc.contains("outside a dispatcher's own call"),
        "the nesting refusal's doc does not say where its reading of the default holds: {doc}"
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
/// `fmt::init`), and the future adapters that install one while they are polled
/// (`WithSubscriber::with_subscriber`, `with_current_subscriber` and the `WithDispatch` they
/// return). A path, a `use`, a rename, a method call, a turbofish, a function value and a macro
/// argument all spell one of these as a token, so the census counts tokens rather than one path's
/// text.
const INSTALLS: [&str; 9] = [
    "set_default",
    "with_default",
    "set_global_default",
    "init",
    "try_init",
    "with_subscriber",
    "with_current_subscriber",
    "WithSubscriber",
    "WithDispatch",
];

/// The names that create or register a dispatcher or a callsite, store a callsite's interest, or
/// rebuild the registry's cached answers, outside an install: every public item of the pinned
/// `tracing-core`, `tracing`, `tracing-subscriber` and `tracing-log` whose body reaches
/// `Dispatch::new`, `callsite::register`, `register_dispatch`, `rebuild_interest_cache` or a
/// callsite's `set_interest`, and the macro-support names the `tracing` macros expand to. A
/// `reload` handle's `modify` and `reload` rebuild the cache (`reload.rs`), and so does
/// `LogTracer`. An answer computed before the floor is the global default can be stored after a
/// capture registered, and a rebuild before the floor raises the global level filter from `OFF`
/// with no dispatcher registered, after which a macro registers its callsite and answers `never`.
const REGISTERS: [&str; 17] = [
    "Dispatch",
    "WeakDispatch",
    "callsite",
    "callsite2",
    "DefaultCallsite",
    "MacroCallsite",
    "__macro_support",
    "rebuild_interest_cache",
    "rebuild_interest",
    "set_interest",
    "register_callsite",
    "identify_callsite",
    "Registrar",
    "reload",
    "with_filter_reloading",
    "reload_handle",
    "LogTracer",
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
/// inside a string hides nothing after it; and every doc comment the lexer met, so a comment is a
/// doc comment by the same lexing that blanks it, never by how its line begins.
fn lex(text: &[char]) -> (Vec<char>, Vec<Doc>) {
    let mut code = Vec::with_capacity(text.len());
    let mut docs = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        let len = if rest.starts_with(&['/', '/']) {
            let len = rest.iter().position(|&c| c == '\n').unwrap_or(rest.len());
            let body: String = rest[..len].iter().collect();
            if let Some(doc) = body.strip_prefix("///").filter(|doc| !doc.starts_with('/')) {
                docs.push(Doc::new(at, false, doc));
            } else if let Some(doc) = body.strip_prefix("//!") {
                docs.push(Doc::new(at, true, doc));
            }
            len
        } else if rest.starts_with(&['/', '*']) {
            let len = block_comment(rest);
            let body: String = rest[..len].iter().collect();
            let inner = body.starts_with("/*!");
            let outer = body.starts_with("/**") && !body.starts_with("/***") && body != "/**/";
            if inner || outer {
                let doc = body[3..].strip_suffix("*/").unwrap_or(&body[3..]);
                let lines = doc
                    .split('\n')
                    .map(|line| {
                        let line = line.trim_start();
                        line.strip_prefix('*').unwrap_or(line).to_owned()
                    })
                    .collect();
                docs.push(Doc { at, inner, lines });
            }
            len
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
    (code, docs)
}

/// A file's code with every comment, string literal and character literal blanked.
fn code(text: &[char]) -> Vec<char> {
    lex(text).0
}

/// One piece of an item's documentation: a doc comment or a doc attribute's string, where it
/// starts, whether it documents the item it sits in (`//!`, `/*!`, `#![doc]`), and its lines.
struct Doc {
    at: usize,
    inner: bool,
    lines: Vec<String>,
}

impl Doc {
    fn new(at: usize, inner: bool, line: &str) -> Self {
        Self {
            at,
            inner,
            lines: vec![line.to_owned()],
        }
    }
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

/// The value of a string literal (`"…"` with its escapes decoded, or a raw string's body) that
/// starts at `at`, or `None` when none starts there or an escape does not decode.
fn string_value(text: &[char], at: usize) -> Option<String> {
    if let Some(len) = raw_string(text, at) {
        let quote = text[at..].iter().position(|&c| c == '"')?;
        let hashes = text[at..at + quote].iter().filter(|&&c| c == '#').count();
        let body = text.get(at + quote + 1..(at + len).checked_sub(1 + hashes)?)?;
        return Some(body.iter().collect());
    }
    if text.get(at) != Some(&'"') {
        return None;
    }
    let len = string(&text[at..]);
    let body = &text[at + 1..at + len - 1];
    let mut value = String::new();
    let mut index = 0;
    while index < body.len() {
        let c = body[index];
        index += 1;
        if c != '\\' {
            value.push(c);
            continue;
        }
        let escape = *body.get(index)?;
        index += 1;
        match escape {
            'n' => value.push('\n'),
            't' => value.push('\t'),
            'r' => value.push('\r'),
            '0' => value.push('\0'),
            '\\' | '"' | '\'' => value.push(escape),
            '\n' => {
                while body.get(index).is_some_and(|c| c.is_whitespace()) {
                    index += 1;
                }
            }
            'x' => {
                let hex: String = body.get(index..index + 2)?.iter().collect();
                value.push(char::from(u8::from_str_radix(&hex, 16).ok()?));
                index += 2;
            }
            'u' => {
                let close = body[index..].iter().position(|&c| c == '}')?;
                let hex: String = body[index + 1..index + close]
                    .iter()
                    .filter(|&&c| c != '_')
                    .collect();
                value.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                index += close + 1;
            }
            _ => return None,
        }
    }
    Some(value)
}

/// The span of every attribute (`#[…]`, `#![…]`) in `code`, so a line that holds only attributes
/// and doc comments keeps an item's documentation together, as rustdoc joins it.
fn attribute_spans(code: &[char]) -> Vec<bool> {
    let mut inside = vec![false; code.len()];
    for start in 0..code.len() {
        if code[start] != '#' {
            continue;
        }
        let Some((mut open, mut c)) = next_char(code, start + 1) else {
            continue;
        };
        if c == '!' {
            let Some(next) = next_char(code, open + 1) else {
                continue;
            };
            (open, c) = next;
        }
        if c != '[' {
            continue;
        }
        let mut depth = 0_usize;
        let mut end = open;
        while end < code.len() {
            match code[end] {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            end += 1;
        }
        for flag in &mut inside[start..=end.min(code.len() - 1)] {
            *flag = true;
        }
    }
    inside
}

/// Every doc attribute's string in `code` (`#[doc = "…"]`, `#![doc = r"…"]`, and inside a
/// `cfg_attr`), and the line of each doc attribute whose value the census cannot read. A value
/// that `include_str!`s a file is followed by `brought_in` instead.
fn doc_attributes(text: &[char], code: &[char]) -> (Vec<Doc>, Vec<usize>) {
    let (mut docs, mut unread) = (Vec::new(), Vec::new());
    for (line, start, token) in tokens(code) {
        let Some((_, inner)) = attribute_open(code, start).filter(|_| token == "doc") else {
            continue;
        };
        let Some((equals, '=')) = next_char(code, start + 3) else {
            continue;
        };
        let Some((value, _)) = next_char(text, equals + 1) else {
            unread.push(line);
            continue;
        };
        if let Some(string) = string_value(text, value) {
            docs.push(Doc {
                at: start,
                inner,
                lines: string.split('\n').map(str::to_owned).collect(),
            });
            continue;
        }
        let head: String = code[value..]
            .iter()
            .take_while(|&&c| !matches!(c, '!' | '(' | ']' | ','))
            .filter(|c| !c.is_whitespace())
            .collect();
        let includes = head == "include_str" || head.ends_with("::include_str");
        if !includes {
            unread.push(line);
        }
    }
    (docs, unread)
}

/// One doctest: its lines, each with the file line it came from.
type Unit = Vec<(usize, String)>;

/// The code rustdoc runs as doctests, one unit per doctest: every Rust fenced block and every
/// indented block of every item's documentation. An item's documentation is every doc comment and
/// doc attribute string with nothing but attributes, comments and blank lines between them, the
/// module's own (`//!`, `#![doc]`) apart from its items', as rustdoc joins them.
fn doctests(text: &[char], code: &[char], mut docs: Vec<Doc>) -> Vec<Unit> {
    let line_of = |at: usize| 1 + text[..at].iter().filter(|&&c| c == '\n').count();
    let attributes = attribute_spans(code);
    docs.sort_by_key(|doc| doc.at);
    let mut units = Vec::new();
    let mut block: Unit = Vec::new();
    let mut previous: Option<(usize, bool)> = None;
    for doc in docs {
        let joined = previous.is_some_and(|(end, inner)| {
            inner == doc.inner && (end..doc.at).all(|at| code[at].is_whitespace() || attributes[at])
        });
        if !joined {
            units.extend(markdown(&block));
            block.clear();
        }
        let line = line_of(doc.at);
        block.extend(doc.lines.iter().map(|content| (line, content.clone())));
        previous = Some((doc.at, doc.inner));
    }
    units.extend(markdown(&block));
    units
}

/// The Rust code one item's documentation holds, one unit per doctest, by `CommonMark`'s blocks as
/// rustdoc reads them: a fence of three or more backticks or tildes, at any container depth (a
/// blockquote, a list item), is Rust unless its info string names another language by rustdoc's
/// rule, and a line indented four or more columns outside a fence is an indented code block, which
/// rustdoc runs as Rust.
fn markdown(block: &[(usize, String)]) -> Vec<Unit> {
    let mut units = Vec::new();
    let mut fence: Option<(char, usize, bool)> = None;
    let (mut fenced, mut indented): (Unit, Unit) = (Vec::new(), Vec::new());
    for (line, content) in block {
        let (indents, rest) = container(content);
        if let Some((mark, len, rust)) = fence {
            let run = rest.chars().take_while(|&c| c == mark).count();
            if run >= len && rest[run..].trim().is_empty() {
                fence = None;
                units.push(std::mem::take(&mut fenced));
            } else if rust {
                fenced.push((*line, content.clone()));
            }
            continue;
        }
        let mark = rest.chars().next().filter(|&c| c == '`' || c == '~');
        if let Some(mark) = mark {
            let run = rest.chars().take_while(|&c| c == mark).count();
            let info = &rest[run..];
            if run >= 3 && !(mark == '`' && info.contains('`')) {
                fence = Some((mark, run, rust_fence(info)));
                units.push(std::mem::take(&mut indented));
                continue;
            }
        }
        if rest.trim().is_empty() {
            if !indented.is_empty() {
                indented.push((*line, String::new()));
            }
        } else if indents.iter().any(|&columns| columns >= 4) {
            indented.push((*line, content.clone()));
        } else {
            units.push(std::mem::take(&mut indented));
        }
    }
    units.push(fenced);
    units.push(indented);
    units.retain(|unit| !unit.is_empty());
    units
}

/// A doc line's container prefixes stripped (blockquote markers and list-item markers), with the
/// width of the whitespace before each, tabs as four columns.
fn container(line: &str) -> (Vec<usize>, &str) {
    let width = |text: &str| {
        text.chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum::<usize>()
    };
    let mut indents = vec![width(line)];
    let mut rest = line.trim_start();
    loop {
        let marker = if rest.starts_with('>') {
            1
        } else if rest.starts_with(['-', '*', '+']) {
            usize::from(rest[1..].starts_with([' ', '\t']))
        } else {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            let closes = rest[digits..].starts_with(['.', ')']);
            let spaced = rest[digits..]
                .get(1..)
                .is_some_and(|after| after.starts_with([' ', '\t']));
            if (1..=9).contains(&digits) && closes && spaced {
                digits + 1
            } else {
                0
            }
        };
        if marker == 0 {
            break;
        }
        indents.push(width(&rest[marker..]));
        rest = rest[marker..].trim_start();
    }
    (indents, rest)
}

/// Whether rustdoc compiles a fence with this info string as Rust, read as a superset of rustdoc's
/// `LangString` rule: the words split on commas, spaces and tabs; a `{…}` or `key=value` word is an
/// attribute and names no language; the block is Rust when no other word is present, or when any
/// word is one rustdoc reads as Rust's.
fn rust_fence(info: &str) -> bool {
    let words: Vec<&str> = info
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|word| !word.is_empty() && !word.contains(['{', '}', '=']))
        .collect();
    let rust = |word: &&str| {
        RUST_FENCE.contains(word)
            || word.starts_with("ignore")
            || word.starts_with("edition")
            || (word.len() == 5
                && word.starts_with('E')
                && word[1..].bytes().all(|b| b.is_ascii_digit()))
    };
    words.is_empty() || words.iter().any(rust)
}

/// The info-string words rustdoc reads as Rust's; `ignore-…` and `edition…` words are Rust's too.
const RUST_FENCE: [&str; 7] = [
    "rust",
    "ignore",
    "should_panic",
    "no_run",
    "compile_fail",
    "test_harness",
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

/// The `[` that opens the attribute (`#[…]` or `#![…]`) the token at `at` sits inside, however
/// deep in its parentheses (`#[cfg_attr(test, path = "…")]`), and whether it is an inner one.
fn attribute_open(code: &[char], at: usize) -> Option<(usize, bool)> {
    let mut depth = 0_usize;
    let mut index = at;
    while index > 0 {
        index -= 1;
        match code[index] {
            ']' | ')' => depth += 1,
            '(' | '[' if depth > 0 => depth -= 1,
            '[' => {
                let (before, c) = previous_char(code, index)?;
                if c == '#' {
                    return Some((index, false));
                }
                let inner = c == '!' && previous_char(code, before).is_some_and(|(_, c)| c == '#');
                return inner.then_some((index, true));
            }
            ';' | '{' | '}' => return None,
            _ => {}
        }
    }
    None
}

/// Whether the token at `at` sits inside an attribute.
fn in_attribute(code: &[char], at: usize) -> bool {
    attribute_open(code, at).is_some()
}

/// A file brought in as code (`false`) or as documentation (`true`), and the line that brings it.
type BroughtIn = (PathBuf, bool, usize);

/// The files a source brings into its crate: each `#[path = "…"]` module, each `include!("…")`,
/// and each `include_str!("…")` inside an attribute (a doc attribute's file, whose fenced blocks
/// rustdoc runs), each resolved against the including file's directory. One the census cannot
/// follow is returned as a finding instead, and so is a `#[path]` inside a block (an inline module
/// or a function body), which rustc resolves under the module's own directories.
fn brought_in(
    path: &Path,
    text: &[char],
    code: &[char],
) -> (Vec<BroughtIn>, Vec<(usize, &'static str)>) {
    let dir = path.parent().expect("a file's directory");
    let (mut files, mut lost) = (Vec::new(), Vec::new());
    for (line, start, token) in tokens(code) {
        let end = start + token.len();
        let target = match token.as_str() {
            "path" => match next_char(code, end) {
                Some((equals, '=')) if in_attribute(code, start) => {
                    let depth = code[..start].iter().fold(0_i64, |depth, &c| match c {
                        '{' => depth + 1,
                        '}' => depth - 1,
                        _ => depth,
                    });
                    if depth != 0 {
                        lost.push((line, "a #[path] inside a block"));
                        continue;
                    }
                    next_char(text, equals + 1)
                        .and_then(|(at, _)| literal(text, at))
                        .map(|name| (name, false))
                }
                _ => continue,
            },
            "include" | "include_str" => {
                let Some((bang, '!')) = next_char(code, end) else {
                    continue;
                };
                let Some((open, '(')) = next_char(code, bang + 1) else {
                    continue;
                };
                let doc = token == "include_str";
                if doc && !in_attribute(code, start) {
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
                .map(|found| (found, doc, line))
        });
        match resolved {
            Some(file) => files.push(file),
            None => lost.push((line, "an include the census cannot follow")),
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

/// Whether `path` lies under `crates/` or `tools/`, the two trees the census walks.
fn walked(root: &Path, path: &Path) -> bool {
    path.starts_with(root.join("crates")) || path.starts_with(root.join("tools"))
}

/// `path` with every `.` and `..` resolved by its text, and cut before its first glob.
fn normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => {
                let text = other.as_os_str().to_string_lossy();
                if text.contains(['*', '?', '[']) {
                    break;
                }
                out.push(other);
            }
        }
    }
    out
}

/// Every Cargo manifest and Cargo configuration file that decides what the workspace compiles.
fn manifests(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        let config = path
            .parent()
            .is_some_and(|parent| parent.ends_with(".cargo"));
        if path.is_dir() {
            manifests(&path, into);
        } else if name.as_deref() == Some("Cargo.toml")
            || (config && matches!(name.as_deref(), Some("config" | "config.toml")))
        {
            into.push(path);
        }
    }
}

/// A TOML token: a bare word, a string (`None` when it is multi-line or holds an escape), or a
/// punctuation character; each with its line.
enum Toml {
    Word(String),
    Text(Option<String>),
    Mark(char),
}

/// A manifest's tokens, comments dropped.
fn toml(text: &str) -> Vec<(usize, Toml)> {
    let chars: Vec<char> = text.chars().collect();
    let (mut found, mut line, mut at) = (Vec::new(), 1, 0);
    while at < chars.len() {
        let c = chars[at];
        if c == '\n' {
            line += 1;
            at += 1;
        } else if c == '#' {
            while at < chars.len() && chars[at] != '\n' {
                at += 1;
            }
        } else if c == '"' || c == '\'' {
            let triple = chars[at..].iter().take(3).all(|&q| q == c);
            let open = if triple { 3 } else { 1 };
            let mut end = at + open;
            let mut escaped = false;
            while end < chars.len() {
                if c == '"' && chars[end] == '\\' {
                    escaped = true;
                    end += 2;
                    continue;
                }
                if chars[end..].iter().take(open).filter(|&&q| q == c).count() == open {
                    break;
                }
                end += 1;
            }
            let end = end.min(chars.len());
            let body: String = chars[(at + open).min(end)..end].iter().collect();
            line += body.matches('\n').count();
            found.push((line, Toml::Text((!triple && !escaped).then_some(body))));
            at = end + open;
        } else if c.is_alphanumeric() || c == '_' || c == '-' {
            let start = at;
            while at < chars.len()
                && (chars[at].is_alphanumeric() || chars[at] == '_' || chars[at] == '-')
            {
                at += 1;
            }
            found.push((line, Toml::Word(chars[start..at].iter().collect())));
        } else {
            if !c.is_whitespace() {
                found.push((line, Toml::Mark(c)));
            }
            at += 1;
        }
    }
    found
}

/// The keys whose values name a file or a directory Cargo compiles from.
const PATH_KEYS: [&str; 6] = [
    "path",
    "build",
    "workspace",
    "members",
    "default-members",
    "paths",
];

/// Every path a manifest or a Cargo configuration names under a `PATH_KEYS` key (bare, quoted or
/// dotted): the files among them, to be read as code, and a finding for each one that lies outside
/// `crates/` and `tools/` or that the census cannot read.
fn manifest_paths(root: &Path, manifest: &Path) -> (Vec<BroughtIn>, Vec<String>) {
    let text = fs::read_to_string(manifest).expect("a readable manifest");
    let name = name_of(root, manifest);
    let base = manifest.parent().expect("a manifest's directory");
    let base = if base.ends_with(".cargo") {
        base.parent().expect("the directory holding .cargo")
    } else {
        base
    };
    let found = toml(&text);
    let (mut files, mut findings) = (Vec::new(), Vec::new());
    for (index, (line, token)) in found.iter().enumerate() {
        let key = match token {
            Toml::Word(word) | Toml::Text(Some(word)) => Some(word.as_str()),
            _ => None,
        };
        let Some(key) = key.filter(|key| PATH_KEYS.contains(key)) else {
            continue;
        };
        if !matches!(found.get(index + 1), Some((_, Toml::Mark('=')))) {
            continue;
        }
        let mut values = Vec::new();
        match found.get(index + 2) {
            Some((_, Toml::Text(value))) => values.push(value.clone()),
            Some((_, Toml::Mark('['))) => {
                for (_, item) in &found[index + 3..] {
                    match item {
                        Toml::Mark(']') => break,
                        Toml::Text(value) => values.push(value.clone()),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        for value in values {
            let Some(value) = value else {
                findings.push(format!("{name}:{line}: a {key} the census cannot read"));
                continue;
            };
            let resolved = normal(&base.join(&value));
            if !walked(root, &resolved) {
                findings.push(format!(
                    "{name}:{line}: {key} = {value:?} lies outside crates/ and tools/"
                ));
            } else if resolved.is_file() {
                files.push((resolved, false, *line));
            }
        }
    }
    (files, findings)
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

/// Every file the manifests name as a target, to be read as code; each manifest path outside
/// `crates/` and `tools/`, or unreadable, is a finding in `raw`.
fn manifest_targets(root: &Path, raw: &mut Vec<String>) -> Vec<(PathBuf, bool)> {
    let mut found = vec![root.join("Cargo.toml")];
    manifests(&root.join("crates"), &mut found);
    manifests(&root.join("tools"), &mut found);
    for config in ["config", "config.toml"] {
        let path = root.join(".cargo").join(config);
        if path.is_file() {
            found.push(path);
        }
    }
    let mut targets = Vec::new();
    for manifest in examined("Cargo manifest(s) and configuration(s)", found) {
        let (files, findings) = manifest_paths(root, &manifest);
        targets.extend(files.into_iter().map(|(path, doc, _)| (path, doc)));
        raw.extend(findings);
    }
    targets
}

/// What the census found so far: the files still to read, the raw captures, the production global
/// defaults, the routed captures and the doctests read.
#[derive(Default)]
struct Census {
    queue: Vec<(PathBuf, bool)>,
    raw: Vec<String>,
    globals: Vec<String>,
    routed: usize,
    doctest_units: usize,
}

/// Reads one file, as code or as documentation: its tokens and its doctests' tokens, the files it
/// brings in, and the captures it routes through the helper.
fn read_file(root: &Path, census: &mut Census, path: &Path, name: &str, doc_only: bool) {
    let Census {
        queue,
        raw,
        globals,
        routed,
        doctest_units,
    } = census;
    let text = fs::read_to_string(path).expect("a readable source");
    let chars: Vec<char> = text.chars().collect();
    let (blanked, units) = if doc_only {
        let blank: Vec<char> = chars
            .iter()
            .map(|&c| if c == '\n' { '\n' } else { ' ' })
            .collect();
        let block: Unit = text
            .split('\n')
            .enumerate()
            .map(|(index, line)| (index + 1, line.to_owned()))
            .collect();
        (blank, markdown(&block))
    } else {
        let (blanked, mut docs) = lex(&chars);
        let (attributes, unread) = doc_attributes(&chars, &blanked);
        for line in unread {
            raw.push(format!(
                "{name}:{line}: a doc attribute the census cannot read"
            ));
        }
        docs.extend(attributes);
        let units = doctests(&chars, &blanked, docs);
        (blanked, units)
    };
    *doctest_units += units.len();
    let mut found = tokens(&blanked);
    let mut sources = vec![(chars.clone(), blanked.clone(), Vec::new())];
    for unit in units {
        let text: Vec<char> = unit
            .iter()
            .map(|(_, line)| line.as_str())
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .collect();
        let unit_code = code(&text);
        let lines: Vec<usize> = unit.iter().map(|(line, _)| *line).collect();
        found.extend(
            tokens(&unit_code)
                .into_iter()
                .map(|(line, at, token)| (lines[line - 1], at, token)),
        );
        sources.push((text, unit_code, lines));
    }
    for (source, source_code, lines) in &sources {
        let (more, lost) = brought_in(path, source, source_code);
        let line_in_file = |line: usize| lines.get(line - 1).copied().unwrap_or(line);
        for (file, doc, line) in more {
            let inside = file.canonicalize().is_ok_and(|file| walked(root, &file));
            if inside {
                queue.push((file, doc));
            } else {
                raw.push(format!(
                    "{name}:{}: brings in {} from outside crates/ and tools/",
                    line_in_file(line),
                    file.display()
                ));
            }
        }
        for (line, why) in lost {
            raw.push(format!("{name}:{}: {why}", line_in_file(line)));
        }
    }
    for (line, _, token) in found {
        let token = token.as_str();
        let allowed = name == HELPER
            || (token == "set_global_default" && name == GLOBAL_DEFAULT)
            || (matches!(token, "callsite" | "set_interest") && name == KILLER);
        if allowed
            || !(INSTALLS.contains(&token) || REGISTERS.contains(&token) || HIDDEN.contains(&token))
        {
            if token == "set_global_default" && name == GLOBAL_DEFAULT {
                globals.push(name.to_owned());
            }
            continue;
        }
        raw.push(format!("{name}:{line}: {token}"));
    }
    if name != KILLER {
        let squashed: String = blanked.iter().filter(|c| !c.is_whitespace()).collect();
        *routed += squashed.matches("log_capture::with_capture(").count()
            + squashed.matches("log_capture::hold_capture(").count();
    }
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
    let mut census = Census {
        queue: examined("Rust source file(s) under crates/ and tools/", files)
            .into_iter()
            .map(|path| (path, false))
            .collect(),
        ..Census::default()
    };
    let targets = manifest_targets(&root, &mut census.raw);
    census.queue.extend(targets);

    let mut seen = std::collections::BTreeSet::new();
    while let Some((path, doc_only)) = census.queue.pop() {
        let name = name_of(&root, &path);
        if seen.insert((name.clone(), doc_only)) {
            read_file(&root, &mut census, &path, &name, doc_only);
        }
    }
    let Census {
        mut raw,
        globals,
        routed,
        doctest_units,
        ..
    } = census;
    raw.extend(proc_macro_crates(&root));
    eprintln!("examined {doctest_units} doctest(s)");
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
    assert!(
        raw.is_empty(),
        "{} capture(s) bypass the helper: {raw:#?}\n{report}",
        raw.len()
    );
    assert_eq!(
        globals,
        [GLOBAL_DEFAULT],
        "the one production global default is the only one: {report}"
    );
    assert_eq!(routed, 25, "{report}");
}

/// A capture whose body panics lowers the helper's count as it unwinds, so a later capture on the
/// thread is admitted: `with_capture`'s drop guard, not a line after the body, restores the count.
#[test]
fn a_capture_after_a_panicking_body_is_admitted() {
    let first = Captured::default();
    let then = Captured::default();
    let body = refusal(|| {
        log_capture::with_capture::<_, ()>(first.clone(), || {
            reach();
            panic!("the capture's body panics");
        });
    });
    assert_eq!(
        body, "the capture's body panics",
        "the body's own panic unwound the capture"
    );
    let admitted = refusal(|| log_capture::with_capture(then.clone(), reach));
    assert_eq!(admitted, "", "a capture after a panicking body is admitted");
    assert_eq!(
        then.lines(),
        ["log_capture_class"],
        "the admitted capture receives its line"
    );
}
