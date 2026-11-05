//! Fixtures the format crate's tests share. The engine makes every collection and media database;
//! a case's rows are written through the engine's own connection, in the form of ingest's
//! `collection_read.rs`.

#![allow(
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::too_many_lines
)]

use std::fs;
use std::path::{Path, PathBuf};

use anki::collection::CollectionBuilder;
use deck_streak_anki_format::opener::digest;
use deck_streak_anki_format::{Kind, Limits, Reader, Refusal, open};
use rusqlite::{Connection, OpenFlags, params};
use serde_json::Value;

/// The workspace root.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

/// The population a test judges, printed by its count and refused when empty.
pub fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The cases of the committed golden of `function`.
pub fn golden_cases(function: &str) -> Vec<Value> {
    let path = root().join(format!("tools/parity-oracle/goldens/{function}.json"));
    let text = fs::read_to_string(path).expect("the golden reads");
    let golden: Value = serde_json::from_str(&text).expect("the golden is JSON");
    golden["cases"].as_array().cloned().unwrap_or_default()
}

/// A fresh collection the engine made, at `dir/collection.anki2`.
pub fn engine_collection(dir: &Path) -> PathBuf {
    let path = dir.join("collection.anki2");
    let col = CollectionBuilder::new(&path)
        .build()
        .expect("the engine creates the collection");
    col.close(None).expect("the engine closes the collection");
    path
}

/// Runs `write` on the engine's own connection to the collection at `path`.
pub fn with_engine(path: &Path, write: impl FnOnce(&Connection)) {
    let col = CollectionBuilder::new(path)
        .build()
        .expect("the engine opens the collection");
    write(col.storage.db());
    col.close(None).expect("the engine closes the collection");
}

/// A media database the engine made, at `dir/collection.media.db2`, holding `names`.
pub fn engine_media(dir: &Path, names: &[&str]) -> PathBuf {
    let folder = dir.join("collection.media");
    fs::create_dir_all(&folder).expect("the media folder");
    let path = dir.join("collection.media.db2");
    let manager =
        anki::media::MediaManager::new(&folder, &path).expect("the engine creates the media db");
    for name in names {
        manager
            .add_file(name, name.as_bytes())
            .expect("the engine adds a media file");
    }
    drop(manager);
    path
}

/// The digest a caller records for the file at `path`.
pub fn recorded(path: &Path) -> [u8; 32] {
    digest(&fs::read(path).expect("the fixture reads"))
}

/// The file at `path`, opened through the crate with its own digest and the default bounds.
pub fn open_file(path: &Path, kind: Kind) -> Result<Reader, Refusal> {
    open(path, &recorded(path), kind, &Limits::default())
}

/// The test's own connection: rusqlite alone, no collation registered.
pub fn plain(path: &Path, flags: OpenFlags) -> Connection {
    Connection::open_with_flags(path, flags).expect("the test's own connection opens")
}

/// The text of a JSON value, a number as its digits.
pub fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The integer of a JSON value.
pub fn int(value: &Value) -> i64 {
    value.as_i64().expect("an integer")
}

/// The items of a JSON array.
pub fn list(value: &Value) -> Vec<Value> {
    value.as_array().cloned().unwrap_or_default()
}

/// `base` plus a corpus index, or `none` for a null.
pub fn id(base: i64, index: &Value, none: i64) -> i64 {
    index.as_i64().map_or(none, |index| base + index)
}

/// The ids a case's corpus indices take in its collection.
pub const DECK: i64 = 1000;
pub const NOTETYPE: i64 = 100;
pub const NOTE: i64 = 10_000;
pub const CARD: i64 = 20_000;

fn varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = u8::try_from(value & 0x7f).expect("seven bits");
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// A varint field, omitted at zero.
pub fn put_varint(out: &mut Vec<u8>, number: u64, value: u64) {
    if value != 0 {
        varint(out, number << 3);
        varint(out, value);
    }
}

/// A length-delimited text field, omitted when empty.
pub fn put_text(out: &mut Vec<u8>, number: u64, text: &str) {
    if !text.is_empty() {
        varint(out, (number << 3) | 2);
        varint(out, u64::try_from(text.len()).expect("a length"));
        out.extend_from_slice(text.as_bytes());
    }
}

