//! A2 (SPEC-602 R2): the defensive opener's settings, read back on the open connection; a write
//! refused; the source unchanged; a damaged page refused at the quick check.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use deck_streak_anki_format::{Kind, Limits, Refusal, open, read};
use rusqlite::OpenFlags;
use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use support::{engine_collection, examined, open_file, plain, recorded};

fn pragma(connection: &rusqlite::Connection, name: &str) -> i64 {
    connection
        .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
        .unwrap_or(-1)
}

#[test]
fn the_opener_sets_every_defensive_setting_on_the_copy() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = engine_collection(dir.path());
    let limits = Limits::default();
    let reader =
        open(&source, &recorded(&source), Kind::Collection, &limits).expect("the collection opens");
    let connection = reader.connection();
    let read_back = examined(
        "setting(s)",
        vec![
            (
                "defensive",
                format!(
                    "{:?}",
                    connection.db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE)
                ),
                "Ok(true)".to_owned(),
            ),
            (
                "trusted_schema",
                format!(
                    "{:?}",
                    connection.db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA)
                ),
                "Ok(false)".to_owned(),
            ),
            (
                "query_only",
                pragma(connection, "query_only").to_string(),
                "1".to_owned(),
            ),
            (
                "cell_size_check",
                pragma(connection, "cell_size_check").to_string(),
                "1".to_owned(),
            ),
            (
                "mmap_size",
                pragma(connection, "mmap_size").to_string(),
                "0".to_owned(),
            ),
            (
                "length",
                format!("{:?}", connection.limit(Limit::SQLITE_LIMIT_LENGTH)),
                format!("Ok({})", limits.max_length),
            ),
            (
                "sql_length",
                format!("{:?}", connection.limit(Limit::SQLITE_LIMIT_SQL_LENGTH)),
                format!("Ok({})", limits.max_sql_length),
            ),
            (
                "unicase",
                format!(
                    "{:?}",
                    connection.query_row("SELECT 'a' = 'A' COLLATE unicase", [], |row| row
                        .get::<_, i64>(0))
                ),
                "Ok(1)".to_owned(),
            ),
            (
                "a private copy",
                (reader.copy_path() != source.as_path()).to_string(),
                "true".to_owned(),
            ),
            (
                "the copy's mode",
                fs::metadata(reader.copy_path())
                    .map(|meta| format!("{:o}", meta.permissions().mode() & 0o777))
                    .unwrap_or_default(),
                "600".to_owned(),
            ),
        ],
    );
    let wrong: Vec<String> = read_back
        .iter()
        .filter(|(_, found, wanted)| found != wanted)
        .map(|(name, found, wanted)| format!("{name}: {found}, not {wanted}"))
        .collect();
    assert_eq!(wrong, Vec::<String>::new(), "the settings on the copy");
}

#[test]
fn a_write_through_the_reader_is_refused_as_read_only() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = engine_collection(dir.path());
    let reader = open_file(&source, Kind::Collection).expect("the collection opens");
    let written = reader
        .connection()
        .execute("UPDATE col SET crt = crt + 1", []);
    assert!(
        matches!(
            &written,
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::ReadOnly
        ),
        "a write is refused as read-only, not {written:?}"
    );
}

#[test]
fn the_source_keeps_its_bytes_and_modification_time() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = engine_collection(dir.path());
    let bytes = fs::read(&source).expect("the source reads");
    let modified = fs::metadata(&source)
        .and_then(|meta| meta.modified())
        .expect("a modification time");
    let reader = open_file(&source, Kind::Collection).expect("the collection opens");
    let rows = read::collection(&reader).expect("the rows read");
    assert_eq!(rows.col.len(), 1, "the one col row is read");
    assert_ne!(
        reader.copy_path(),
        source.as_path(),
        "the reader reads a private copy, never the source"
    );
    drop(reader);
    assert_eq!(
        fs::read(&source).expect("the source reads"),
        bytes,
        "the bytes"
    );
    assert_eq!(
        fs::metadata(&source)
            .and_then(|meta| meta.modified())
            .expect("a modification time"),
        modified,
        "the modification time"
    );
}

#[test]
fn a_copy_with_a_damaged_page_is_refused_at_the_quick_check() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = engine_collection(dir.path());
    let (page_size, root) = {
        let connection = plain(&source, OpenFlags::SQLITE_OPEN_READ_ONLY);
        let page_size: u64 = connection
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .expect("a page size");
        let root: u64 = connection
            .query_row(
                "SELECT rootpage FROM sqlite_master WHERE type = 'table' AND name = 'col'",
                [],
                |row| row.get(0),
            )
            .expect("the col table's root page");
        (page_size, root)
    };
    let mut bytes = fs::read(&source).expect("the source reads");
    let start = usize::try_from((root - 1) * page_size).expect("an offset");
    for byte in &mut bytes[start..start + 16] {
        *byte = 0xff;
    }
    fs::write(&source, &bytes).expect("the damaged page is written");
    let opened = open(
        &source,
        &recorded(&source),
        Kind::Collection,
        &Limits::default(),
    );
    assert!(
        matches!(opened, Err(Refusal::QuickCheck)),
        "the damaged page is refused at the quick check, not {:?}",
        opened.map(|reader| reader.copy_path().display().to_string())
    );
}

#[test]
fn two_readers_held_at_once_each_hold_their_own_private_copy() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = engine_collection(dir.path());
    let first = open_file(&source, Kind::Collection).expect("the first reader opens");
    let second = open_file(&source, Kind::Collection).expect("the second reader opens");
    let folders = [first.copy_path(), second.copy_path()].map(|copy| {
        copy.parent()
            .expect("the copy sits in a folder")
            .to_path_buf()
    });
    assert_ne!(folders[0], folders[1], "each reader its own folder");
    for folder in &folders {
        assert!(folder.is_dir(), "{} is held", folder.display());
    }
    let read =
        [&first, &second].map(|reader| read::collection(reader).expect("the copy reads").col.len());
    assert_eq!(read, [1, 1], "both copies read their one col row");
}

#[test]
fn dropping_the_reader_removes_its_copy_and_its_folder() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = engine_collection(dir.path());
    let reader = open_file(&source, Kind::Collection).expect("the reader opens");
    let copy = reader.copy_path().to_path_buf();
    let folder = copy
        .parent()
        .expect("the copy sits in a folder")
        .to_path_buf();
    assert!(copy.is_file(), "the copy is held while the reader lives");
    drop(reader);
    assert!(!copy.exists(), "the copy is removed with the reader");
    assert!(!folder.exists(), "the folder is removed with the reader");
    assert!(source.is_file(), "the source stays");
}
