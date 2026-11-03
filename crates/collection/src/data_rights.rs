//! The store's data-rights port (SPEC-512 R10; ADR-530 D11): its export, the server's erase, a
//! device's erase and the client record's own erase. Outside the migrations, this is the one file
//! that names `erase_gate`: only an erase opens it, and the same transaction closes it.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use rusqlite::types::ValueRef;
use serde_json::{Map, Value};

use crate::error::StoreError;
use crate::store::{Store, Tx};

/// What an erase does to a declared table, as `privacy.json` writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Erase {
    /// Every row is deleted.
    Delete,
    /// The row stays, its personal value cleared.
    Anonymise,
}

impl Erase {
    /// The mode as `privacy.json` writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::Anonymise => "anonymise",
        }
    }
}

/// Every table the port exports, with what an erase does to it.
pub const DECLARED: &[(&str, Erase)] = &[
    ("store_meta", Erase::Anonymise),
    ("study_rule", Erase::Delete),
    ("notetypes", Erase::Delete),
    ("fields", Erase::Delete),
    ("templates", Erase::Delete),
    ("notes", Erase::Delete),
    ("note_tags", Erase::Delete),
    ("note_field_history", Erase::Delete),
    ("presets", Erase::Delete),
    ("decks", Erase::Delete),
    ("cards", Erase::Delete),
    ("review_log", Erase::Delete),
    ("review_retraction", Erase::Delete),
    ("review_client", Erase::Delete),
    ("outbox", Erase::Delete),
    ("applied_ops", Erase::Delete),
    ("undo_journal", Erase::Delete),
    ("schedule", Erase::Delete),
    ("device_setting", Erase::Delete),
];

/// The tables the port neither exports nor erases: the gate itself, which holds no personal data
/// and which an erase must find closed again.
pub const EXEMPT: &[&str] = &["erase_gate"];

/// The gate opened, for this transaction's deletes only.
pub const OPEN_GATE: &str = "UPDATE erase_gate SET open = 1 WHERE id = 1";
/// The gate closed again, before the erase commits.
pub const CLOSE_GATE: &str = "UPDATE erase_gate SET open = 0 WHERE id = 1";
/// The store's record kept, its import digest cleared: its role, writer id, slot and counters stay,
/// so no id is ever made twice.
pub const ANONYMISE_STORE_RECORD: &str = "UPDATE store_meta SET import_source = NULL WHERE id = 1";
/// The client record's own erase, and the first of the server's deletes.
pub const ERASE_CLIENT_RECORDS: &str = "DELETE FROM review_client";
/// The retractions.
pub const ERASE_RETRACTIONS: &str = "DELETE FROM review_retraction";
/// The review log.
pub const ERASE_REVIEWS: &str = "DELETE FROM review_log";
/// The notes' field history.
pub const ERASE_FIELD_HISTORY: &str = "DELETE FROM note_field_history";
/// The notes' tags.
pub const ERASE_NOTE_TAGS: &str = "DELETE FROM note_tags";
/// The cards.
pub const ERASE_CARDS: &str = "DELETE FROM cards";
/// The notes.
pub const ERASE_NOTES: &str = "DELETE FROM notes";
/// The templates.
pub const ERASE_TEMPLATES: &str = "DELETE FROM templates";
/// The fields.
pub const ERASE_FIELDS: &str = "DELETE FROM fields";
/// The notetypes.
pub const ERASE_NOTETYPES: &str = "DELETE FROM notetypes";
/// The decks.
pub const ERASE_DECKS: &str = "DELETE FROM decks";
/// The presets.
pub const ERASE_PRESETS: &str = "DELETE FROM presets";
/// The study rule.
pub const ERASE_STUDY_RULE: &str = "DELETE FROM study_rule";
/// The outbox.
pub const ERASE_OUTBOX: &str = "DELETE FROM outbox";
/// The applied operations.
pub const ERASE_APPLIED_OPS: &str = "DELETE FROM applied_ops";
/// The undo journal.
pub const ERASE_UNDO_JOURNAL: &str = "DELETE FROM undo_journal";
/// The schedule.
pub const ERASE_SCHEDULE: &str = "DELETE FROM schedule";
/// The device setting.
pub const ERASE_DEVICE_SETTING: &str = "DELETE FROM device_setting";
/// The file rebuilt, so no freed page keeps an erased value.
pub const VACUUM_FILE: &str = "VACUUM";
/// The log moved into the file and emptied.
pub const CHECKPOINT: &str = "PRAGMA wal_checkpoint(TRUNCATE)";

