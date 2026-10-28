//! The kernel re-exports the study day crate's own types at every path it exported them from, and
//! `SystemClock` stays in the kernel's clock while the instant leaves it (SPEC-497 A2 and A6;
//! ADR-515 D2 and D3).

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined counts on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::any::TypeId;
use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Clock, SystemClock};

/// The kernel's clock, by its path from the root.
const CLOCK: &str = "crates/kernel/src/clock.rs";

/// The lines the kernel's clock keeps: the trait, the system's clock and its one read of the
/// system time, and the manual clock (SPEC-020 R6).
const KEPT: [&str; 4] = [
    "pub trait Clock: Send + Sync {",
    "pub struct SystemClock;",
    "UtcMillis::from_system_time(SystemTime::now())",
    "pub struct ManualClock {",
];

/// The line by which the kernel's clock takes the instant from the crate.
const TAKEN: &str = "pub use deck_streak_study_day::UtcMillis;";

/// A kernel path to a moved item, the id of the type it names, and the id of the crate's own type.
type Path3 = (&'static str, TypeId, TypeId);

/// Every kernel path to a moved type, each beside the crate's own type of that name.
fn kernel_paths() -> Vec<Path3> {
    use deck_streak_kernel as kernel;
    use deck_streak_study_day as moved;
    vec![
        (
            "deck_streak_kernel::UtcMillis",
            TypeId::of::<kernel::UtcMillis>(),
            TypeId::of::<moved::UtcMillis>(),
        ),
        (
            "deck_streak_kernel::clock::UtcMillis",
            TypeId::of::<kernel::clock::UtcMillis>(),
            TypeId::of::<moved::UtcMillis>(),
        ),
        (
            "deck_streak_kernel::IsoDateError",
            TypeId::of::<kernel::IsoDateError>(),
            TypeId::of::<moved::IsoDateError>(),
        ),
        (
            "deck_streak_kernel::error::IsoDateError",
            TypeId::of::<kernel::error::IsoDateError>(),
            TypeId::of::<moved::IsoDateError>(),
        ),
        (
            "deck_streak_kernel::Hour",
            TypeId::of::<kernel::Hour>(),
            TypeId::of::<moved::Hour>(),
        ),
        (
            "deck_streak_kernel::study_day::Hour",
            TypeId::of::<kernel::study_day::Hour>(),
            TypeId::of::<moved::Hour>(),
        ),
        (
            "deck_streak_kernel::UtcOffset",
            TypeId::of::<kernel::UtcOffset>(),
            TypeId::of::<moved::UtcOffset>(),
        ),
        (
            "deck_streak_kernel::study_day::UtcOffset",
            TypeId::of::<kernel::study_day::UtcOffset>(),
            TypeId::of::<moved::UtcOffset>(),
        ),
        (
            "deck_streak_kernel::StudyDayRule",
            TypeId::of::<kernel::StudyDayRule>(),
            TypeId::of::<moved::StudyDayRule>(),
        ),
        (
            "deck_streak_kernel::study_day::StudyDayRule",
            TypeId::of::<kernel::study_day::StudyDayRule>(),
            TypeId::of::<moved::StudyDayRule>(),
        ),
        (
            "deck_streak_kernel::StudyDay",
            TypeId::of::<kernel::StudyDay>(),
            TypeId::of::<moved::StudyDay>(),
        ),
        (
            "deck_streak_kernel::study_day::StudyDay",
            TypeId::of::<kernel::study_day::StudyDay>(),
            TypeId::of::<moved::StudyDay>(),
        ),
    ]
}

/// The paths that name a type of their own rather than the crate's.
fn own_types(paths: &[Path3]) -> Vec<&'static str> {
    paths
        .iter()
        .filter(|(_, named, moved)| named != moved)
        .map(|(path, _, _)| *path)
        .collect()
}

/// The names on the code part of `line`, before any `//`, with lifetimes (`'static`) left out.
fn names(line: &str) -> Vec<&str> {
    let code = line.find("//").map_or(line, |at| &line[..at]);
    let mut found = Vec::new();
    let mut start = None;
    for (index, ch) in code
        .char_indices()
        .chain(std::iter::once((code.len(), ' ')))
    {
        let part = ch.is_ascii_alphanumeric() || ch == '_';
        match (start, part) {
            (None, true) => start = Some(index),
            (Some(from), false) => {
                if !code[..from].ends_with('\'') {
                    found.push(&code[from..index]);
                }
                start = None;
            }
            _ => {}
        }
    }
    found
}

