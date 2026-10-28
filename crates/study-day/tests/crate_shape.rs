//! The crate holds the study day, the instant and the date error, each defined here and nowhere in
//! the kernel, with no dependency, no build script and no reach outside its arguments (SPEC-497 A1;
//! ADR-515 D1, D2 and D4).
//!
//! The census reads the crate's manifest line by line and every Rust file under the crate's `src`
//! and the kernel's `src`, by path. It judges the real tree first, prints how much it examined and
//! refuses zero, then refuses each planted defect by name, so a census that went blind fails on its
//! own controls. It reads files only and starts no process.

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined counts on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};

/// The file the move brings into the crate whole.
const STUDY_DAY_FILE: &str = "crates/study-day/src/study_day.rs";

/// The types the move gives the crate: each is defined once in it and never in the kernel.
const MOVED: [&str; 6] = [
    "Hour",
    "IsoDateError",
    "StudyDay",
    "StudyDayRule",
    "UtcMillis",
    "UtcOffset",
];

/// The keywords that define a type of a name.
const DEFINES: [&str; 4] = ["struct", "enum", "type", "trait"];

/// The manifest keys and tables that declare a dependency, a build script or a native link.
const DECLARES: [&str; 5] = [
    "dependencies",
    "build-dependencies",
    "target",
    "build",
    "links",
];

/// The names a source would hold to reach outside its arguments: a clock read, the file system,
/// the environment, the network, a process, a thread, a file read when the crate is built, or code
/// the compiler does not check.
const REACHES_OUT: [&str; 16] = [
    "now",
    "fs",
    "File",
    "env",
    "option_env",
    "net",
    "TcpStream",
    "UdpSocket",
    "process",
    "Command",
    "thread",
    "io",
    "include_str",
    "include_bytes",
    "unsafe",
    "extern",
];

