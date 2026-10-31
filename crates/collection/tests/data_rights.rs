//! The store's data-rights port and its privacy inventory (SPEC-512 A10; ADR-530 D11): the export
//! writes every declared table as JSON, the server's erase empties every table through the gate and
//! closes it, the client record's erase keeps every review, a device's erase removes the file, and
//! `privacy.json` declares every store table once with the port's erase mode.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use deck_streak_collection::Store;
use deck_streak_collection::data_rights::{DECLARED, EXEMPT, Erase};
use serde_json::Value;
use support::{SERVER, base_store, device, examined, raw, root};

/// The glob `privacy.json` reads the store's migrations through.
const STORE_GLOB: &str = "crates/collection/migrations/*.sql";

/// How many rows `table` holds, by a fixture's own connection.
fn rows(path: &Path, table: &str) -> i64 {
    raw(path)
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap_or(-1)
}

/// Every table the store's migrations create.
fn store_tables() -> BTreeSet<String> {
    let dir = root().join("crates/collection/migrations");
    let mut tables = BTreeSet::new();
    for entry in fs::read_dir(dir).into_iter().flatten() {
        let text = fs::read_to_string(entry.expect("an entry").path()).expect("a migration");
        for line in text.lines() {
            if let Some(rest) = line.trim_start().strip_prefix("CREATE TABLE ") {
                let name: String = rest
                    .chars()
                    .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                    .collect();
                tables.insert(name);
            }
        }
    }
    tables
}

