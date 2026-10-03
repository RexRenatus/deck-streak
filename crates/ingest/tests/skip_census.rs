//! Only the skip's write module reaches the engine's card writes or pushes a local change
//! (SPEC-083 A24, guardrail i): the engine's crate is named in code by `engine.rs` alone (ADR-022),
//! inside it the calls that change a card or replace the server's collection sit only in the write
//! port's impl, a normal sync only in the two impls of `RslibEngine` (the syncer's pull and the
//! take's push), and the write port is named by `engine.rs` and the skip's write module alone, with
//! no glob import of the port's module to bring it into scope unnamed.
//!
//! The census reads every crate's `src`, prints how many files it examined and how many engine card
//! write calls it found inside the write port's impl, and refuses zero of either. A planted tree
//! that breaks each rule is refused by file and line; a comment that names the engine's crate is
//! prose and passes.

// An integration test is test code: its helpers panic on an unreadable tree, and it prints the
// examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;
use std::fs;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

/// The one file whose code may name the engine's crate: the port (ADR-022).
const ENGINE: &str = "crates/ingest/src/engine.rs";
/// The skip's write module: beside the port, the one file that may name the write port.
const SKIP_WRITE: &str = "crates/ingest/src/skip_write.rs";
/// The engine's crate, as code names a path into it.
const ENGINE_CRATE: &str = "anki::";
/// The write port.
const PORT: &str = "CollectionWrite";
/// The impl that alone may call the engine's card writes.
const WRITE_IMPL: &str = "impl CollectionWrite for RslibEngine";
/// The impl of the read port, whose normal sync is the syncer's pull.
const READ_IMPL: &str = "impl AnkiEngine for RslibEngine";
/// The engine's calls that change a card, or replace the server's collection with the local one.
const WRITE_CALLS: [&str; 4] = [
    ".set_due_date(",
    ".update_card",
    ".answer_card(",
    ".full_upload(",
];
/// The engine's call that sends the local changes to the server.
const PUSH_CALL: &str = ".normal_sync(";
/// A glob import of the port's module, which brings the write port into scope without naming it.
const GLOB: &str = "engine::*";

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
    let Ok(entries) = fs::read_dir(directory) else {
        return found;
    };
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|name| name == "rs") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// `path` relative to `root`, with `/` separators.
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("a path under the root")
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The code of one line: none for a whole-line comment, and the text before ` //` otherwise.
fn code(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return "";
    }
    trimmed.split(" //").next().unwrap_or_default()
}

