//! The one opener (SPEC-512 A1, A2 and A8; ADR-530 D2 to D4): its pragmas by role, its read-only
//! reader, its role check, its migrations to head under `user_version`, and its refusal of a newer
//! store or of another application's file, which it leaves byte for byte as it was.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::fs;
use std::path::Path;

use deck_streak_collection::migrate::{HEAD, MIGRATIONS};
use deck_streak_collection::{
    APPLICATION_ID, BUSY_TIMEOUT_MS, ForeignFile, Slot, Store, StoreError,
};
use support::{SERVER, device, examined, raw, root};

/// The file's bytes, and whether a WAL or shared-memory file sits beside it.
fn snapshot(path: &Path) -> (Vec<u8>, bool, bool) {
    let mut wal = path.as_os_str().to_owned();
    wal.push("-wal");
    let mut shm = path.as_os_str().to_owned();
    shm.push("-shm");
    (
        fs::read(path).expect("the file is readable"),
        Path::new(&wal).exists(),
        Path::new(&shm).exists(),
    )
}

#[test]
fn the_opener_sets_each_roles_pragmas_and_stamps_the_application_id() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut findings = Vec::new();
    let roles = [("server", SERVER, 2), ("device", device(1), 1)];
    for (name, role, synchronous) in examined("role(s)", roles.to_vec()) {
        let path = dir.path().join(format!("{name}.store"));
        let store = match Store::open(&path, role) {
            Ok(store) => store,
            Err(error) => {
                findings.push(format!("{name}: the opener refused: {error}"));
                continue;
            }
        };
        let settings = store.settings().expect("the settings read back");
        let checks = [
            ("application_id", settings.application_id == APPLICATION_ID),
            ("journal_mode wal", settings.journal_mode == "wal"),
            ("synchronous", settings.synchronous == synchronous),
            ("foreign_keys", settings.foreign_keys),
            ("secure_delete", settings.secure_delete),
            ("writable", !settings.query_only),
            (
                "busy_timeout",
                settings.busy_timeout_ms == i64::try_from(BUSY_TIMEOUT_MS).expect("a timeout"),
            ),
        ];
        for (check, held) in checks {
            if !held {
                findings.push(format!("{name}: {check} is not set ({settings:?})"));
            }
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the pragmas of each role");
}

#[test]
fn the_reader_opens_query_only_and_refuses_every_write() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("server.store");
    drop(Store::open(&path, SERVER).expect("the store is made"));
    let mut findings = Vec::new();

    let mut reader = Store::open_reader(&path).expect("the reader opens the store");
    let settings = reader.settings().expect("the settings read back");
    if !settings.query_only {
        findings.push(format!("the reader is not query_only ({settings:?})"));
    }
    if reader.role().is_some() {
        findings.push("the reader holds a role".to_owned());
    }
    match reader.write(|_| Ok(())) {
        Err(StoreError::ReadOnly) => {}
        other => findings.push(format!("a write through the reader answered {other:?}")),
    }

    // A reader never makes a file.
    let missing = dir.path().join("missing.store");
    if Store::open_reader(&missing).is_ok() {
        findings.push("the reader opened a path that holds nothing".to_owned());
    }
    if missing.exists() {
        findings.push("the reader made a file at a path that held nothing".to_owned());
    }
    assert_eq!(findings, Vec::<String>::new(), "the reader");
}

#[test]
fn a_store_opened_in_another_role_is_refused() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let server = dir.path().join("server.store");
    let one = dir.path().join("device-one.store");
    let mut findings = Vec::new();
    match Store::open(&server, SERVER) {
        Ok(store) => drop(store),
        Err(error) => findings.push(format!("the server's store was refused: {error}")),
    }
    match Store::open(&one, device(1)) {
        Ok(store) => drop(store),
        Err(error) => findings.push(format!("the device's store was refused: {error}")),
    }
    let cases = [
        ("the server's store as a device's", &server, device(1)),
        ("a device's store as the server's", &one, SERVER),
        ("a device's store in another slot", &one, device(2)),
    ];
    for (what, path, role) in examined("reopening(s) in another role", cases.to_vec()) {
        match Store::open(path, role) {
            Err(StoreError::RoleMismatch) => {}
            other => findings.push(format!("{what}: answered {other:?}")),
        }
    }
    // The same role reopens.
    if let Err(error) = Store::open(&one, device(1)) {
        findings.push(format!(
            "the device's store reopened in its own role: {error}"
        ));
    }
    assert_eq!(findings, Vec::<String>::new(), "the role check");
}