#[test]
fn the_export_writes_every_declared_table_as_json() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("export.store");
    let mut findings = Vec::new();
    if let Err(why) = base_store(&path, SERVER) {
        findings.push(why);
    }
    let store = Store::open(&path, SERVER).expect("the store reopens");
    let export = store.export().expect("the export runs");
    for (table, _) in examined("declared table(s)", DECLARED.to_vec()) {
        let exported = export["tables"][table].as_array().map(Vec::len);
        let held = usize::try_from(rows(&path, table)).ok();
        if exported != held {
            findings.push(format!(
                "{table}: exported {exported:?} rows, holds {held:?}"
            ));
        }
    }
    // A blob is written as lowercase hex, and a text as itself.
    let origin = &export["tables"]["store_meta"][0]["origin"];
    if origin != &Value::String("07".repeat(16)) {
        findings.push(format!("the writer id was exported as {origin}"));
    }
    let fields = &export["tables"]["notes"][0]["fields"];
    if fields != &Value::String("[\"MARKERFRONTFIELD\",\"back\"]".to_owned()) {
        findings.push(format!("the note's fields were exported as {fields}"));
    }
    for exempt in EXEMPT {
        if !export["tables"][*exempt].is_null() {
            findings.push(format!("{exempt} is exempt, and was exported"));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the export");
}

#[test]
fn the_server_erase_empties_every_table_through_the_gate_and_closes_it() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("erase.store");
    let mut findings = Vec::new();
    if let Err(why) = base_store(&path, SERVER) {
        findings.push(why);
    }
    if let Err(error) =
        raw(&path).execute_batch("UPDATE store_meta SET import_source = 'anki-schema-18:digest'")
    {
        findings.push(format!("the fixture could not record an import: {error}"));
    }
    let mut store = Store::open(&path, SERVER).expect("the store reopens");
    if let Err(error) = store.erase() {
        findings.push(format!("the erase failed: {error}"));
    }
    drop(store);
    for (table, erase) in examined("declared table(s)", DECLARED.to_vec()) {
        let left = rows(&path, table);
        match erase {
            Erase::Delete if left != 0 => findings.push(format!("{table}: {left} row(s) left")),
            Erase::Anonymise if left != 1 => {
                findings.push(format!("{table}: {left} row(s), not the one record"));
            }
            _ => {}
        }
    }
    let conn = raw(&path);
    let record: (i64, Option<String>) = conn
        .query_row("SELECT role, import_source FROM store_meta", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap_or((-1, Some("unread".to_owned())));
    if record != (0, None) {
        findings.push(format!("the store's record reads {record:?}"));
    }
    let gate: i64 = conn
        .query_row("SELECT open FROM erase_gate WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap_or(-1);
    if gate != 0 {
        findings.push(format!("the gate reads {gate} after the erase"));
    }
    drop(conn);
    // secure_delete and VACUUM: the erased text is nowhere in the file's bytes.
    let bytes = fs::read(&path).expect("the file reads");
    if bytes
        .windows(b"MARKERFRONTFIELD".len())
        .any(|window| window == b"MARKERFRONTFIELD")
    {
        findings.push("an erased note's text is still in the file".to_owned());
    }
    assert_eq!(findings, Vec::<String>::new(), "the server's erase");
}

#[test]
fn the_client_record_erase_deletes_only_the_client_rows_and_keeps_every_review() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("client.store");
    let mut findings = Vec::new();
    if let Err(why) = base_store(&path, SERVER) {
        findings.push(why);
    }
    let before: BTreeMap<&str, i64> = DECLARED
        .iter()
        .map(|(table, _)| (*table, rows(&path, table)))
        .collect();
    let mut store = Store::open(&path, SERVER).expect("the store reopens");
    match store.erase_client_records() {
        Ok(1) => {}
        other => findings.push(format!("the client record's erase answered {other:?}")),
    }
    drop(store);
    for (table, held) in examined("declared table(s)", before.into_iter().collect()) {
        let now = rows(&path, table);
        let expected = if table == "review_client" { 0 } else { held };
        if now != expected {
            findings.push(format!("{table}: {now} row(s), expected {expected}"));
        }
    }
    let gate: i64 = raw(&path)
        .query_row("SELECT open FROM erase_gate WHERE id = 1", [], |row| {
            row.get(0)
        })
        .unwrap_or(-1);
    if gate != 0 {
        findings.push(format!("the gate reads {gate} after the erase"));
    }
    assert_eq!(findings, Vec::<String>::new(), "the client record's erase");
}

#[test]
fn a_device_erase_removes_the_store_file() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("device.store");
    let mut findings = Vec::new();
    if let Err(why) = base_store(&path, device(3)) {
        findings.push(why);
    }
    let store = Store::open(&path, device(3)).expect("the store reopens");
    let sidecars = ["", "-wal", "-shm"].map(|suffix| {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        std::path::PathBuf::from(name)
    });
    if !path.exists() {
        findings.push("the store's file is missing before the erase".to_owned());
    }
    if let Err(error) = store.erase_file() {
        findings.push(format!("the erase failed: {error}"));
    }
    for file in examined("store file(s)", sidecars.to_vec()) {
        if file.exists() {
            findings.push(format!("{} is still there", file.display()));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "a device's erase");
}

/// Every refusal of `inventory` against the port: a store table neither declared nor exempt, a
/// declared table no category names or two do, a category whose erase mode is not the port's or
/// that does not export, and a missing glob.
fn judge(inventory: &Value) -> Vec<String> {
    let mut refused = Vec::new();
    let globs: Vec<&str> = inventory["sql"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if !globs.contains(&STORE_GLOB) {
        refused.push(format!("privacy.json does not read {STORE_GLOB}"));
    }
    let declared: BTreeMap<&str, Erase> = DECLARED.iter().copied().collect();
    for table in store_tables() {
        if !declared.contains_key(table.as_str()) && !EXEMPT.contains(&table.as_str()) {
            refused.push(format!("{table} is neither declared nor exempt"));
        }
    }
    let mut named: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for category in inventory["categories"].as_array().into_iter().flatten() {
        let tables: BTreeSet<&str> = category["stores"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|store| store.split_once('.').map(|(table, _)| table))
            .collect();
        for table in tables {
            named.entry(table.to_owned()).or_default().push(category);
        }
    }
    for (table, erase) in &declared {
        match named.get(*table).map(Vec::as_slice) {
            None => refused.push(format!("{table} is named by no category")),
            Some([category]) => {
                let id = category["id"].as_str().unwrap_or("?");
                if category["erase"] != erase.as_str() {
                    refused.push(format!(
                        "{table}'s category {id} erases by {}",
                        category["erase"]
                    ));
                }
                if category["export"] != Value::Bool(true) {
                    refused.push(format!("{table}'s category {id} does not export it"));
                }
            }
            Some(many) => refused.push(format!("{table} is named by {} categories", many.len())),
        }
    }
    refused
}

#[test]
fn privacy_json_declares_every_store_table_once_with_the_ports_erase_mode() {
    let text = fs::read_to_string(root().join("privacy.json")).expect("privacy.json is readable");
    let inventory: Value = serde_json::from_str(&text).expect("privacy.json is JSON");
    examined("store table(s)", store_tables().into_iter().collect());
    examined("declared table(s)", DECLARED.to_vec());
    assert_eq!(judge(&inventory), Vec::<String>::new(), "the inventory");

    // A planted inventory that forgets the client record's category is refused by its table.
    let mut planted = inventory.clone();
    if let Some(categories) = planted["categories"].as_array_mut() {
        categories.retain(|category| category["id"] != "collection-client-record");
    }
    assert_eq!(
        judge(&planted),
        ["review_client is named by no category"],
        "the planted inventory"
    );
}
