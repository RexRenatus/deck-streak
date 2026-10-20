//! The scheduling port's censuses (SPEC-494 A5 and A6): the replay's due days come only through
//! the port, no new file asks the engine for its queue, and no clock or timezone read sits inside
//! the port.
//!
//! Each census reads every `.rs` file under the production sources of the engine core, the two
//! client adapters, ingest and the FSRS-7 crate, ignores a line's text from its first `//`, prints
//! how many files it examined and refuses zero, and is run once over a planted tree whose every
//! planted bypass it must refuse by name.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The production source trees the censuses read, relative to the workspace root.
const TREES: [&str; 5] = [
    "crates/engine-core/src",
    "crates/ffi/src",
    "crates/web-engine/src",
    "crates/ingest/src",
    "crates/fsrs7/src",
];

/// The FSRS-7 crate's sources, where a model call belongs.
const FSRS7_SRC: &str = "crates/fsrs7/src/";

/// The file that holds the port's FSRS-7 module and the replay, its producer.
const PORT_FILE: &str = "crates/engine-core/src/replay.rs";

/// The port's own module.
const SCHEDULER_FILE: &str = "crates/engine-core/src/scheduler.rs";

/// The line that opens the port's span in [`PORT_FILE`].
const PORT_OPEN: &str = "mod fsrs7_port {";

/// The line that closes it: the first line after [`PORT_OPEN`] that is exactly this.
const PORT_CLOSE: &str = "}";

/// The model's calls: the interval and the retrievability functions, the batch and single
/// replays, the stock projection and the crate's replay.
const MODEL_CALLS: [&str; 6] = [
    "interval_at_retrievability",
    "memory_state_batch",
    "current_retrievability",
    "next_states(",
    "stock::project",
    "scheduler::replay",
];

/// The call the producer's schedule comes through.
const PORT_ROUTE: &str = ".interval_days(";

/// The engine's queue call, in its Rust and its service spelling.
const QUEUE_CALLS: [&str; 2] = ["get_queued_cards", "GetQueuedCards"];

/// The five files that ask the engine for its queue at the cut.
const QUEUE_FILES: [&str; 5] = [
    "crates/engine-core/src/table.rs",
    "crates/ffi/src/allow_list.rs",
    "crates/ingest/src/engine.rs",
    "crates/web-engine/src/study.rs",
    "crates/web-engine/src/wasm.rs",
];

/// A clock or timezone read: the system clock, a monotonic instant, the epoch, the page's clock,
/// a date library, the engine's timing of today, a UTC offset and a rollover hour.
const CLOCK_READS: [&str; 10] = [
    "SystemTime",
    "Instant",
    "UNIX_EPOCH",
    "Date::now",
    "js_sys::Date",
    "chrono",
    "timing_today",
    "SchedTimingToday",
    "utc_offset",
    "rollover",
];

/// Every `.rs` file under the census trees of `root`, as `/`-separated paths relative to `root`,
/// sorted.
fn sources(root: &Path) -> Vec<String> {
    let mut pending: Vec<PathBuf> = TREES
        .iter()
        .map(|tree| root.join(tree))
        .filter(|tree| tree.is_dir())
        .collect();
    let mut found = Vec::new();
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("a source directory reads") {
            let path = entry.expect("a source entry reads").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let relative = path
                    .strip_prefix(root)
                    .expect("a walked file sits under its root");
                found.push(
                    relative
                        .components()
                        .map(|part| part.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
            }
        }
    }
    found.sort();
    found
}

/// A line's code: its text before its first `//`.
fn code(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}

/// What the route census refuses in the `walked` files of `root`, each by name, and how many lines
/// of the replay's producer it read.
fn route_refusals(root: &Path, walked: &[String]) -> (Vec<String>, usize) {
    let mut refusals = Vec::new();
    let mut producer_lines = 0;
    let mut producer_routes = 0;
    let mut asking = BTreeSet::new();
    for file in walked {
        let file = file.as_str();
        let text = fs::read_to_string(root.join(file)).expect("a walked source reads");
        let mut inside = false;
        for (index, line) in text.lines().enumerate() {
            if file == PORT_FILE && line == PORT_OPEN {
                inside = true;
            } else if inside && line == PORT_CLOSE {
                inside = false;
            }
            let code = code(line);
            for name in MODEL_CALLS.iter().filter(|name| code.contains(*name)) {
                let allowed = file.starts_with(FSRS7_SRC) || (file == PORT_FILE && inside);
                if !allowed {
                    refusals.push(format!(
                        "{file}:{} names {name} outside the FSRS-7 crate and the port",
                        index + 1
                    ));
                }
            }
            if file == PORT_FILE && !inside {
                producer_lines += 1;
                if code.contains(PORT_ROUTE) {
                    producer_routes += 1;
                }
            }
            if QUEUE_CALLS.iter().any(|name| code.contains(name)) {
                asking.insert(file.to_owned());
            }
        }
    }
    if walked.iter().any(|file| file == PORT_FILE) && producer_routes == 0 {
        refusals.push(format!("{PORT_FILE}: the replay names no {PORT_ROUTE}"));
    }
    let cut: BTreeSet<String> = QUEUE_FILES.iter().map(|file| (*file).to_owned()).collect();
    for file in asking.difference(&cut) {
        refusals.push(format!(
            "{file} asks the engine for its queue outside the cut's five files"
        ));
    }
    for file in cut.difference(&asking) {
        refusals.push(format!(
            "{file}, one of the cut's five files, no longer asks the engine for its queue"
        ));
    }
    (refusals, producer_lines)
}