#[test]
fn migrations_run_from_an_empty_file_to_head_under_user_version() {
    let mut findings = Vec::new();
    // The embedded list is the migrations directory, file for file, versions 1 to head.
    let dir = root().join("crates/collection/migrations");
    let mut files: Vec<String> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .map(|entry| {
                    entry
                        .expect("an entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let embedded: Vec<String> = MIGRATIONS
        .iter()
        .map(|migration| format!("{:06}_{}.sql", migration.version, migration.description))
        .collect();
    if embedded != files {
        findings.push(format!(
            "embedded {embedded:?}, the directory holds {files:?}"
        ));
    }
    let versions: Vec<i64> = MIGRATIONS
        .iter()
        .map(|migration| migration.version)
        .collect();
    let expected: Vec<i64> = (1..=HEAD).collect();
    if versions != expected || HEAD < 1 {
        findings.push(format!("versions {versions:?} are not 1 to head {HEAD}"));
    }
    for (name, migration) in files.iter().zip(MIGRATIONS) {
        let text = fs::read_to_string(dir.join(name)).expect("a readable migration");
        if text != migration.sql {
            findings.push(format!("{name}: the embedded text differs from the file"));
        }
    }
    examined("migration file(s)", files.clone());

    let temp = tempfile::tempdir().expect("a temporary directory");
    let path = temp.path().join("fresh.store");
    for pass in ["made", "reopened"] {
        let store = Store::open(&path, SERVER).expect("the store opens");
        let settings = store.settings().expect("the settings read back");
        if settings.user_version != HEAD {
            findings.push(format!(
                "{pass}: user_version {} is not head {HEAD}",
                settings.user_version
            ));
        }
        let health = store.health().expect("the checks run");
        if health.integrity != ["ok"] || health.foreign_key_violations != 0 {
            findings.push(format!("{pass}: {health:?}"));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the migrations");
}

#[test]
fn a_store_one_schema_above_head_is_refused_and_never_downgraded() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("newer.store");
    drop(Store::open(&path, SERVER).expect("the store is made"));
    raw(&path)
        .execute_batch(&format!("PRAGMA user_version = {}", HEAD + 1))
        .expect("the fixture raises the schema");
    let before = snapshot(&path);
    let mut findings = Vec::new();
    match Store::open(&path, SERVER) {
        Err(StoreError::NewerSchema { found, newest }) if found == HEAD + 1 && newest == HEAD => {}
        other => findings.push(format!("the opener answered {other:?}")),
    }
    match Store::open_reader(&path) {
        Err(StoreError::NewerSchema { .. }) => {}
        other => findings.push(format!("the reader answered {other:?}")),
    }
    if snapshot(&path) != before {
        findings.push("the refused store changed".to_owned());
    }
    let version: i64 = raw(&path)
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("the version reads");
    if version != HEAD + 1 {
        findings.push(format!("the store was moved to schema {version}"));
    }
    assert_eq!(findings, Vec::<String>::new(), "a newer store");
}

#[test]
fn the_store_opener_refuses_a_file_of_another_application_and_leaves_it_untouched() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let files = [
        (
            "no id, one table",
            "CREATE TABLE notes (id INTEGER PRIMARY KEY);",
            0,
        ),
        ("another id, no table", "PRAGMA application_id = 42;", 42),
        (
            "another id, one table",
            "PRAGMA application_id = 42; CREATE TABLE col (id INTEGER PRIMARY KEY);",
            42,
        ),
    ];
    let mut findings = Vec::new();
    for (index, (what, setup, id)) in examined("foreign file(s)", files.to_vec())
        .into_iter()
        .enumerate()
    {
        let path = dir.path().join(format!("foreign-{index}.sqlite"));
        raw(&path)
            .execute_batch(setup)
            .expect("the fixture writes its file");
        let before = snapshot(&path);
        match Store::open(&path, SERVER) {
            Err(StoreError::ForeignFile(found)) if found == id => {}
            other => findings.push(format!("{what}: the opener answered {other:?}")),
        }
        match Store::open_reader(&path) {
            Err(StoreError::ForeignFile(found)) if found == id => {}
            other => findings.push(format!("{what}: the reader answered {other:?}")),
        }
        if snapshot(&path) != before {
            findings.push(format!("{what}: the refused file changed"));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "foreign files");
}

#[test]
fn a_path_whose_parent_is_a_file_is_refused_as_a_file_error() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let parent = dir.path().join("plain.txt");
    fs::write(&parent, b"not a directory").expect("the fixture writes its file");
    let path = parent.join("server.store");
    let mut findings = Vec::new();
    match Store::open(&path, SERVER) {
        Err(StoreError::File(_)) => {}
        other => findings.push(format!("the opener answered {other:?}")),
    }
    match Store::open_reader(&path) {
        Err(StoreError::File(_)) => {}
        other => findings.push(format!("the reader answered {other:?}")),
    }
    assert_eq!(findings, Vec::<String>::new(), "a path under a file");
}

#[test]
fn the_writer_holds_its_role_and_each_slot_answers_its_number() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut findings = Vec::new();
    for (what, role) in examined("role(s)", vec![("server", SERVER), ("device 3", device(3))]) {
        let path = dir.path().join(format!("{}.store", what.replace(' ', "-")));
        match Store::open(&path, role) {
            Ok(store) if store.role() == Some(role) => {}
            Ok(store) => findings.push(format!("{what}: the store holds {:?}", store.role())),
            Err(error) => findings.push(format!("{what}: the store was refused: {error}")),
        }
    }
    let slots = [
        ("the server's slot", Some(Slot::SERVER), 0),
        ("device slot 1", Slot::device(1), 1),
        ("device slot 5", Slot::device(5), 5),
        ("device slot 63", Slot::device(63), 63),
    ];
    for (what, slot, number) in examined("slot(s)", slots.to_vec()) {
        match slot {
            Some(slot) if slot.get() == number => {}
            other => findings.push(format!("{what}: answered {:?}", other.map(Slot::get))),
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "roles and slots");
}

#[test]
fn the_reader_refuses_a_store_one_schema_below_head() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("older.store");
    drop(Store::open(&path, SERVER).expect("the store is made"));
    raw(&path)
        .execute_batch(&format!("PRAGMA user_version = {}", HEAD - 1))
        .expect("the fixture lowers the schema");
    match Store::open_reader(&path) {
        Err(StoreError::NotAtHead { found, newest }) if found == HEAD - 1 && newest == HEAD => {}
        other => panic!("the reader answered {other:?}"),
    }
}

#[test]
fn a_foreign_file_opens_read_only_and_prepares_only_what_it_holds() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let foreign = dir.path().join("foreign.sqlite");
    raw(&foreign)
        .execute_batch("PRAGMA application_id = 42; CREATE TABLE col (id INTEGER PRIMARY KEY);")
        .expect("the fixture writes its file");
    let store = dir.path().join("server.store");
    drop(Store::open(&store, SERVER).expect("the store is made"));
    let mut findings = Vec::new();
    match ForeignFile::open_read_only(&foreign) {
        Ok(file) => {
            if let Err(error) = file.check_statement("SELECT id FROM col") {
                findings.push(format!("a statement over its table was refused: {error}"));
            }
            match file.check_statement("SELECT id FROM no_such_table") {
                Err(StoreError::Sqlite(_)) => {}
                other => findings.push(format!("a statement over no table answered {other:?}")),
            }
        }
        Err(error) => findings.push(format!("the foreign file was refused: {error}")),
    }
    match ForeignFile::open_read_only(&store) {
        Err(StoreError::StoreFile) => {}
        other => findings.push(format!("a collection store answered {other:?}")),
    }
    assert_eq!(findings, Vec::<String>::new(), "a foreign file");
}
