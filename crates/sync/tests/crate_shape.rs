//! The crate depends on nothing and reaches nothing outside its arguments (SPEC-509 A10; the
//! design's contract item 10, "no I/O dependency").
//!
//! The census reads the crate's manifest line by line and every Rust file under its `src`, by
//! path. It judges the real tree first, prints how much it examined and refuses zero, then refuses
//! each planted defect by name, so a census that went blind fails on its own controls. It reads
//! files only and starts no process. The builds for the clients' targets are CI's.

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::fs;
use std::path::{Path, PathBuf};

/// The manifest keys and tables that declare a dependency, a build script or a native link.
const DECLARES: [&str; 5] = [
    "dependencies",
    "build-dependencies",
    "target",
    "build",
    "links",
];

/// The names a source would hold to reach outside its arguments: a clock read, the file system,
/// the environment, the network, a process, a thread, randomness, a file read when the crate is
/// built, or code the compiler does not check.
const REACHES_OUT: [&str; 20] = [
    "SystemTime",
    "Instant",
    "time",
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
    "rand",
    "getrandom",
    "include_str",
    "include_bytes",
    "unsafe",
    "extern",
];

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

/// The crate as the census reads it: its manifest, whether it has a build script, and its Rust
/// files, each as its path from the root and its text.
struct Tree {
    manifest: String,
    build_script: bool,
    sources: Vec<(String, String)>,
}

/// Everything the tree gets wrong, sorted; empty when the crate depends on nothing and reaches
/// nothing.
fn judge(tree: &Tree) -> Vec<String> {
    let mut findings = manifest_findings(&tree.manifest);
    if tree.build_script {
        findings.push("crates/sync/build.rs is a build script".to_owned());
    }
    for (path, text) in &tree.sources {
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

/// The crate's directory.
fn crate_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every `.rs` file at or under `relative` in the crate, sorted, as its path from the root and
/// its text.
fn rust_files(relative: &str) -> Vec<(String, String)> {
    let mut entries: Vec<_> = fs::read_dir(crate_dir().join(relative))
        .unwrap_or_else(|error| panic!("{relative} reads: {error}"))
        .map(|entry| entry.expect("a directory entry").file_name())
        .collect();
    entries.sort();
    let mut found = Vec::new();
    for name in entries {
        let name = name.to_str().expect("a UTF-8 name");
        let child = format!("{relative}/{name}");
        let path = crate_dir().join(&child);
        if path.is_dir() {
            found.extend(rust_files(&child));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = fs::read_to_string(&path).expect("a source reads");
            found.push((format!("crates/sync/{child}"), text));
        }
    }
    found
}

/// Prints how many items a census examined and refuses zero (the examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[test]
fn the_crate_depends_on_nothing_and_reaches_nothing_outside_its_arguments() {
    let manifest = fs::read_to_string(crate_dir().join("Cargo.toml")).expect("the manifest reads");
    examined("manifest line(s)", manifest.lines().collect::<Vec<_>>());
    let real = Tree {
        build_script: crate_dir().join("build.rs").exists(),
        sources: examined("Rust file(s) under crates/sync/src", rust_files("src")),
        manifest,
    };
    assert_eq!(judge(&real), Vec::<String>::new(), "the crate");

    let clean = || Tree {
        manifest: "[package]\nname = \"deck-streak-sync\"\n\n[lints]\nworkspace = true\n\n\
                   # [dependencies] in a comment is not read\n[dev-dependencies]\n\
                   serde_json.workspace = true\n\n[features]\nkani-witness-x = []\n"
            .to_owned(),
        build_script: false,
        sources: vec![(
            "crates/sync/src/lib.rs".to_owned(),
            "pub fn tick(now: i64) -> i64 { now }\n// SystemTime::now in a comment is not read\n\
             const T: &'static str = \"a\";\n"
                .to_owned(),
        )],
    };
    assert_eq!(judge(&clean()), Vec::<String>::new(), "the clean tree");

    let mut planted = clean();
    planted.manifest.push_str(
        "[dependencies]\nthiserror.workspace = true\n[target.'cfg(unix)'.dependencies]\n\
         [build-dependencies]\n",
    );
    planted.manifest = planted
        .manifest
        .replace("name = ", "build = \"build.rs\"\nname = ");
    planted.build_script = true;
    planted.sources.push((
        "crates/sync/src/planted.rs".to_owned(),
        "use std::{env, io};\nlet t = std::time::SystemTime::now();\n\
         const B: &[u8] = include_bytes!(\"b\");\nunsafe {}\n"
            .to_owned(),
    ));
    assert_eq!(
        judge(&planted),
        vec![
            "Cargo.toml:14: [dependencies]".to_owned(),
            "Cargo.toml:16: [target.'cfg(unix)'.dependencies]".to_owned(),
            "Cargo.toml:17: [build-dependencies]".to_owned(),
            "Cargo.toml:2: build = \"build.rs\"".to_owned(),
            "crates/sync/build.rs is a build script".to_owned(),
            "crates/sync/src/planted.rs:1: names env".to_owned(),
            "crates/sync/src/planted.rs:1: names io".to_owned(),
            "crates/sync/src/planted.rs:2: names SystemTime".to_owned(),
            "crates/sync/src/planted.rs:2: names time".to_owned(),
            "crates/sync/src/planted.rs:3: names include_bytes".to_owned(),
            "crates/sync/src/planted.rs:4: names unsafe".to_owned(),
        ],
        "each planted defect is refused by name"
    );
}
