//! The store's tables, each `STRICT` with its columns, and every check refusing its planted row
//! (SPEC-512 A3; ADR-530 D5).

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::collections::BTreeMap;

use deck_streak_collection::Store;
use support::{SERVER, base_store, examined, is_refusal, raw};

/// Every table of the store, with its columns in order.
const TABLES: [(&str, &[&str]); 20] = [
    (
        "store_meta",
        &[
            "id",
            "role",
            "origin",
            "slot",
            "id_floor",
            "next_seq",
            "import_source",
        ],
    ),
    ("study_rule", &["id", "rollover_hour", "utc_offset_minutes"]),
    ("erase_gate", &["id", "open"]),
    (
        "notetypes",
        &[
            "id",
            "name",
            "name_key",
            "kind",
            "css",
            "sort_field",
            "config",
            "mtime_ms",
        ],
    ),
    ("fields", &["notetype_id", "ord", "name", "config"]),
    (
        "templates",
        &["notetype_id", "ord", "name", "front", "back", "config"],
    ),
    (
        "notes",
        &[
            "id",
            "guid",
            "notetype_id",
            "fields",
            "sort_text",
            "first_digest",
            "mtime_ms",
        ],
    ),
    ("note_tags", &["note_id", "tag_key", "tag"]),
    (
        "note_field_history",
        &[
            "note_id",
            "ord",
            "version",
            "html",
            "origin",
            "seq",
            "at_ms",
            "lost_to_concurrent",
        ],
    ),
    ("presets", &["id", "name", "config", "mtime_ms"]),
    (
        "decks",
        &[
            "id",
            "name",
            "name_key",
            "preset_id",
            "kind",
            "config",
            "mtime_ms",
        ],
    ),
    (
        "cards",
        &[
            "id",
            "note_id",
            "ord",
            "deck_id",
            "new_position",
            "suspended",
            "bury_day",
            "flag",
            "created_ms",
        ],
    ),
    (
        "review_log",
        &[
            "id",
            "origin",
            "seq",
            "card_id",
            "note_id",
            "deck_id",
            "preset_id",
            "kind",
            "ease",
            "duration_ms",
            "study_day",
            "shown_due_day",
            "due_day",
            "interval_days",
            "scheduler",
            "basis_t",
            "basis_origin",
            "basis_seq",
            "server_seq",
        ],
    ),
    (
        "review_retraction",
        &["review_id", "origin", "seq", "at_ms"],
    ),
    ("review_client", &["review_id", "client"]),
    (
        "outbox",
        &["seq", "kind", "payload", "made_ms", "server_seq", "outcome"],
    ),
    ("applied_ops", &["origin", "seq", "server_seq", "outcome"]),
    (
        "undo_journal",
        &[
            "step",
            "kind",
            "card_id",
            "review_id",
            "before",
            "after",
            "op_seq",
            "undone",
        ],
    ),
    (
        "schedule",
        &[
            "card_id",
            "scheduler",
            "basis_review",
            "due_day",
            "due_ms",
            "order_key",
        ],
    ),
    ("device_setting", &["key", "value"]),
];

