//! Only the wallet writes the coin ledger (SPEC-082 A9, R1): no crate but economy names
//! `coin_ledger` in its code, and no migration but economy's names it, so the wallet's ports are
//! the one writer of a coin movement and the economy data-rights port the one eraser, beside them
//! (ADR-308, CHARTER 13).
//!
//! The census reads every crate's `src` and every migration of the repository, prints how many it
//! examined and refuses zero. It asserts a positive artifact beside the absence: exactly the
//! wallet, the economy data-rights port and economy's migration name the table. A planted crate
//! and a planted migration that name it are refused, by name.

// An integration test is test code: its helpers panic on an unreadable tree, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The table the census guards.
const TABLE: &str = "coin_ledger";
/// The context that owns it (docs/CONTEXT-MAP.md).
const OWNER: &str = "economy";

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

/// Every file under `directory` whose name ends in `extension`, recursively, by path.
fn files(directory: &Path, extension: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(directory) else {
        return found;
    };
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(files(&path, extension));
        } else if path
            .extension()
            .is_some_and(|name| name.to_str() == Some(extension))
        {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Whether Rust source text names the table on a line of code. A whole-line comment is prose, and
/// so is what follows ` //` on a line.
fn rust_names_the_table(text: &str) -> bool {
    text.lines().any(|line| {
        let code = line.trim_start();
        if code.starts_with("//") {
            return false;
        }
        let code = code.split(" //").next().unwrap_or_default();
        code.contains(TABLE)
    })
}

/// Whether SQL text names the table outside a `--` comment.
fn sql_names_the_table(text: &str) -> bool {
    text.lines()
        .any(|line| line.split("--").next().unwrap_or_default().contains(TABLE))
}

/// The census of a tree at `root`: every file it read, every file that names the table, and why
/// each one it refused was refused.
struct Census {
    sources: Vec<String>,
    migrations: Vec<String>,
    naming: BTreeSet<String>,
    refused: Vec<String>,
}

fn census(root: &Path) -> Census {
    let mut census = Census {
        sources: Vec::new(),
        migrations: Vec::new(),
        naming: BTreeSet::new(),
        refused: Vec::new(),
    };
    let crates = root.join("crates");
    let mut members: Vec<PathBuf> = fs::read_dir(&crates)
        .expect("crates/ is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.is_dir())
        .collect();
    members.sort();
    for member in members {
        let context = member
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a UTF-8 crate directory")
            .to_owned();
        for source in files(&member.join("src"), "rs") {
            let name = relative(root, &source);
            let text = fs::read_to_string(&source).expect("a readable source");
            if rust_names_the_table(&text) {
                census.naming.insert(name.clone());
                if context != OWNER {
                    census
                        .refused
                        .push(format!("{name} names {TABLE}, and only {OWNER}'s code may"));
                }
            }
            census.sources.push(name);
        }
    }
    for migration in files(&root.join("migrations"), "sql") {
        let name = relative(root, &migration);
        let file = migration
            .file_name()
            .and_then(|name| name.to_str())
            .expect("a UTF-8 migration name");
        // `<SPEC number and sequence>_<owning context>_<slug>.sql` (ADR-020).
        let context = file.split('_').nth(1).unwrap_or_default();
        let text = fs::read_to_string(&migration).expect("a readable migration");
        if sql_names_the_table(&text) {
            census.naming.insert(name.clone());
            if context != OWNER {
                census.refused.push(format!(
                    "{name} names {TABLE}, and only {OWNER}'s migrations may"
                ));
            }
        }
        census.migrations.push(name);
    }
    census
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

/// Writes `text` to `root`/`path`, making its folders.
fn plant(root: &Path, path: &str, text: &str) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().expect("a planted file's folder")).expect("its folder");
    fs::write(file, text).expect("the planted file");
}

#[test]
fn only_the_wallet_writes_the_coin_ledger() {
    let found = census(&root());
    examined("crate source file(s)", found.sources.clone());
    examined("migration(s)", found.migrations.clone());
    assert_eq!(found.refused, Vec::<String>::new());
    // The positive artifact: the wallet's ports write the table, the data-rights port exports and
    // erases it, economy's own migration creates it, the skip tariff reads what one skip paid by its
    // source and reference and writes nothing (SPEC-083 T4), and no other file names it.
    let naming: BTreeSet<&str> = found.naming.iter().map(String::as_str).collect();
    assert_eq!(
        naming,
        BTreeSet::from([
            "crates/economy/src/data_rights.rs",
            "crates/economy/src/tariff.rs",
            "crates/economy/src/wallet.rs",
            "migrations/008201_economy_wallet_and_shop.sql",
        ]),
        "the wallet, the data-rights port, the tariff's read and economy's migration name the table, \
         and nothing else"
    );

    // A planted crate whose code names the table, and a planted migration of another context that
    // does, are refused by name; the owner's own files, and a mention in a comment, are not.
    let planted = tempfile::tempdir().expect("a temporary directory");
    plant(
        planted.path(),
        "crates/economy/src/wallet.rs",
        "const Q: &str = \"INSERT INTO coin_ledger (delta) VALUES (1)\";\n",
    );
    plant(
        planted.path(),
        "crates/streaks/src/freeze.rs",
        "//! A freeze never writes coin_ledger itself: it asks the shop.\n\
         fn buy() {\n    let _ = sqlx::query(\"INSERT INTO coin_ledger (delta) VALUES (-150)\"); // a debit\n}\n",
    );
    plant(
        planted.path(),
        "crates/quests/src/chest.rs",
        "fn open() {} // not coin_ledger: the wallet pays a chest\n",
    );
    plant(
        planted.path(),
        "migrations/008201_economy_wallet_and_shop.sql",
        "CREATE TABLE coin_ledger (id INTEGER PRIMARY KEY) STRICT;\n",
    );
    plant(
        planted.path(),
        "migrations/999901_streaks_refund.sql",
        "-- a planted migration of another context\nDELETE FROM coin_ledger;\n",
    );
    let refused = census(planted.path());
    examined("planted crate source file(s)", refused.sources.clone());
    assert_eq!(
        refused.refused,
        [
            "crates/streaks/src/freeze.rs names coin_ledger, and only economy's code may",
            "migrations/999901_streaks_refund.sql names coin_ledger, and only economy's \
             migrations may",
        ]
    );
}
