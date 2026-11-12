//! The register of the files that name the due-card scores input (SPEC-414 R7, A8; ADR-428 D7),
//! ADR-339's shape test: no score input reaches a write path of the engine core.
//!
//! Among the core's sources and both client adapters' sources, only the reorder module names the
//! input, and that module names no engine crate, no other module of the core and no input or
//! output of the process. A wiring delivery adds its own file to the register in the change that
//! names the input. The test also judges a planted tree, so a census that went blind fails it.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

/// The input's type as source code spells it.
const INPUT: &str = "DueScores";

/// The reorder module: the one file that holds the input and the function that reads it.
const MODULE: &str = "crates/engine-core/src/reorder.rs";

/// Every file that may name the input. A wiring delivery adds its own file here (ADR-428 D7).
const REGISTER: [&str; 1] = [MODULE];

/// The directories the register judges: the core's sources and both client adapters' sources.
const WALKED: [&str; 3] = [
    "crates/engine-core/src",
    "crates/ffi/src",
    "crates/web-engine/src",
];

/// What the reorder module never names: the engine's crates, the core's other modules, and the
/// process's files, streams, sockets and children.
const REACHES: [&str; 8] = [
    "anki::",
    "anki_proto",
    "crate::",
    "super::",
    "std::fs",
    "std::io",
    "std::net",
    "std::process",
];

/// Every `.rs` file under `root.join(dir)`, as `/`-separated paths relative to `root`, sorted.
fn sources(root: &Path, dir: &str) -> Vec<String> {
    let mut pending = vec![root.join(dir)];
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

/// Every source under the walked directories of `root`, directory by directory.
fn walk(root: &Path) -> Vec<String> {
    WALKED.iter().flat_map(|dir| sources(root, dir)).collect()
}

/// Each of `files` (relative to `root`) whose text holds `name`.
fn naming(root: &Path, files: &[String], name: &str) -> Vec<String> {
    files
        .iter()
        .filter(|file| {
            fs::read_to_string(root.join(file.as_str()))
                .expect("a walked source reads")
                .contains(name)
        })
        .cloned()
        .collect()
}

/// Each walked file outside the register that names the input, refused by name.
fn reader_refusals(root: &Path, walked: &[String]) -> Vec<String> {
    naming(root, walked, INPUT)
        .into_iter()
        .filter(|file| !REGISTER.contains(&file.as_str()))
        .map(|file| format!("{file} names {INPUT}: only the register's files may"))
        .collect()
}

/// Each name of `REACHES` the reorder module under `root` holds, refused by name.
fn reach_refusals(root: &Path) -> Vec<String> {
    let text = fs::read_to_string(root.join(MODULE)).expect("the reorder module reads");
    REACHES
        .iter()
        .filter(|name| text.contains(**name))
        .map(|name| format!("{MODULE} names {name}: the reorder reaches no engine, module or I/O"))
        .collect()
}

/// A tree of its own for one test, holding `files`.
fn plant(test: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = support::scratch("engine-core-reorder-shape", test);
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().expect("a planted file has a directory"))
            .expect("a planted directory");
        fs::write(path, text).expect("a planted file writes");
    }
    root
}

#[test]
fn the_input_is_named_by_the_reorder_module_alone_and_it_reaches_no_engine() {
    let workspace = support::workspace();
    let walked = walk(&workspace);
    assert_eq!(
        naming(&workspace, &walked, INPUT),
        vec![MODULE.to_owned()],
        "the core's and the client adapters' sources that name {INPUT}"
    );
    assert_eq!(
        reader_refusals(&workspace, &walked),
        Vec::<String>::new(),
        "the register's census"
    );
    assert_eq!(
        reach_refusals(&workspace),
        Vec::<String>::new(),
        "what the reorder module names"
    );
    support::examined("core and client adapter source file(s)", walked);

    let test = "the_input_is_named_by_the_reorder_module_alone_and_it_reaches_no_engine";
    let planted = plant(
        test,
        &[
            (
                MODULE,
                "use anki::card::Card;\nuse crate::dispatch::Dispatcher;\npub struct DueScores;\n",
            ),
            (
                "crates/engine-core/src/dispatch.rs",
                "fn due(input: &DueScores) {}\n",
            ),
            ("crates/ffi/src/engine.rs", "pub fn open() {}\n"),
            (
                "crates/web-engine/src/wasm.rs",
                "use deck_streak_engine_core::reorder::DueScores;\n",
            ),
        ],
    );
    let planted_walk = walk(&planted);
    assert_eq!(
        reader_refusals(&planted, &planted_walk),
        vec![
            "crates/engine-core/src/dispatch.rs names DueScores: only the register's files may"
                .to_owned(),
            "crates/web-engine/src/wasm.rs names DueScores: only the register's files may"
                .to_owned(),
        ],
        "each planted second reader of the input is refused by name"
    );
    assert_eq!(
        reach_refusals(&planted),
        vec![
            format!("{MODULE} names anki::: the reorder reaches no engine, module or I/O"),
            format!("{MODULE} names crate::: the reorder reaches no engine, module or I/O"),
        ],
        "a planted engine import and a planted core import are each refused by name"
    );
    support::examined("planted source file(s)", planted_walk);
}