/// The server's deletes, children before the rows they name, so no foreign key breaks.
const ERASES: [&str; 18] = [
    ERASE_CLIENT_RECORDS,
    ERASE_RETRACTIONS,
    ERASE_REVIEWS,
    ERASE_FIELD_HISTORY,
    ERASE_NOTE_TAGS,
    ERASE_CARDS,
    ERASE_NOTES,
    ERASE_TEMPLATES,
    ERASE_FIELDS,
    ERASE_NOTETYPES,
    ERASE_DECKS,
    ERASE_PRESETS,
    ERASE_STUDY_RULE,
    ERASE_OUTBOX,
    ERASE_APPLIED_OPS,
    ERASE_UNDO_JOURNAL,
    ERASE_SCHEDULE,
    ERASE_DEVICE_SETTING,
];

/// The store's record.
pub const EXPORT_STORE_META: &str = "SELECT * FROM store_meta";
/// The study rule.
pub const EXPORT_STUDY_RULE: &str = "SELECT * FROM study_rule";
/// The notetypes.
pub const EXPORT_NOTETYPES: &str = "SELECT * FROM notetypes";
/// The fields.
pub const EXPORT_FIELDS: &str = "SELECT * FROM fields";
/// The templates.
pub const EXPORT_TEMPLATES: &str = "SELECT * FROM templates";
/// The notes.
pub const EXPORT_NOTES: &str = "SELECT * FROM notes";
/// The notes' tags.
pub const EXPORT_NOTE_TAGS: &str = "SELECT * FROM note_tags";
/// The notes' field history.
pub const EXPORT_NOTE_FIELD_HISTORY: &str = "SELECT * FROM note_field_history";
/// The presets.
pub const EXPORT_PRESETS: &str = "SELECT * FROM presets";
/// The decks.
pub const EXPORT_DECKS: &str = "SELECT * FROM decks";
/// The cards.
pub const EXPORT_CARDS: &str = "SELECT * FROM cards";
/// The review log.
pub const EXPORT_REVIEW_LOG: &str = "SELECT * FROM review_log";
/// The retractions.
pub const EXPORT_REVIEW_RETRACTION: &str = "SELECT * FROM review_retraction";
/// The client record.
pub const EXPORT_REVIEW_CLIENT: &str = "SELECT * FROM review_client";
/// The outbox.
pub const EXPORT_OUTBOX: &str = "SELECT * FROM outbox";
/// The applied operations.
pub const EXPORT_APPLIED_OPS: &str = "SELECT * FROM applied_ops";
/// The undo journal.
pub const EXPORT_UNDO_JOURNAL: &str = "SELECT * FROM undo_journal";
/// The schedule.
pub const EXPORT_SCHEDULE: &str = "SELECT * FROM schedule";
/// The device setting.
pub const EXPORT_DEVICE_SETTING: &str = "SELECT * FROM device_setting";

/// Each declared table's export statement, in [`DECLARED`]'s order.
const EXPORTS: [(&str, &str); 19] = [
    ("store_meta", EXPORT_STORE_META),
    ("study_rule", EXPORT_STUDY_RULE),
    ("notetypes", EXPORT_NOTETYPES),
    ("fields", EXPORT_FIELDS),
    ("templates", EXPORT_TEMPLATES),
    ("notes", EXPORT_NOTES),
    ("note_tags", EXPORT_NOTE_TAGS),
    ("note_field_history", EXPORT_NOTE_FIELD_HISTORY),
    ("presets", EXPORT_PRESETS),
    ("decks", EXPORT_DECKS),
    ("cards", EXPORT_CARDS),
    ("review_log", EXPORT_REVIEW_LOG),
    ("review_retraction", EXPORT_REVIEW_RETRACTION),
    ("review_client", EXPORT_REVIEW_CLIENT),
    ("outbox", EXPORT_OUTBOX),
    ("applied_ops", EXPORT_APPLIED_OPS),
    ("undo_journal", EXPORT_UNDO_JOURNAL),
    ("schedule", EXPORT_SCHEDULE),
    ("device_setting", EXPORT_DEVICE_SETTING),
];

