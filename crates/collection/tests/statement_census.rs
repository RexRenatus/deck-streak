//! The statement census (SPEC-512 A1 and A5; ADR-530 D2 and D7): every registered statement of
//! the store prepares against a store at head, no source of the crate holds SQL text outside a
//! registered constant, and no file of the workspace but the opener builds a `SQLite` connection.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::collections::BTreeSet;
use std::path::Path;

use deck_streak_collection::Store;
use deck_streak_collection::sql::STATEMENTS;
use support::{SERVER, examined, plant, root, rust_files, workspace_sources};

/// The files whose constants may hold SQL text.
const STATEMENT_FILES: [&str; 2] = [
    "crates/collection/src/sql.rs",
    "crates/collection/src/data_rights.rs",
];

/// The words a statement of the store starts with.
const SQL_WORDS: [&str; 10] = [
    "SELECT", "INSERT", "UPDATE", "DELETE", "PRAGMA", "VACUUM", "CREATE", "WITH", "BEGIN", "DROP",
];

/// The one file that may build a connection.
const OPENER: &str = "crates/collection/src/store.rs";

/// The calls that build a `SQLite` connection.
const CONSTRUCTORS: [&str; 4] = [
    "Connection::open(",
    "Connection::open_with_flags(",
    "Connection::open_in_memory",
    "Connection::open_with_flags_and_vfs(",
];

/// Whether a string literal starting at `rest` (just past its opening quote) is SQL text: it opens
/// with a statement's first word, written in capitals, or in any case and followed by more words
/// (so `"delete"`, a word of prose, is not a statement, and `"select id FROM decks"` is).
fn is_sql(rest: &str) -> bool {
    let head: String = rest.chars().take_while(char::is_ascii_alphabetic).collect();
    let continues = rest[head.len()..].starts_with(' ');
    SQL_WORDS
        .iter()
        .any(|word| head == *word || (continues && head.eq_ignore_ascii_case(word)))
}

/// Every SQL literal of `sources`: `(path, line, the constant it is the value of, if any)`.
fn sql_literals(sources: &[(String, String)]) -> Vec<(String, usize, Option<String>)> {
    let mut found = Vec::new();
    for (path, text) in sources {
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let Some(at) = code.find('"') else {
                continue;
            };
            if !is_sql(&code[at + 1..]) {
                continue;
            }
            // The constant whose value this literal is: on this line, or on the line before.
            let owner = [
                Some(*line),
                index.checked_sub(1).map(|before| lines[before]),
            ]
            .into_iter()
            .flatten()
            .find_map(|candidate| {
                let after = candidate.trim_start().strip_prefix("pub const ")?;
                let (name, kind) = after.split_once(':')?;
                kind.trim_start()
                    .starts_with("&str =")
                    .then(|| name.trim().to_owned())
            });
            found.push((path.clone(), index + 1, owner));
        }
    }
    found
}

/// Every refusal of the crate's SQL texts against the register `registered`.
fn unregistered(sources: &[(String, String)], registered: &BTreeSet<&str>) -> Vec<String> {
    let mut refused = Vec::new();
    for (path, line, owner) in sql_literals(sources) {
        match owner {
            Some(name) if STATEMENT_FILES.contains(&path.as_str()) => {
                if !registered.contains(name.as_str()) {
                    refused.push(format!("{path}:{line}: {name} is not registered"));
                }
            }
            _ => refused.push(format!(
                "{path}:{line}: SQL text outside a registered constant"
            )),
        }
    }
    refused
}

/// The crate's own sources under `root`.
fn crate_sources(root: &Path) -> Vec<(String, String)> {
    let mut sources = Vec::new();
    rust_files(root, &root.join("crates/collection/src"), &mut sources);
    sources
}

