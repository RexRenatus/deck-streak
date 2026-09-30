//! The readings use cases create no streak (SPEC-047 A10, R7): no module of the readings context or
//! of its coordination use cases names the streaks context, and nothing of the streaks context
//! names a reading.
//!
//! The census reads every source file of the readings crate and of the coordination readings
//! module, and of the streaks crate; it prints how many it examined and refuses zero. It asserts a
//! positive artifact beside the absence: the tap and settle use cases are in the population. A
//! planted import is refused, by name.

// An integration test is test code: its helpers panic on an unreadable tree, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every Rust file under `directory`, recursively, by path.
fn sources(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(directory).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// The code lines of `source`, comments dropped: prose may name a context, code may not.
fn code(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `source` names the streaks context in code.
fn names_streaks(source: &str) -> bool {
    let code = code(source);
    code.contains("deck_streak_streaks") || code.contains("streaks::")
}

/// Whether `source` names a reading in code.
fn names_reading(source: &str) -> bool {
    let code = code(source);
    code.contains("deck_streak_readings") || code.contains("readings::")
}

#[test]
fn the_reading_use_cases_touch_no_streak() {
    let root = root();
    let mut population = sources(&root.join("crates/readings/src"));
    population.extend(sources(&root.join("crates/coordination/src/readings")));
    let population = examined("readings source file(s)", population);

    // The positive artifact: the tap and the settle use cases are in what was judged.
    for name in ["read_tap.rs", "settle.rs"] {
        assert!(
            population.iter().any(|path| path.ends_with(name)),
            "{name} is not in the examined population"
        );
    }
    for path in &population {
        let text = fs::read_to_string(path).expect("a readable source");
        assert!(
            !names_streaks(&text),
            "{} names the streaks context",
            path.display()
        );
    }

    // And no streak reads a reading.
    for path in examined(
        "streaks source file(s)",
        sources(&root.join("crates/streaks/src")),
    ) {
        let text = fs::read_to_string(&path).expect("a readable source");
        assert!(!names_reading(&text), "{} names a reading", path.display());
    }

    // Controls: a planted import of either shape is refused, prose naming it is not.
    assert!(names_streaks("use deck_streak_streaks::Streak;\n"));
    assert!(names_streaks("let day = streaks::current(db);\n"));
    assert!(!names_streaks("// a reading creates no streaks::Streak\n"));
    assert!(names_reading(
        "use deck_streak_readings::store::SqliteReadings;\n"
    ));
}