/// One exported value: an integer or a real as a number, a text as itself, a blob as lowercase hex.
fn value(column: ValueRef<'_>) -> Value {
    match column {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => Value::from(number),
        ValueRef::Real(number) => Value::from(number),
        ValueRef::Text(text) => Value::String(String::from_utf8_lossy(text).into_owned()),
        ValueRef::Blob(bytes) => Value::String(bytes.iter().fold(
            String::with_capacity(bytes.len() * 2),
            |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            },
        )),
    }
}

/// Runs `statements` under the open gate in `tx`, closing it again, and returns the rows they deleted.
fn under_the_gate(tx: &Tx<'_>, statements: &[&str]) -> Result<usize, StoreError> {
    let conn = tx.conn();
    conn.execute(OPEN_GATE, [])?;
    let mut removed = 0;
    for sql in statements {
        removed += conn.execute(sql, [])?;
    }
    conn.execute(CLOSE_GATE, [])?;
    Ok(removed)
}

impl Store {
    /// Every declared table's rows, as JSON: `{"tables": {table: [row, ...]}}`, each row an object
    /// of its columns.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when a table cannot be read.
    pub fn export(&self) -> Result<Value, StoreError> {
        self.read(|snapshot| {
            let mut tables = Map::new();
            for (table, statement) in EXPORTS {
                let mut prepared = snapshot.conn().prepare_cached(statement)?;
                let columns: Vec<String> = prepared
                    .column_names()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                let mut rows = prepared.query([])?;
                let mut exported = Vec::new();
                while let Some(row) = rows.next()? {
                    let mut object = Map::new();
                    for (index, column) in columns.iter().enumerate() {
                        object.insert(column.clone(), value(row.get_ref(index)?));
                    }
                    exported.push(Value::Object(object));
                }
                tables.insert(table.to_owned(), Value::Array(exported));
            }
            let mut export = Map::new();
            export.insert("tables".to_owned(), Value::Object(tables));
            Ok(Value::Object(export))
        })
    }

    /// The server's erase: in one transaction the gate opens, every declared table but the store's
    /// record is emptied, the record's import digest is cleared and the gate closes; then the file
    /// is vacuumed and its log moved in, with `secure_delete` on, so no erased value stays in it.
    ///
    /// # Errors
    ///
    /// [`StoreError::ReadOnly`] for a reader, and [`StoreError::Sqlite`] when a statement fails;
    /// the transaction then rolls back whole, the gate closed.
    pub fn erase(&mut self) -> Result<(), StoreError> {
        self.write(|tx| {
            under_the_gate(tx, &ERASES)?;
            tx.conn().execute(ANONYMISE_STORE_RECORD, [])?;
            Ok(())
        })?;
        self.conn().execute_batch(VACUUM_FILE)?;
        self.conn().execute_batch(CHECKPOINT)?;
        Ok(())
    }

    /// The client record's own erase: its rows deleted under the gate, every review kept, so each
    /// review then reads unknown. Returns how many rows it deleted.
    ///
    /// # Errors
    ///
    /// [`StoreError::ReadOnly`] for a reader, and [`StoreError::Sqlite`] when a statement fails.
    pub fn erase_client_records(&mut self) -> Result<usize, StoreError> {
        self.write(|tx| under_the_gate(tx, &[ERASE_CLIENT_RECORDS]))
    }

    /// A device's erase: the connection closed, then the store's file and its WAL and
    /// shared-memory files removed.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when the connection cannot close, and [`StoreError::File`] when a
    /// file cannot be removed.
    pub fn erase_file(self) -> Result<(), StoreError> {
        let (conn, path) = self.into_parts();
        conn.close()
            .map_err(|(_, error)| StoreError::Sqlite(error))?;
        for file in [
            sidecar(&path, ""),
            sidecar(&path, "-wal"),
            sidecar(&path, "-shm"),
        ] {
            match std::fs::remove_file(&file) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(StoreError::File(error)),
            }
        }
        Ok(())
    }
}

/// `path` with `suffix` added to its file name.
fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