#[test]
fn every_statement_of_the_store_prepares_against_the_migrated_schema() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("census.store");
    let store = Store::open(&path, SERVER).expect("the store opens");
    let mut findings = Vec::new();
    for statement in examined("registered statement(s)", STATEMENTS.to_vec()) {
        if let Err(error) = store.check_statement(statement.sql) {
            findings.push(format!("{}: {error:?}", statement.name));
        }
    }
    let names: BTreeSet<&str> = STATEMENTS.iter().map(|statement| statement.name).collect();
    if names.len() != STATEMENTS.len() {
        findings.push("a name is registered twice".to_owned());
    }
    assert_eq!(
        findings,
        Vec::<String>::new(),
        "statements that do not prepare"
    );

    // A planted statement with a misspelled column does not prepare.
    assert!(
        store
            .check_statement("SELECT id, nmae FROM decks ORDER BY id")
            .is_err(),
        "a misspelled column prepares"
    );
}

#[test]
fn every_sql_text_of_the_crate_is_a_registered_statement() {
    let root = root();
    let sources = crate_sources(&root);
    let registered: BTreeSet<&str> = STATEMENTS.iter().map(|statement| statement.name).collect();
    let literals = examined("SQL literal(s) of the crate", sql_literals(&sources));
    let constants: BTreeSet<String> = literals
        .iter()
        .filter_map(|(_, _, owner)| owner.clone())
        .collect();
    assert_eq!(unregistered(&sources, &registered), Vec::<String>::new());
    let missing: Vec<&str> = registered
        .iter()
        .filter(|name| !constants.contains(**name))
        .copied()
        .collect();
    assert_eq!(
        missing,
        Vec::<&str>::new(),
        "registered names with no constant"
    );

    // A planted crate: a statement outside a constant, and a constant the register lacks.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/collection/src/sql.rs",
        "/// Registered.\npub const KNOWN: &str = \"SELECT 1\";\n/// Not registered.\npub const \
         STRAY: &str =\n    \"select id FROM decks\";\n",
    );
    plant(
        planted.path(),
        "crates/collection/src/store.rs",
        "fn open() { conn.execute_batch(\"PRAGMA foreign_keys = ON\"); }\n",
    );
    let refused = unregistered(&crate_sources(planted.path()), &BTreeSet::from(["KNOWN"]));
    assert_eq!(
        refused,
        [
            "crates/collection/src/sql.rs:5: STRAY is not registered",
            "crates/collection/src/store.rs:1: SQL text outside a registered constant",
        ],
        "the planted crate"
    );
}

/// Every place outside the opener that builds a connection, in `sources`.
fn builders(sources: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in sources {
        if path == OPENER {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if CONSTRUCTORS.iter().any(|call| code.contains(call)) {
                found.push(format!("{path}:{}", index + 1));
            }
        }
    }
    found
}

#[test]
fn no_other_file_of_the_workspace_builds_a_sqlite_connection() {
    let root = root();
    let sources = examined("workspace source file(s)", workspace_sources(&root));
    assert_eq!(
        builders(&sources),
        Vec::<String>::new(),
        "connection builders"
    );
    let opener = sources
        .iter()
        .find(|(path, _)| path == OPENER)
        .map(|(_, text)| text.as_str())
        .unwrap_or_default();
    let calls = opener
        .lines()
        .filter(|line| CONSTRUCTORS.iter().any(|call| line.contains(call)))
        .count();
    assert!(
        calls >= 2,
        "the opener builds the store's and the foreign file's connections"
    );

    // A planted tree: a second builder is refused by file and line, a comment is prose.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        OPENER,
        "fn open() { Connection::open(path); }\n",
    );
    plant(
        planted.path(),
        "crates/ingest/src/schema18.rs",
        "// Connection::open( is the opener's alone\nfn read() { \
         rusqlite::Connection::open_with_flags(path, flags); }\n",
    );
    assert_eq!(
        builders(&workspace_sources(planted.path())),
        ["crates/ingest/src/schema18.rs:2"],
        "the planted builder"
    );
}
