//! A3 and A4 (SPEC-602 R3, R4): the five refusals, the four planted objects and the bound, each
//! refused before any row is read; the snapshot's own file opens and reads.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_anki_format::opener::USER_VERSION;
use deck_streak_anki_format::{Kind, Limits, Refusal, open, read};
use rusqlite::OpenFlags;
use support::{engine_collection, open_file, plain, recorded, root};

/// A fresh engine collection with `plant` run on the test's own connection. The schema's
/// indexes name `unicase`, so the connection registers a stand-in for the rename to re-read them.
fn planted(dir: &Path, plant: &str) -> PathBuf {
    let source = engine_collection(dir);
    let connection = plain(&source, OpenFlags::SQLITE_OPEN_READ_WRITE);
    connection
        .create_collation("unicase", |a, b| a.to_lowercase().cmp(&b.to_lowercase()))
        .expect("a stand-in collation for writing the plant");
    connection
        .execute_batch(plant)
        .expect("the planted change is written");
    drop(connection);
    source
}

fn refusal_of(source: &Path) -> Option<Refusal> {
    open_file(source, Kind::Collection).err()
}

#[test]
fn a_copy_whose_digest_differs_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = engine_collection(dir.path());
    let mut digest = recorded(&source);
    digest[0] ^= 1;
    let opened = open(&source, &digest, Kind::Collection, &Limits::default()).err();
    assert_eq!(opened, Some(Refusal::Digest), "the digest check");
}

#[test]
fn a_non_empty_wal_beside_the_source_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = engine_collection(dir.path());
    let wal = PathBuf::from(format!("{}-wal", source.display()));
    fs::write(&wal, b"not empty").expect("a planted write-ahead log");
    assert_eq!(
        refusal_of(&source),
        Some(Refusal::Wal),
        "the write-ahead log"
    );
}

#[test]
fn a_collection_of_version_17_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = planted(dir.path(), "UPDATE col SET ver = 17;");
    assert_eq!(refusal_of(&source), Some(Refusal::Version), "col.ver 17");
}

#[test]
fn a_table_set_off_the_snapshot_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = planted(dir.path(), "ALTER TABLE graves RENAME TO graves_moved;");
    assert!(
        matches!(refusal_of(&source), Some(Refusal::TableSet { .. })),
        "a table set without graves is refused"
    );
}

#[test]
fn a_user_version_off_the_pin_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = planted(
        dir.path(),
        &format!("PRAGMA user_version = {};", USER_VERSION + 7),
    );
    assert_eq!(
        refusal_of(&source),
        Some(Refusal::UserVersion),
        "the user version"
    );
}

#[test]
fn a_planted_notes_view_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = planted(
        dir.path(),
        "ALTER TABLE notes RENAME TO notes_moved; \
         CREATE VIEW notes AS SELECT * FROM notes_moved;",
    );
    assert_eq!(
        refusal_of(&source),
        Some(Refusal::Object {
            kind: "view",
            name: "notes".to_owned()
        }),
        "the planted view"
    );
}

#[test]
fn a_planted_trigger_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = planted(
        dir.path(),
        "CREATE TRIGGER planted AFTER UPDATE ON col BEGIN SELECT 1; END;",
    );
    assert_eq!(
        refusal_of(&source),
        Some(Refusal::Object {
            kind: "trigger",
            name: "planted".to_owned()
        }),
        "the planted trigger"
    );
}

#[test]
fn a_planted_virtual_table_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = planted(
        dir.path(),
        "PRAGMA writable_schema = ON; \
         INSERT INTO sqlite_master VALUES \
         ('table', 'planted', 'planted', 0, 'CREATE VIRTUAL TABLE planted USING planted(x)'); \
         PRAGMA writable_schema = OFF;",
    );
    assert_eq!(
        refusal_of(&source),
        Some(Refusal::Object {
            kind: "virtual table",
            name: "planted".to_owned()
        }),
        "the planted virtual table"
    );
}

#[test]
fn a_planted_table_outside_the_snapshot_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = planted(dir.path(), "CREATE TABLE planted (x);");
    assert_eq!(
        refusal_of(&source),
        Some(Refusal::TableSet {
            table: "planted".to_owned()
        }),
        "the planted table"
    );
}

#[test]
fn a_source_over_its_bound_is_refused_while_copied() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = engine_collection(dir.path());
    let length = fs::metadata(&source).expect("the source's length").len();
    let limits = Limits {
        max_bytes: length - 1,
        ..Limits::default()
    };
    let opened = open(&source, &recorded(&source), Kind::Collection, &limits).err();
    assert_eq!(opened, Some(Refusal::OverBound), "the bound");
}

#[test]
fn the_snapshot_file_opens_and_reads_its_one_col_row() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = dir.path().join("snapshot.anki2");
    let sql = fs::read_to_string(root().join("tools/parity-oracle/anki/schema-snapshot.sql"))
        .expect("the snapshot's SQL");
    let statements: Vec<String> = sql
        .lines()
        .filter(|line| !line.starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
        .split(';')
        .map(|statement| statement.trim().to_owned())
        // The snapshot lists the statistics tables SQLite keeps for itself; SQLite makes them,
        // and a statement may not, so the fixture leaves them out.
        .filter(|statement| !statement.is_empty() && !statement.starts_with("CREATE TABLE sqlite_"))
        .collect();
    let connection = plain(
        &source,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    );
    connection
        .create_collation("unicase", |a, b| a.to_lowercase().cmp(&b.to_lowercase()))
        .expect("a stand-in collation for writing the fixture");
    let tables_first = statements
        .iter()
        .filter(|statement| statement.starts_with("CREATE TABLE"))
        .chain(
            statements
                .iter()
                .filter(|statement| !statement.starts_with("CREATE TABLE")),
        );
    for statement in tables_first {
        connection
            .execute_batch(statement)
            .expect("a snapshot statement");
    }
    let golden: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root().join("tools/parity-oracle/goldens/anki.schema.snapshot.json"))
            .expect("the snapshot golden"),
    )
    .expect("the snapshot golden parses");
    let col_ver = golden["cases"][0]["output"]["col_ver"]
        .as_i64()
        .expect("the golden's col_ver");
    assert_eq!(col_ver, 18, "the snapshot golden's col_ver");
    connection
        .execute(
            "INSERT INTO col (id, crt, mod, scm, ver, dty, usn, ls, conf, models, decks, dconf, \
             tags) VALUES (1, 0, 0, 0, ?1, 0, 0, 0, '', '', '', '', '')",
            [col_ver],
        )
        .expect("the one col row");
    drop(connection);
    let reader = open_file(&source, Kind::Collection).expect("the snapshot's file opens");
    let rows = read::collection(&reader).expect("the snapshot's file reads");
    let counts = [
        rows.cards.len(),
        rows.config.len(),
        rows.deck_config.len(),
        rows.decks.len(),
        rows.fields.len(),
        rows.graves.len(),
        rows.notes.len(),
        rows.notetypes.len(),
        rows.revlog.len(),
        rows.tags.len(),
        rows.templates.len(),
    ];
    assert_eq!(
        rows.col.iter().map(|col| col.ver).collect::<Vec<_>>(),
        [18],
        "the one col row, ver 18"
    );
    assert_eq!(counts, [0; 11], "every other table is empty");
}
