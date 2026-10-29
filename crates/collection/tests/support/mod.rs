//! Shared fixtures of the collection store's tests (SPEC-512): a temporary store, a fixture's own
//! connection to write its rows, the valid base rows every table holds once, and the workspace's
//! Rust sources for the censuses. A fixture writes its rows over its own connection: the store's
//! write ports come with the changes that first write each table.

// Test code: helpers panic on an unreadable tree, and print the examined counts on purpose.
#![allow(clippy::expect_used, clippy::print_stdout, dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_collection::{Origin, Role, Slot, Store};
use rusqlite::Connection;

/// The server's writer id in these tests.
pub const SERVER_ORIGIN: Origin = Origin::new([7; 16]);

/// The server's role in these tests.
pub const SERVER: Role = Role::Server {
    origin: SERVER_ORIGIN,
};

/// A device's role in slot `slot`, with a writer id of that byte.
#[must_use]
pub fn device(slot: u8) -> Role {
    Role::Device {
        slot: Slot::device(slot).expect("a device slot"),
        origin: Origin::new([slot; 16]),
    }
}

/// The workspace's root.
#[must_use]
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

/// Prints how many `what` a census examined, and refuses zero.
pub fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// A fixture's own connection to the file at `path`.
#[must_use]
pub fn raw(path: &Path) -> Connection {
    Connection::open(path).expect("the fixture opens its connection")
}

/// One valid row in every table but the store's record and the erase gate, which the opener
/// writes. The review row carries its server order, so the server's store admits it too.
pub const BASE_ROWS: &str = "
INSERT INTO study_rule (id, rollover_hour, utc_offset_minutes) VALUES (1, 4, 0);
INSERT INTO notetypes (id, name, name_key, kind, css, sort_field, config, mtime_ms)
  VALUES (1, 'Basic', 'basic', 0, '.card {}', 0, '{}', 0);
INSERT INTO fields (notetype_id, ord, name, config) VALUES (1, 0, 'Front', '{}');
INSERT INTO templates (notetype_id, ord, name, front, back, config)
  VALUES (1, 0, 'Card 1', '{{Front}}', '{{FrontSide}}', '{}');
INSERT INTO presets (id, name, config, mtime_ms) VALUES (1, 'Default', '{}', 0);
INSERT INTO decks (id, name, name_key, preset_id, kind, config, mtime_ms)
  VALUES (1, 'Default', 'default', 1, 0, '{}', 0);
INSERT INTO notes (id, guid, notetype_id, fields, sort_text, first_digest, mtime_ms)
  VALUES (1, 'guid-one', 1, '[\"MARKERFRONTFIELD\",\"back\"]', 'MARKERFRONTFIELD', zeroblob(32), 0);
INSERT INTO note_tags (note_id, tag_key, tag) VALUES (1, 'ds', 'ds');
INSERT INTO note_field_history (note_id, ord, version, html, origin, seq, at_ms, lost_to_concurrent)
  VALUES (1, 0, 1, 'MARKERFRONTFIELD', zeroblob(16), 1, 0, 0);
INSERT INTO cards (id, note_id, ord, deck_id, new_position, suspended, bury_day, flag, created_ms)
  VALUES (1, 1, 0, 1, NULL, 0, NULL, 0, 0);
INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease,
  duration_ms, study_day, server_seq)
  VALUES (64, zeroblob(16), 1, 1, 1, 1, 1, 1, 3, 4000, 0, 1);
INSERT INTO review_retraction (review_id, origin, seq, at_ms) VALUES (64, zeroblob(16), 2, 0);
INSERT INTO review_client (review_id, client) VALUES (64, 1);
INSERT INTO outbox (seq, kind, payload, made_ms, server_seq, outcome) VALUES (1, 1, '{}', 0, NULL, NULL);
INSERT INTO applied_ops (origin, seq, server_seq, outcome) VALUES (zeroblob(16), 1, NULL, 0);
INSERT INTO undo_journal (step, kind, card_id, review_id, before, after, op_seq, undone)
  VALUES (1, 1, 1, 64, '{}', '{}', 1, 0);
INSERT INTO schedule (card_id, scheduler, basis_review, due_day, due_ms, order_key)
  VALUES (1, zeroblob(32), NULL, 0, NULL, NULL);
INSERT INTO device_setting (key, value) VALUES ('deck', '1');
";

/// A store at `path` in `role`, made by the opener and then given [`BASE_ROWS`] over a fixture's
/// own connection. Returns why the base rows were refused, if they were.
pub fn base_store(path: &Path, role: Role) -> Result<(), String> {
    drop(Store::open(path, role).map_err(|error| format!("the opener refused: {error}"))?);
    raw(path)
        .execute_batch(BASE_ROWS)
        .map_err(|error| format!("the base rows were refused: {error}"))
}

/// Every Rust file under `dir`, recursively, as `(path relative to root, text)`.
pub fn rust_files(root: &Path, dir: &Path, into: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(root, &path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let relative = path
                .strip_prefix(root)
                .expect("a path under the root")
                .to_string_lossy()
                .replace('\\', "/");
            into.push((
                relative,
                fs::read_to_string(&path).expect("a readable source"),
            ));
        }
    }
}

/// Every Rust source of every crate's `src` under `root`, as `(relative path, text)`.
#[must_use]
pub fn workspace_sources(root: &Path) -> Vec<(String, String)> {
    let mut sources = Vec::new();
    let mut crates: Vec<PathBuf> = fs::read_dir(root.join("crates"))
        .map(|entries| {
            entries
                .map(|entry| entry.expect("a crate entry").path())
                .collect()
        })
        .unwrap_or_default();
    crates.sort();
    for krate in crates {
        rust_files(root, &krate.join("src"), &mut sources);
    }
    sources
}

/// Writes `text` at `relative` under `root`, making its directories: a planted tree's file.
pub fn plant(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("a parent")).expect("the planted directory is made");
    fs::write(path, text).expect("the planted file is written");
}

/// Whether a statement's refusal is a constraint's or a `STRICT` column's.
#[must_use]
pub fn is_refusal(error: &rusqlite::Error) -> bool {
    let text = error.to_string();
    text.contains("constraint failed") || text.contains("cannot store")
}