/// One planted row per check: each must be refused by a constraint or a `STRICT` column, against a
/// store holding the base rows.
const PLANTED: [(&str, &str); 50] = [
    (
        "store_meta.id",
        "INSERT INTO store_meta VALUES (2, 0, zeroblob(16), 0, 0, 0, NULL)",
    ),
    ("store_meta.role", "UPDATE store_meta SET role = 2"),
    (
        "store_meta.origin",
        "UPDATE store_meta SET origin = zeroblob(15)",
    ),
    ("store_meta.slot", "UPDATE store_meta SET slot = 64"),
    ("store_meta.next_seq", "UPDATE store_meta SET next_seq = -1"),
    ("study_rule.id", "INSERT INTO study_rule VALUES (2, 4, 0)"),
    (
        "study_rule.rollover_hour",
        "UPDATE study_rule SET rollover_hour = 24",
    ),
    (
        "study_rule.utc_offset_minutes high",
        "UPDATE study_rule SET utc_offset_minutes = 841",
    ),
    (
        "study_rule.utc_offset_minutes low",
        "UPDATE study_rule SET utc_offset_minutes = -721",
    ),
    ("erase_gate.id", "INSERT INTO erase_gate VALUES (2, 0)"),
    ("erase_gate.open", "UPDATE erase_gate SET open = 2"),
    (
        "notetypes.kind",
        "INSERT INTO notetypes VALUES (2, 'b', 'b', 2, '', 0, '{}', 0)",
    ),
    (
        "notetypes.sort_field",
        "INSERT INTO notetypes VALUES (2, 'b', 'b', 0, '', -1, '{}', 0)",
    ),
    (
        "notetypes.config",
        "INSERT INTO notetypes VALUES (2, 'b', 'b', 0, '', 0, 'nope', 0)",
    ),
    (
        "notetypes.name_key",
        "INSERT INTO notetypes VALUES (2, 'B', 'basic', 0, '', 0, '{}', 0)",
    ),
    (
        "fields.ord",
        "INSERT INTO fields VALUES (1, -1, 'Back', '{}')",
    ),
    (
        "fields.config",
        "INSERT INTO fields VALUES (1, 1, 'Back', 'nope')",
    ),
    (
        "fields.name",
        "INSERT INTO fields VALUES (1, 1, 'Front', '{}')",
    ),
    (
        "templates.ord",
        "INSERT INTO templates VALUES (1, -1, 'c', '', '', '{}')",
    ),
    (
        "templates.config",
        "INSERT INTO templates VALUES (1, 1, 'c', '', '', 'nope')",
    ),
    (
        "notes.fields object",
        "INSERT INTO notes VALUES (2, 'g2', 1, '{}', '', zeroblob(32), 0)",
    ),
    (
        "notes.fields text",
        "INSERT INTO notes VALUES (2, 'g2', 1, 'nope', '', zeroblob(32), 0)",
    ),
    (
        "notes.first_digest",
        "INSERT INTO notes VALUES (2, 'g2', 1, '[]', '', zeroblob(31), 0)",
    ),
    (
        "notes.guid",
        "INSERT INTO notes VALUES (2, 'guid-one', 1, '[]', '', zeroblob(32), 0)",
    ),
    (
        "notes.notetype_id",
        "INSERT INTO notes VALUES (2, 'g2', 9, '[]', '', zeroblob(32), 0)",
    ),
    (
        "note_field_history.version",
        "INSERT INTO note_field_history VALUES (1, 0, 0, '', zeroblob(16), 1, 0, 0)",
    ),
    (
        "note_field_history.origin",
        "INSERT INTO note_field_history VALUES (1, 0, 2, '', zeroblob(15), 1, 0, 0)",
    ),
    (
        "note_field_history.lost_to_concurrent",
        "INSERT INTO note_field_history VALUES (1, 0, 2, '', zeroblob(16), 1, 0, 2)",
    ),
    (
        "presets.config",
        "INSERT INTO presets VALUES (2, 'p', 'nope', 0)",
    ),
    (
        "presets.mtime_ms strict",
        "INSERT INTO presets VALUES (2, 'p', '{}', 'late')",
    ),
    (
        "decks.name",
        "INSERT INTO decks VALUES (2, '', 'two', 1, 0, '{}', 0)",
    ),
    (
        "decks.kind",
        "INSERT INTO decks VALUES (2, 'Two', 'two', 1, 2, '{}', 0)",
    ),
    (
        "decks.name_key",
        "INSERT INTO decks VALUES (2, 'DEFAULT', 'default', 1, 0, '{}', 0)",
    ),
    (
        "cards.ord",
        "INSERT INTO cards VALUES (2, 1, -1, 1, NULL, 0, NULL, 0, 0)",
    ),
    (
        "cards.new_position",
        "INSERT INTO cards VALUES (2, 1, 1, 1, -1, 0, NULL, 0, 0)",
    ),
    (
        "cards.suspended",
        "INSERT INTO cards VALUES (2, 1, 1, 1, NULL, 2, NULL, 0, 0)",
    ),
    (
        "cards.flag",
        "INSERT INTO cards VALUES (2, 1, 1, 1, NULL, 0, NULL, 8, 0)",
    ),
    (
        "cards.note and ordinal",
        "INSERT INTO cards VALUES (2, 1, 0, 1, NULL, 0, NULL, 0, 0)",
    ),
    (
        "review_log.origin",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(15), 9, 1, 1, 1, 1, 1, 3, 0, 0, 9)",
    ),
    (
        "review_log.seq",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(16), -1, 1, 1, 1, 1, 1, 3, 0, 0, 9)",
    ),
    (
        "review_log.kind",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 6, 3, 0, 0, 9)",
    ),
    (
        "review_log.ease",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 5, 0, 0, 9)",
    ),
    (
        "review_log.duration_ms",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, -1, 0, 9)",
    ),
    (
        "review_log.interval_days",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, interval_days, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, 0, 0, -1, 9)",
    ),
    (
        "review_log.scheduler",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, scheduler, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, 0, 0, zeroblob(31), 9)",
    ),
    (
        "review_log.basis all or none",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, basis_t, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, 0, 0, 64, 9)",
    ),
    (
        "review_log.basis_origin",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, basis_t, basis_origin, basis_seq, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, 0, 0, 64, zeroblob(15), 1, 9)",
    ),
    (
        "review_log.origin and seq",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(16), 1, 1, 1, 1, 1, 1, 3, 0, 0, 9)",
    ),
    (
        "review_log.server_seq",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, 0, 0, 1)",
    ),
    (
        "review_client.client four",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, 0, 0, 9); INSERT INTO review_client VALUES (128, 4)",
    ),
];