/// The lines, by index, of the impl whose header starts with `header`, through its closing brace;
/// `None` when the file holds no such impl.
fn impl_block(lines: &[&str], header: &str) -> Option<RangeInclusive<usize>> {
    let start = lines
        .iter()
        .position(|line| code(line).starts_with(header))?;
    let mut depth = 0_i64;
    let mut opened = false;
    for (index, line) in lines.iter().enumerate().skip(start) {
        for character in code(line).chars() {
            match character {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if opened && depth == 0 {
            return Some(start..=index);
        }
    }
    Some(start..=lines.len().saturating_sub(1))
}

/// Whether `index` lies in `block`.
fn within(block: Option<&RangeInclusive<usize>>, index: usize) -> bool {
    block.is_some_and(|block| block.contains(&index))
}

/// The census of a tree at `root`: every source it read, every file that names the engine's crate
/// or the write port in code, every engine card write call inside the write port's impl, and why
/// each line it refused was refused.
struct Census {
    sources: Vec<String>,
    naming_engine: BTreeSet<String>,
    naming_port: BTreeSet<String>,
    writes_inside: Vec<String>,
    refused: Vec<String>,
}

fn census(root: &Path) -> Census {
    let mut census = Census {
        sources: Vec::new(),
        naming_engine: BTreeSet::new(),
        naming_port: BTreeSet::new(),
        writes_inside: Vec::new(),
        refused: Vec::new(),
    };
    let mut members: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .expect("crates/ is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    members.sort();
    for member in members {
        for source in sources(&member.join("src")) {
            let name = relative(root, &source);
            let text = fs::read_to_string(&source).expect("a readable source");
            judge(&mut census, &name, &text);
            census.sources.push(name);
        }
    }
    census
}

/// Judges one source file, `name`, holding `text`.
fn judge(census: &mut Census, name: &str, text: &str) {
    let lines: Vec<&str> = text.lines().collect();
    let is_engine = name == ENGINE;
    let write_impl = is_engine.then(|| impl_block(&lines, WRITE_IMPL)).flatten();
    let read_impl = is_engine.then(|| impl_block(&lines, READ_IMPL)).flatten();
    for (index, line) in lines.iter().enumerate() {
        let code = code(line);
        let at = format!("{name}:{}", index + 1);
        if code.contains(ENGINE_CRATE) {
            census.naming_engine.insert(name.to_owned());
            if !is_engine {
                census
                    .refused
                    .push(format!("{at} names {ENGINE_CRATE}, and only {ENGINE} may"));
            }
        }
        if is_engine {
            for call in WRITE_CALLS {
                if !code.contains(call) {
                    continue;
                }
                if within(write_impl.as_ref(), index) {
                    census.writes_inside.push(format!("{at} {call}"));
                } else {
                    census
                        .refused
                        .push(format!("{at} calls {call} outside {WRITE_IMPL}"));
                }
            }
            if code.contains(PUSH_CALL)
                && !within(write_impl.as_ref(), index)
                && !within(read_impl.as_ref(), index)
            {
                census.refused.push(format!(
                    "{at} calls {PUSH_CALL} outside {WRITE_IMPL} and {READ_IMPL}"
                ));
            }
        }
        if code.contains(PORT) {
            census.naming_port.insert(name.to_owned());
            if name != ENGINE && name != SKIP_WRITE {
                census.refused.push(format!(
                    "{at} names {PORT}, and only {ENGINE} and {SKIP_WRITE} may"
                ));
            }
        }
        if code.contains(GLOB) {
            census.refused.push(format!(
                "{at} imports {GLOB}, which brings {PORT} in unnamed"
            ));
        }
    }
}

/// Writes `text` to `root`/`path`, making its folders.
fn plant(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().expect("a planted file's folder")).expect("its folder");
    fs::write(file, text).expect("the planted file");
}

/// A planted engine port: the write port's impl calls the engine's Set Due Date and pushes, the
/// read port's impl syncs, and two free functions outside both impls write a card and push.
const PLANTED_ENGINE: &str = "use anki::collection::Collection;\n\
pub trait CollectionWrite {}\n\
impl AnkiEngine for RslibEngine {\n\
    fn normal_sync(&self) {\n\
        col.normal_sync(auth, client);\n\
    }\n\
}\n\
impl CollectionWrite for RslibEngine {\n\
    fn set_due_date(&self) {\n\
        let spec = format!(\"{lo}-{hi}\");\n\
        col.set_due_date(&ids, &spec, None);\n\
    }\n\
    fn write_sync(&self) {\n\
        col.normal_sync(auth, client);\n\
    }\n\
}\n\
fn sneak(col: &mut Collection) {\n\
    col.update_card(&mut card);\n\
}\n\
fn push(col: &mut Collection) {\n\
    col.normal_sync(auth, client);\n\
}\n";

#[test]
fn only_the_skip_write_reaches_an_engine_write_or_a_push() {
    // A planted tree breaking each rule is refused by file and line; the port's own impls, the skip's
    // write module naming the port, and comments naming the engine's crate are not.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(planted.path(), ENGINE, PLANTED_ENGINE);
    plant(
        planted.path(),
        "crates/ingest/src/reader.rs",
        "//! The read never names anki:: in code.\nuse anki::collection::Collection;\n",
    );
    plant(
        planted.path(),
        "crates/ingest/src/sync.rs",
        "use crate::engine::*;\n",
    );
    plant(
        planted.path(),
        "crates/coordination/src/skip/mod.rs",
        "fn take<W: CollectionWrite>(writer: &W) {}\n",
    );
    plant(
        planted.path(),
        SKIP_WRITE,
        "use crate::engine::{CollectionWrite, RslibEngine};\n",
    );
    plant(
        planted.path(),
        "crates/ingest/src/window.rs",
        "// anki::collection stays behind the port\nconst DAYS: i64 = 7; // not anki::\n",
    );
    let refused = census(planted.path());
    examined("planted source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/coordination/src/skip/mod.rs:1 names CollectionWrite, and only \
             crates/ingest/src/engine.rs and crates/ingest/src/skip_write.rs may",
            "crates/ingest/src/engine.rs:18 calls .update_card outside impl CollectionWrite for \
             RslibEngine",
            "crates/ingest/src/engine.rs:21 calls .normal_sync( outside impl CollectionWrite for \
             RslibEngine and impl AnkiEngine for RslibEngine",
            "crates/ingest/src/reader.rs:2 names anki::, and only crates/ingest/src/engine.rs may",
            "crates/ingest/src/sync.rs:1 imports engine::*, which brings CollectionWrite in unnamed",
        ],
        "each planted breach is refused by file and line, and nothing else is"
    );
    assert_eq!(
        refused.writes_inside,
        ["crates/ingest/src/engine.rs:11 .set_due_date("],
        "the planted impl's own Set Due Date is counted inside it"
    );

    // The tree itself: every source examined, the engine's card writes found inside the write port's
    // impl, and zero refused.
    let found = census(&root());
    examined("crate source file(s)", found.sources.clone());
    examined(
        "engine card write call(s) inside impl CollectionWrite for RslibEngine",
        found.writes_inside.clone(),
    );
    assert_eq!(found.refused, Vec::<String>::new());
    // The positive artifacts: the port alone names the engine's crate, and the port names the write
    // port, which only the skip's write module may name beside it.
    let naming: BTreeSet<&str> = found.naming_engine.iter().map(String::as_str).collect();
    assert_eq!(naming, BTreeSet::from([ENGINE]));
    let naming: BTreeSet<&str> = found.naming_port.iter().map(String::as_str).collect();
    assert!(
        naming.contains(ENGINE) && naming.is_subset(&BTreeSet::from([ENGINE, SKIP_WRITE])),
        "the write port is named by the port and at most the skip's write module: {naming:?}"
    );
}