/// One tree as the census reads it: the crate's manifest, whether it has a build script, and the
/// crate's and the kernel's Rust files, each as its path from the root and its text.
struct Tree {
    manifest: String,
    build_script: bool,
    crate_sources: Vec<(String, String)>,
    kernel_sources: Vec<(String, String)>,
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

/// The paths of `sources` that define a type named `name`, once per definition.
fn definitions(sources: &[(String, String)], name: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in sources {
        for line in text.lines() {
            let held = names(line);
            if held
                .windows(2)
                .any(|pair| DEFINES.contains(&pair[0]) && pair[1] == name)
            {
                found.push(path.clone());
            }
        }
    }
    found
}

/// Each line of a manifest that declares a dependency, a build script or a link, by a table header
/// or by a key, plain or dotted, as `Cargo.toml:<line>: <text>`. A comment is not read.
fn manifest_findings(text: &str) -> Vec<String> {
    let mut findings = Vec::new();
    for (number, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        let key = match line.strip_prefix('[') {
            Some(header) => header.trim_matches(['[', ']']),
            None => line.split('=').next().unwrap_or_default(),
        };
        let head = key
            .split('.')
            .next()
            .unwrap_or_default()
            .trim()
            .trim_matches(['"', '\'']);
        if DECLARES.contains(&head) {
            findings.push(format!("Cargo.toml:{}: {line}", number + 1));
        }
    }
    findings
}

/// Everything the tree gets wrong, sorted; empty when the crate holds what the move gives it.
fn judge(tree: &Tree) -> Vec<String> {
    let mut findings = manifest_findings(&tree.manifest);
    if tree.build_script {
        findings.push("crates/study-day/build.rs is a build script".to_owned());
    }
    if !tree
        .crate_sources
        .iter()
        .any(|(path, _)| path == STUDY_DAY_FILE)
    {
        findings.push(format!("{STUDY_DAY_FILE} is missing"));
    }
    for name in MOVED {
        let held = definitions(&tree.crate_sources, name).len();
        if held != 1 {
            findings.push(format!(
                "{name}: defined {held} time(s) under crates/study-day/src, not once"
            ));
        }
        for path in definitions(&tree.kernel_sources, name) {
            findings.push(format!("{name}: still defined in {path}"));
        }
    }
    for (path, text) in &tree.crate_sources {
        for (number, line) in text.lines().enumerate() {
            for name in names(line) {
                if REACHES_OUT.contains(&name) {
                    findings.push(format!("{path}:{}: names {name}", number + 1));
                }
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

/// Prints how many items a census examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The real tree.
fn real_tree() -> Tree {
    let crate_dir = workspace().join("crates/study-day");
    let manifest = fs::read_to_string(crate_dir.join("Cargo.toml")).expect("the manifest reads");
    examined("manifest line(s)", manifest.lines().collect::<Vec<_>>());
    Tree {
        build_script: crate_dir.join("build.rs").exists(),
        crate_sources: examined(
            "Rust file(s) under crates/study-day/src",
            rust_files("crates/study-day/src"),
        ),
        kernel_sources: examined(
            "Rust file(s) under crates/kernel/src",
            rust_files("crates/kernel/src"),
        ),
        manifest,
    }
}

/// A clean tree: the move done, nothing planted.
fn clean_tree() -> Tree {
    Tree {
        manifest: "[package]\nname = \"deck-streak-study-day\"\nversion.workspace = true\n\n\
                   [lints]\nworkspace = true\n\n# [dependencies] in a comment is not read\n\
                   [dev-dependencies]\nserde.workspace = true\n"
            .to_owned(),
        build_script: false,
        crate_sources: vec![
            (
                "crates/study-day/src/lib.rs".to_owned(),
                "pub struct IsoDateError;\n// SystemTime::now in a comment is not read\n"
                    .to_owned(),
            ),
            (
                STUDY_DAY_FILE.to_owned(),
                "pub struct Hour(u8);\npub struct UtcOffset(i16);\npub struct StudyDayRule {}\n\
                 pub struct StudyDay(i64);\npub struct UtcMillis(i64);\nfn f(x: &'static str) {}\n"
                    .to_owned(),
            ),
        ],
        kernel_sources: vec![(
            "crates/kernel/src/clock.rs".to_owned(),
            "pub use deck_streak_study_day::UtcMillis;\npub struct SystemClock;\n".to_owned(),
        )],
    }
}

#[test]
fn the_crate_holds_the_study_day_the_instant_and_the_date_error_with_no_dependency_and_no_io() {
    assert_eq!(
        judge(&real_tree()),
        Vec::<String>::new(),
        "the crate and the kernel"
    );
    assert_eq!(judge(&clean_tree()), Vec::<String>::new(), "the clean tree");

    let mut tree = clean_tree();
    tree.manifest.push_str(
        "[dependencies]\nthiserror.workspace = true\n[target.'cfg(unix)'.dependencies]\n\
         [build-dependencies]\n[dependencies.serde]\ndependencies.extra = \"1\"\n",
    );
    tree.manifest = tree.manifest.replace(
        "version.workspace",
        "build = \"build.rs\"\nversion.workspace",
    );
    tree.build_script = true;
    tree.crate_sources[1].1.push_str("pub enum Hour {}\n");
    tree.crate_sources.remove(0);
    tree.crate_sources.push((
        "crates/study-day/src/planted.rs".to_owned(),
        "use std::{env, io};\nconst T: &str = include_str!(\"t\");\n".to_owned(),
    ));
    tree.kernel_sources.push((
        "crates/kernel/src/error.rs".to_owned(),
        "pub struct IsoDateError;\n".to_owned(),
    ));
    assert_eq!(
        judge(&tree),
        vec![
            "Cargo.toml:12: [dependencies]".to_owned(),
            "Cargo.toml:14: [target.'cfg(unix)'.dependencies]".to_owned(),
            "Cargo.toml:15: [build-dependencies]".to_owned(),
            "Cargo.toml:16: [dependencies.serde]".to_owned(),
            "Cargo.toml:17: dependencies.extra = \"1\"".to_owned(),
            "Cargo.toml:3: build = \"build.rs\"".to_owned(),
            "Hour: defined 2 time(s) under crates/study-day/src, not once".to_owned(),
            "IsoDateError: defined 0 time(s) under crates/study-day/src, not once".to_owned(),
            "IsoDateError: still defined in crates/kernel/src/error.rs".to_owned(),
            "crates/study-day/build.rs is a build script".to_owned(),
            "crates/study-day/src/planted.rs:1: names env".to_owned(),
            "crates/study-day/src/planted.rs:1: names io".to_owned(),
            "crates/study-day/src/planted.rs:2: names include_str".to_owned(),
        ],
        "each planted defect is refused by name"
    );

    let mut tree = clean_tree();
    tree.crate_sources[0]
        .1
        .push_str("use std::fs;\nlet t = std::time::SystemTime::now();\nunsafe {}\n");
    tree.crate_sources.remove(1);
    assert_eq!(
        judge(&tree),
        vec![
            "Hour: defined 0 time(s) under crates/study-day/src, not once".to_owned(),
            "StudyDay: defined 0 time(s) under crates/study-day/src, not once".to_owned(),
            "StudyDayRule: defined 0 time(s) under crates/study-day/src, not once".to_owned(),
            "UtcMillis: defined 0 time(s) under crates/study-day/src, not once".to_owned(),
            "UtcOffset: defined 0 time(s) under crates/study-day/src, not once".to_owned(),
            "crates/study-day/src/lib.rs:3: names fs".to_owned(),
            "crates/study-day/src/lib.rs:4: names now".to_owned(),
            "crates/study-day/src/lib.rs:5: names unsafe".to_owned(),
            format!("{STUDY_DAY_FILE} is missing"),
        ],
        "a missing file, a missing type and a reach outside are refused by name"
    );
}