/// The planted rows of the tables the first list leaves out, so every check is planted once.
const PLANTED_MORE: [(&str, &str); 12] = [
    (
        "review_client.client zero",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, 0, 0, 9); INSERT INTO review_client VALUES (128, 0)",
    ),
    (
        "review_retraction.origin",
        "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease, duration_ms, study_day, server_seq) VALUES (128, zeroblob(16), 9, 1, 1, 1, 1, 1, 3, 0, 0, 9); INSERT INTO review_retraction VALUES (128, zeroblob(15), 3, 0)",
    ),
    (
        "review_retraction.review_id",
        "INSERT INTO review_retraction VALUES (999, zeroblob(16), 3, 0)",
    ),
    (
        "outbox.payload",
        "INSERT INTO outbox VALUES (2, 1, 'nope', 0, NULL, NULL)",
    ),
    (
        "outbox.outcome",
        "INSERT INTO outbox VALUES (2, 1, '{}', 0, NULL, 2)",
    ),
    (
        "applied_ops.outcome",
        "INSERT INTO applied_ops VALUES (zeroblob(16), 2, NULL, 2)",
    ),
    (
        "undo_journal.before",
        "INSERT INTO undo_journal VALUES (2, 1, 1, 64, 'nope', '{}', 1, 0)",
    ),
    (
        "undo_journal.undone",
        "INSERT INTO undo_journal VALUES (2, 1, 1, 64, '{}', '{}', 1, 2)",
    ),
    (
        "schedule.scheduler",
        "INSERT INTO schedule VALUES (2, zeroblob(31), NULL, 0, NULL, NULL)",
    ),
    (
        "schedule.no due",
        "INSERT INTO schedule VALUES (2, zeroblob(32), NULL, NULL, NULL, NULL)",
    ),
    (
        "schedule.two dues",
        "INSERT INTO schedule VALUES (2, zeroblob(32), NULL, 0, 0, NULL)",
    ),
    (
        "device_setting.value",
        "INSERT INTO device_setting VALUES ('x', NULL)",
    ),
];

#[test]
fn every_table_of_the_schema_is_strict_with_its_columns() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("schema.store");
    let mut findings = Vec::new();
    if let Err(error) = Store::open(&path, SERVER) {
        findings.push(format!("the opener refused: {error}"));
    }
    let conn = raw(&path);
    let strict: BTreeMap<String, bool> = conn
        .prepare("SELECT name, strict FROM pragma_table_list WHERE schema = 'main' AND name NOT LIKE 'sqlite_%'")
        .expect("the table list prepares")
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? == 1)))
        .expect("the table list reads")
        .collect::<Result<_, _>>()
        .expect("every table row reads");
    for (table, columns) in examined("table(s)", TABLES.to_vec()) {
        match strict.get(table) {
            None => findings.push(format!("{table} is missing")),
            Some(false) => findings.push(format!("{table} is not STRICT")),
            Some(true) => {}
        }
        let found: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
            .expect("the column list prepares")
            .query_map([table], |row| row.get(0))
            .expect("the column list reads")
            .collect::<Result<_, _>>()
            .expect("every column reads");
        if found != columns {
            findings.push(format!("{table} holds {found:?}, not {columns:?}"));
        }
    }
    let named: Vec<&str> = TABLES.iter().map(|(table, _)| *table).collect();
    for table in strict.keys() {
        if !named.contains(&table.as_str()) {
            findings.push(format!("{table} is a table the schema does not name"));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the tables");
}

#[test]
fn each_check_refuses_its_planted_row() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("checks.store");
    let mut findings = Vec::new();
    if let Err(why) = base_store(&path, SERVER) {
        findings.push(why);
    }
    let mut conn = raw(&path);
    conn.execute_batch("PRAGMA foreign_keys = ON")
        .expect("the fixture enforces foreign keys");
    let planted: Vec<(&str, &str)> = PLANTED.iter().chain(PLANTED_MORE.iter()).copied().collect();
    for (check, statement) in examined("planted row(s)", planted) {
        let tx = conn.transaction().expect("a fixture transaction");
        match tx.execute_batch(statement) {
            Err(error) if is_refusal(&error) => {}
            Err(error) => findings.push(format!("{check}: refused for another reason: {error}")),
            Ok(()) => findings.push(format!("{check}: admitted")),
        }
        tx.rollback().expect("the planted row is rolled back");
    }
    assert_eq!(findings, Vec::<String>::new(), "the checks");
}