/// What the kernel's clock and the crate's sources get wrong about the clock's split, sorted:
/// a kept line the clock lost, an instant the clock still defines or does not take from the crate,
/// and a source of the crate that names the system's clock or reads the system time.
fn clock_findings(clock: &str, crate_sources: &[(String, String)]) -> Vec<String> {
    let mut findings = Vec::new();
    for kept in KEPT {
        if !clock.lines().any(|line| line.trim() == kept) {
            findings.push(format!("{CLOCK} holds no `{kept}`"));
        }
    }
    if !clock.lines().any(|line| line.trim() == TAKEN) {
        findings.push(format!("{CLOCK} does not take UtcMillis from the crate"));
    }
    if clock.lines().any(|line| {
        names(line)
            .windows(2)
            .any(|pair| pair == ["struct", "UtcMillis"])
    }) {
        findings.push(format!("{CLOCK} still defines UtcMillis"));
    }
    for (path, text) in crate_sources {
        for (number, line) in text.lines().enumerate() {
            let held = names(line);
            if held.contains(&"SystemClock") {
                findings.push(format!("{path}:{}: names SystemClock", number + 1));
            }
            if held.windows(2).any(|pair| pair == ["SystemTime", "now"]) {
                findings.push(format!("{path}:{}: reads the system time", number + 1));
            }
        }
    }
    findings.sort();
    findings
}

/// The workspace's root, from this crate's manifest directory.
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.rs` file at or under `relative`, sorted, as its path from the root and its text.
fn rust_files(relative: &str) -> Vec<(String, String)> {
    let mut entries: Vec<_> = fs::read_dir(workspace().join(relative))
        .unwrap_or_else(|error| panic!("{relative} reads: {error}"))
        .map(|entry| entry.expect("a directory entry").file_name())
        .collect();
    entries.sort();
    let mut found = Vec::new();
    for name in entries {
        let name = name.to_str().expect("a UTF-8 name");
        let child = format!("{relative}/{name}");
        let path = workspace().join(&child);
        if path.is_dir() {
            found.extend(rust_files(&child));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = fs::read_to_string(&path).expect("a source reads");
            found.push((child, text));
        }
    }
    found
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[test]
fn the_kernel_re_exports_the_study_day_crates_own_types() {
    let paths = examined("kernel path(s) to a moved type", kernel_paths());
    assert_eq!(
        own_types(&paths),
        Vec::<&str>::new(),
        "each kernel path names the study day crate's own type"
    );
    // The rollover hour's default is the crate's, by value, at the path the kernel's settings name.
    assert_eq!(
        deck_streak_kernel::study_day::DEFAULT_ROLLOVER_HOUR,
        deck_streak_study_day::study_day::DEFAULT_ROLLOVER_HOUR
    );
    // A path that names a type of its own is refused by name.
    let planted = [
        paths[0],
        (
            "planted::Hour",
            TypeId::of::<u8>(),
            TypeId::of::<deck_streak_study_day::Hour>(),
        ),
    ];
    assert_eq!(own_types(&planted), vec!["planted::Hour"]);
}

#[test]
fn system_clock_stays_in_the_kernel_and_the_instant_leaves_it() {
    let clock = fs::read_to_string(workspace().join(CLOCK)).expect("the kernel's clock reads");
    let crate_sources = examined(
        "Rust file(s) under crates/study-day/src",
        rust_files("crates/study-day/src"),
    );
    examined("line(s) of the kernel's clock", clock.lines().collect());
    assert_eq!(
        clock_findings(&clock, &crate_sources),
        Vec::<String>::new(),
        "the kernel's clock and the crate"
    );
    // The system's clock still reads an instant, through the kernel.
    assert!(SystemClock.now().epoch_millis() > 1_600_000_000_000);

    // Each planted defect is refused by name: a clock that lost its system clock and defines the
    // instant again, and a crate source that names the system's clock and reads the system time.
    let planted_clock = clock
        .replace(TAKEN, "pub struct UtcMillis(i64);")
        .replace("pub struct SystemClock;", "struct Gone;");
    let mut planted_sources = crate_sources.clone();
    planted_sources.push((
        "crates/study-day/src/planted.rs".to_owned(),
        "// SystemClock in a comment is not read\nlet t = SystemClock.now();\n\
         let u = std::time::SystemTime::now();\n"
            .to_owned(),
    ));
    assert_eq!(
        clock_findings(&planted_clock, &planted_sources),
        vec![
            format!("{CLOCK} does not take UtcMillis from the crate"),
            format!("{CLOCK} holds no `pub struct SystemClock;`"),
            format!("{CLOCK} still defines UtcMillis"),
            "crates/study-day/src/planted.rs:2: names SystemClock".to_owned(),
            "crates/study-day/src/planted.rs:3: reads the system time".to_owned(),
        ]
    );
}