/// The schema-18 collection of one `anki.read.collection_view` case: the engine makes the file,
/// its stock rows but the default deck are cleared, and the case's per-table view is written
/// back as rows, each corpus index at its own id.
pub fn case_collection(dir: &Path, case: &Value) -> PathBuf {
    let path = engine_collection(dir);
    let input = &case["input"]["collection"];
    let output = &case["output"];
    with_engine(&path, |db| {
        db.execute_batch(
            "DELETE FROM cards; DELETE FROM notes; DELETE FROM revlog; DELETE FROM notetypes; \
             DELETE FROM fields; DELETE FROM templates; DELETE FROM graves; \
             DELETE FROM decks WHERE id != 1;",
        )
        .expect("the engine's stock rows are cleared");
        db.execute("UPDATE col SET crt = ?1", [int(&input["crt"])])
            .expect("the creation stamp");
        for deck in list(&output["decks"]) {
            if deck["index"].is_null() {
                continue;
            }
            let kind: &[u8] = if deck["filtered"] == Value::Bool(true) {
                &[0x12, 0x00]
            } else {
                &[0x0a, 0x02, 0x08, 0x01]
            };
            db.execute(
                "INSERT INTO decks VALUES (?1, ?2, 0, 0, x'', ?3)",
                params![
                    id(DECK, &deck["index"], 1),
                    text(&deck["name"]).replace("::", "\u{1f}"),
                    kind
                ],
            )
            .expect("a deck");
        }
        for notetype in list(&output["notetypes"]) {
            let ntid = id(NOTETYPE, &notetype["index"], 1);
            let mut config = Vec::new();
            put_varint(&mut config, 1, notetype["kind"].as_u64().expect("a kind"));
            put_varint(
                &mut config,
                2,
                notetype["sort_field"].as_u64().expect("a sort field"),
            );
            put_text(&mut config, 3, &text(&notetype["css"]));
            db.execute(
                "INSERT INTO notetypes VALUES (?1, ?2, 0, 0, ?3)",
                params![ntid, text(&notetype["name"]), config],
            )
            .expect("a notetype");
            for (ord, name) in list(&notetype["fields"]).iter().enumerate() {
                db.execute(
                    "INSERT INTO fields VALUES (?1, ?2, ?3, x'')",
                    params![ntid, i64::try_from(ord).expect("an ordinal"), text(name)],
                )
                .expect("a field");
            }
            for (ord, template) in list(&notetype["templates"]).iter().enumerate() {
                let mut config = Vec::new();
                put_text(&mut config, 1, &text(&template["front"]));
                put_text(&mut config, 2, &text(&template["back"]));
                db.execute(
                    "INSERT INTO templates VALUES (?1, ?2, ?3, 0, 0, ?4)",
                    params![
                        ntid,
                        i64::try_from(ord).expect("an ordinal"),
                        text(&template["name"]),
                        config
                    ],
                )
                .expect("a template");
            }
        }
        for note in list(&output["notes"]) {
            let fields: Vec<String> = list(&note["fields"]).iter().map(text).collect();
            let tags: Vec<String> = list(&note["tags"]).iter().map(text).collect();
            let tags = if tags.is_empty() {
                String::new()
            } else {
                format!(" {} ", tags.join(" "))
            };
            db.execute(
                "INSERT INTO notes VALUES (?1, ?2, ?3, 0, 0, ?4, ?5, ?6, ?7, 0, '')",
                params![
                    id(NOTE, &note["index"], 0),
                    text(&note["guid"]),
                    id(NOTETYPE, &note["notetype"], 1),
                    tags,
                    fields.join("\u{1f}"),
                    text(&note["sfld"]),
                    int(&note["csum"])
                ],
            )
            .expect("a note");
        }
        for card in list(&output["cards"]) {
            db.execute(
                "INSERT INTO cards VALUES (?1, ?2, ?3, ?4, 0, 0, ?5, ?6, ?7, ?8, ?9, ?10, ?11, \
                 0, ?12, ?13, 0, '')",
                params![
                    id(CARD, &card["index"], 0),
                    id(NOTE, &card["note"], 0),
                    id(DECK, &card["deck"], 1),
                    int(&card["ord"]),
                    int(&card["type"]),
                    int(&card["queue"]),
                    int(&card["due"]),
                    int(&card["ivl"]),
                    int(&card["factor"]),
                    int(&card["reps"]),
                    int(&card["lapses"]),
                    int(&card["odue"]),
                    id(DECK, &card["odeck"], 0)
                ],
            )
            .expect("a card");
        }
        for review in list(&output["revlog"]) {
            db.execute(
                "INSERT INTO revlog VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    int(&review["at_ms"]),
                    id(CARD, &review["card"], 0),
                    int(&review["usn"]),
                    int(&review["ease"]),
                    int(&review["ivl"]),
                    int(&review["last_ivl"]),
                    int(&review["factor"]),
                    int(&review["taken_ms"]),
                    int(&review["type"])
                ],
            )
            .expect("a review");
        }
    });
    path
}