/// What the purity census refuses in the `walked` files of `root`, each by name, and how many
/// lines of the port it judged.
fn purity_refusals(root: &Path, walked: &[String]) -> (Vec<String>, usize) {
    let mut refusals = Vec::new();
    let mut judged_lines = 0;
    for file in walked {
        let file = file.as_str();
        let text = fs::read_to_string(root.join(file)).expect("a walked source reads");
        let mut in_port = false;
        for (index, line) in text.lines().enumerate() {
            if file == PORT_FILE && line == PORT_OPEN {
                in_port = true;
            } else if in_port && line == PORT_CLOSE {
                in_port = false;
            }
            let judged = file == SCHEDULER_FILE || (file == PORT_FILE && in_port);
            if !judged {
                continue;
            }
            judged_lines += 1;
            let code = code(line);
            for name in CLOCK_READS.iter().filter(|name| code.contains(*name)) {
                refusals.push(format!("{file}:{} reads {name} inside the port", index + 1));
            }
        }
    }
    (refusals, judged_lines)
}

/// Writes `files` (path, text) under a fresh scratch directory and returns it.
fn plant(test: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = support::scratch("engine-core-scheduler-census", test);
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a planted file has a directory"))
            .expect("a planted directory");
        fs::write(path, text).expect("a planted file writes");
    }
    root
}

#[test]
fn the_due_days_come_only_through_the_port_and_no_new_file_asks_for_the_engine_queue() {
    let workspace = support::workspace();
    let walked = support::examined("production source file(s)", sources(&workspace));
    let (refusals, producer_lines) = route_refusals(&workspace, &walked);
    assert_eq!(
        refusals,
        Vec::<String>::new(),
        "the route census over the tree"
    );
    support::examined("line(s) of the replay's producer", vec![(); producer_lines]);

    let planted = plant(
        "the_due_days_come_only_through_the_port",
        &[
            (
                PORT_FILE,
                "use crate::scheduler as port;\n\
                 mod fsrs7_port {\n    \
                     fn whole(&self) -> f32 { stock::project(&self.model, state, target).interval }\n\
                 }\n\
                 fn replay() {\n    \
                     let rounded = fsrs7.interval_days(&state, history.cid, target);\n    \
                     let direct = stock::project(&model, state, retention).interval.round();\n\
                 }\n",
            ),
            (
                "crates/ffi/src/engine.rs",
                "let states = model.memory_state_batch(items, starting);\n\
                 // next_states( in a comment is no call\n",
            ),
            (
                "crates/fsrs7/src/stock.rs",
                "let at = model.interval_at_retrievability(state, 0.9);\n",
            ),
            (
                "crates/engine-core/src/table.rs",
                "(13, 3, \"GetQueuedCards\"),\n",
            ),
            (
                "crates/ffi/src/allow_list.rs",
                "(13, 3, \"GetQueuedCards\"),\n",
            ),
            (
                "crates/ingest/src/engine.rs",
                "col.get_queued_cards(1000, false)\n",
            ),
            ("crates/web-engine/src/study.rs", "\"GetQueuedCards\"\n"),
            (
                "crates/web-engine/src/wasm.rs",
                "run(GET_QUEUED_CARDS) // GetQueuedCards\nGetQueuedCards\n",
            ),
            (
                "crates/web-engine/src/sixth.rs",
                "let queue = col.get_queued_cards(1, false);\n",
            ),
            (
                "crates/ingest/src/notes.rs",
                "// get_queued_cards named in a comment\n",
            ),
        ],
    );
    let planted_walk = support::examined("planted source file(s)", sources(&planted));
    assert_eq!(
        route_refusals(&planted, &planted_walk).0,
        vec![
            format!("{PORT_FILE}:7 names stock::project outside the FSRS-7 crate and the port"),
            "crates/ffi/src/engine.rs:1 names memory_state_batch outside the FSRS-7 crate and the \
             port"
                .to_owned(),
            "crates/web-engine/src/sixth.rs asks the engine for its queue outside the cut's five \
             files"
                .to_owned(),
        ],
        "a producer that rounds the projection itself, a model call in an adapter and a sixth \
         file asking for the engine's queue are each refused by name"
    );
}

#[test]
fn no_clock_or_timezone_read_sits_inside_the_port() {
    let workspace = support::workspace();
    let walked = support::examined("production source file(s)", sources(&workspace));
    let (refusals, judged_lines) = purity_refusals(&workspace, &walked);
    assert_eq!(
        refusals,
        Vec::<String>::new(),
        "the purity census over the tree"
    );
    support::examined("line(s) of the port judged", vec![(); judged_lines]);

    let planted = plant(
        "no_clock_or_timezone_read_sits_inside_the_port",
        &[
            (
                PORT_FILE,
                "mod fsrs7_port {\n    \
                     fn fold(&self) { let _planted = std::time::SystemTime::now(); }\n\
                 }\n\
                 fn replay() {\n    \
                     let _planted = std::time::SystemTime::now();\n\
                 }\n",
            ),
            (
                SCHEDULER_FILE,
                "pub fn day(at: std::time::Instant) -> i32 { 0 }\n",
            ),
        ],
    );
    let planted_walk = support::examined("planted source file(s)", sources(&planted));
    assert_eq!(
        purity_refusals(&planted, &planted_walk).0,
        vec![
            format!("{PORT_FILE}:2 reads SystemTime inside the port"),
            format!("{SCHEDULER_FILE}:1 reads Instant inside the port"),
        ],
        "a planted port that reads the clock and a planted Instant in the port's module are \
         refused by name, and the same read in the producer is not"
    );
}
