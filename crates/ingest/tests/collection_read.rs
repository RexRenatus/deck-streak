//! The schema-18 adapter of the read port (SPEC-512 A5, A6 and A8; ADR-530 D8, D9): the 19
//! statements live once in it, each byte-equal to its text before the move; each prepares against
//! a collection the engine made; both adapters read every collection of the engine's reader golden
//! as its expected domain values; and each adapter refuses the other's file by `application_id`.
//!
//! The parity test builds, for each golden case, a schema-18 file from the rows the engine wrote
//! (over the engine's own connection, which carries Anki's collation) and a store holding the same
//! collection, loaded as an import will load it: every borrowed card returned home, a suspended
//! card marked suspended, each tag a row. The expected values are computed from the case's rows by
//! the port's rules, so neither adapter is judged only against the other.

#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anki::collection::CollectionBuilder;
use deck_streak_collection::{
    CardRead, CollectionRead, FieldRead, LogCounts, NoteRead, Origin, ReviewRead, Role, Store,
    StoreError, TemplateRead,
};
use deck_streak_ingest::schema18::{STATEMENTS, Schema18};
use rusqlite::{Connection, params};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The 19 moved statements, each with the SHA-256 of its text in the file it came from, before the
/// move.
const MOVED: [(&str, &str, &str); 19] = [
    (
        "reader.rs",
        "DECK_NAMES",
        "508ce44b2252d60cf94de7071f2fcfe0946a2e34369121e47fa0cc00ab84ca89",
    ),
    (
        "reader.rs",
        "CARDS",
        "5adc0181d5f1fee0ac70eddb38d31b3095f41204b1e9fd77b2a72b69c3f7f469",
    ),
    (
        "reader.rs",
        "REVIEWS",
        "785e93481def3a7125f999a70d8e5f9b8be944af98207d9f0f7608529791aeff",
    ),
    (
        "reader.rs",
        "CREATED",
        "77645f8d76399302494a07aa9429f3504d802a24f449e82bddd2e92d903d8b01",
    ),
    (
        "study_days.rs",
        "STUDY_EVENT_IDS",
        "99207bfde5d564d7b0ad21b8b62fc9ac9187ae0a180b126b4d45426c6a0b56cc",
    ),
    (
        "window.rs",
        "STUDY_EVENTS_BEFORE",
        "f7166482528422e078d2e34c446a687fc7751fcfde4a9d959ab1819d27812711",
    ),
    (
        "gate.rs",
        "NEWEST_REVIEW",
        "5567c919660946522cc906c7ee471a8296b6c6226d21f997ea8d45dd3687115d",
    ),
    (
        "gate.rs",
        "CARD_FINGERPRINT",
        "99d88cb548925754335fe3bf9ca8cd7725035e86885e4c1456238598ed4bccc7",
    ),
    (
        "structure.rs",
        "TEMPLATES",
        "cd46753a4ab1411a9d0d08a337655123ad0788ed0a42c15d26027199162a237b",
    ),
    (
        "structure.rs",
        "FIELDS",
        "ba5da7a715256dfb5c6c6177e8a6baa43e97f48a0edecdcf3738d247168d0aed",
    ),
    (
        "structure.rs",
        "REVIEWED_NOTES",
        "a7a95243490d01d31449c68bc0cc62086bcda80c51b7ff9688c8298a52192918",
    ),
    (
        "structure.rs",
        "NOTE_FIELDS",
        "01a9d3b5b2f4d21f25889ed058434e14ea16e251de2a2ee866f3027164850799",
    ),
    (
        "engine.rs",
        "CARD_STATE",
        "3470bfcec7053e8c91d29f8c4bc506a74b2ea839938fe44a148915eeb3676f3c",
    ),
    (
        "skip_write.rs",
        "STUDIED_AFTER",
        "39fb38b64c0371bbbebe2a40457a19e3476611bff0481cc350bb392ebed0c650",
    ),
    (
        "skip_write.rs",
        "CARD_COUNT",
        "f1e136124e3659823b98df23ddaf8a3eba3b05dccc459a012193fbdb9923332b",
    ),
    (
        "skip_write.rs",
        "NOTE_COUNT",
        "41540a347080c34ed575f652136a04c429b7955e7ec8f238076ad1ac8f8a2592",
    ),
    (
        "skip_write.rs",
        "REVIEW_ROW_COUNT",
        "e5c65a07aa7ac3a68bace59695886a83c7e21c0bbe56202039d3cc1a86f1fe64",
    ),
    (
        "skip_write.rs",
        "RESCHEDULE_ROW_COUNT",
        "c7505c4afe9807e50ab490fb75c87d2b28dae75b24c8f867470fa2b10e2d19c7",
    ),
    (
        "skip_write.rs",
        "CARDS_BY_QUEUE_AND_TYPE",
        "83f5068786c15db3c73d2154cec945e302725fd6c259972aa7a831f68b6d8d35",
    ),
];

/// The adapter's module.
const ADAPTER: &str = "crates/ingest/src/schema18.rs";

/// The server's role for the stores these tests load.
const SERVER: Role = Role::Server {
    origin: Origin::new([7; 16]),
};

/// The writer id an imported review keeps: "anki-revlog", padded with zero bytes.
const IMPORTED: [u8; 16] = *b"anki-revlog\0\0\0\0\0";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn the_moved_statements_live_once_in_the_schema_18_adapter() {
    let mut findings = Vec::new();
    let registered: BTreeMap<&str, &str> = STATEMENTS
        .iter()
        .map(|statement| (statement.name, statement.sql))
        .collect();
    for (file, name, digest) in examined("moved statement(s)", MOVED.to_vec()) {
        match registered.get(name) {
            None => findings.push(format!("{name} (from {file}) is not in the adapter")),
            Some(sql) => {
                let found = hex(&Sha256::digest(sql.as_bytes()));
                if found != digest {
                    findings.push(format!("{name}: its text changed (sha256 {found})"));
                }
            }
        }
    }
    if registered.len() != MOVED.len() {
        findings.push(format!(
            "the adapter registers {} statements",
            registered.len()
        ));
    }
    // Each is defined once in ingest's sources, in the adapter, and the seven files hold no SQL.
    let src = root().join("crates/ingest/src");
    let mut sources = Vec::new();
    for entry in fs::read_dir(&src).expect("ingest's sources") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let name = path
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .into_owned();
            sources.push((name, fs::read_to_string(&path).expect("a readable source")));
        }
    }
    for (_, name, _) in MOVED {
        let definition = format!("const {name}: &str");
        let defining: Vec<&str> = sources
            .iter()
            .filter(|(_, text)| text.contains(&definition))
            .map(|(file, _)| file.as_str())
            .collect();
        if defining != ["schema18.rs"] {
            findings.push(format!("{name} is defined in {defining:?}"));
        }
    }
    let seven: BTreeSet<&str> = MOVED.iter().map(|(file, _, _)| *file).collect();
    for (file, text) in &sources {
        if !seven.contains(file.as_str()) {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if code.to_ascii_uppercase().contains("\"SELECT ") {
                findings.push(format!("{file}:{} holds SQL text", index + 1));
            }
        }
    }
    let adapter = fs::read_to_string(root().join(ADAPTER)).unwrap_or_default();
    if adapter.contains("anki::") {
        findings.push("the adapter names the engine's crate".to_owned());
    }
    assert_eq!(findings, Vec::<String>::new(), "the moved statements");
}

/// A collection the engine makes at `path`, closed.
fn engine_collection(path: &Path) {
    let col = CollectionBuilder::new(path)
        .build()
        .expect("the engine creates the collection");
    col.close(None).expect("the engine closes the collection");
}

#[test]
fn every_schema_18_statement_prepares_against_an_engine_collection() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("collection.anki2");
    engine_collection(&path);
    let file = Schema18::open(&path).expect("the adapter opens the engine's collection");
    let mut findings = Vec::new();
    for statement in examined("adapter statement(s)", STATEMENTS.to_vec()) {
        if let Err(error) = file.check_statement(statement.sql) {
            findings.push(format!("{}: {error:?}", statement.name));
        }
    }
    assert_eq!(
        findings,
        Vec::<String>::new(),
        "statements that do not prepare"
    );
    assert!(
        file.check_statement("SELECT id, nmae FROM decks ORDER BY id")
            .is_err(),
        "a misspelled column prepares"
    );
}

#[test]
fn each_adapter_refuses_the_others_file_by_application_id() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let anki = dir.path().join("collection.anki2");
    let store = dir.path().join("collection.store");
    engine_collection(&anki);
    drop(Store::open(&store, SERVER).expect("the store is made"));
    let mut findings = Vec::new();
    let cases: Vec<(&str, Result<(), String>, bool)> = vec![
        (
            "the schema-18 adapter on a store",
            Schema18::open(&store)
                .map(drop)
                .map_err(|error| format!("{error:?}")),
            matches!(Schema18::open(&store), Err(StoreError::StoreFile)),
        ),
        (
            "the store's opener on a schema-18 file",
            Store::open(&anki, SERVER)
                .map(drop)
                .map_err(|error| format!("{error:?}")),
            matches!(Store::open(&anki, SERVER), Err(StoreError::ForeignFile(0))),
        ),
        (
            "the store's reader on a schema-18 file",
            Store::open_reader(&anki)
                .map(drop)
                .map_err(|error| format!("{error:?}")),
            matches!(Store::open_reader(&anki), Err(StoreError::ForeignFile(0))),
        ),
    ];
    for (what, answer, refused) in examined("cross-open(s)", cases) {
        if !refused {
            findings.push(format!("{what}: answered {answer:?}"));
        }
    }
    // Each still opens its own file.
    if let Err(error) = Schema18::open(&anki) {
        findings.push(format!(
            "the schema-18 adapter refused its own file: {error}"
        ));
    }
    if let Err(error) = Store::open_reader(&store) {
        findings.push(format!("the store's reader refused its own file: {error}"));
    }
    assert_eq!(findings, Vec::<String>::new(), "the refusals");
}

/// One golden case's collection, in its own ids: the case's corpus indices mapped to ids.
struct Case {
    class: String,
    crt: i64,
    /// Deck id, name, usn, filtered.
    decks: Vec<(i64, String, i64, bool)>,
    /// Notetype id, name, usn, kind, css.
    notetypes: Vec<(i64, String, i64, i64, String)>,
    /// Notetype id, ordinal, name.
    fields: Vec<(i64, i64, String)>,
    /// Notetype id, ordinal, name, usn, front, back.
    templates: Vec<(i64, i64, String, i64, String, String)>,
    /// Id, guid, notetype, usn, tags, flds, sfld, csum, flags, data.
    notes: Vec<(
        i64,
        String,
        i64,
        i64,
        String,
        String,
        String,
        i64,
        i64,
        String,
    )>,
    /// The 18 schema-18 card columns.
    cards: Vec<[Value; 18]>,
    /// Id, card, usn, ease, ivl, lastIvl, factor, time, type.
    revlog: Vec<[i64; 9]>,
}

fn int(value: &Value) -> i64 {
    value.as_i64().expect("an integer")
}

fn text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn deck_id(index: &Value) -> i64 {
    index.as_i64().map_or(1, |index| 1000 + index)
}

fn notetype_id(index: &Value) -> i64 {
    index.as_i64().map_or(1, |index| 100 + index)
}

fn note_id(index: &Value) -> i64 {
    10_000 + int(index)
}

fn card_id(index: &Value) -> i64 {
    20_000 + int(index)
}

impl Case {
    fn of(case: &Value) -> Self {
        let input = &case["input"]["collection"];
        let tables = &case["output"]["tables"];
        let rows = |table: &str| tables[table].as_array().cloned().unwrap_or_default();
        let input_notetype =
            |index: &Value| index.as_u64().map(|i| &input["notetypes"][i as usize]);
        let decks = rows("decks")
            .iter()
            .map(|row| {
                let filtered = row[0]
                    .as_u64()
                    .is_some_and(|i| input["decks"][i as usize]["filtered"] == Value::Bool(true));
                (deck_id(&row[0]), text(&row[1]), int(&row[2]), filtered)
            })
            .collect();
        let notetypes = rows("notetypes")
            .iter()
            .map(|row| {
                let source = input_notetype(&row[0]);
                let kind = i64::from(source.is_some_and(|nt| nt["kind"] == "cloze"));
                let css = source.map(|nt| text(&nt["css"])).unwrap_or_default();
                (notetype_id(&row[0]), text(&row[1]), int(&row[2]), kind, css)
            })
            .collect();
        let fields = rows("fields")
            .iter()
            .map(|row| (notetype_id(&row[0]), int(&row[1]), text(&row[2])))
            .collect();
        let templates = rows("templates")
            .iter()
            .map(|row| {
                let source = input_notetype(&row[0])
                    .and_then(|nt| nt["templates"].get(int(&row[1]) as usize).cloned());
                let side = |key: &str| source.as_ref().map(|t| text(&t[key])).unwrap_or_default();
                (
                    notetype_id(&row[0]),
                    int(&row[1]),
                    text(&row[2]),
                    int(&row[3]),
                    side("front"),
                    side("back"),
                )
            })
            .collect();
        let notes = rows("notes")
            .iter()
            .map(|row| {
                (
                    note_id(&row[0]),
                    text(&row[1]),
                    notetype_id(&row[2]),
                    int(&row[3]),
                    text(&row[4]),
                    text(&row[5]),
                    text(&row[6]),
                    int(&row[7]),
                    int(&row[8]),
                    text(&row[9]),
                )
            })
            .collect();
        let cards = rows("cards")
            .iter()
            .map(|row| {
                // id, nid, did, ord, usn, type, queue, due, ivl, factor, reps, lapses, left, odue,
                // odid, flags, data: written back as the 18 schema-18 columns, mod 0.
                let odid = if row[14].is_null() {
                    0
                } else {
                    deck_id(&row[14])
                };
                [
                    Value::from(card_id(&row[0])),
                    Value::from(note_id(&row[1])),
                    Value::from(deck_id(&row[2])),
                    row[3].clone(),
                    Value::from(0),
                    row[4].clone(),
                    row[5].clone(),
                    row[6].clone(),
                    row[7].clone(),
                    row[8].clone(),
                    row[9].clone(),
                    row[10].clone(),
                    row[11].clone(),
                    row[12].clone(),
                    row[13].clone(),
                    Value::from(odid),
                    row[15].clone(),
                    Value::from(text(&row[16])),
                ]
            })
            .collect();
        let revlog = rows("revlog")
            .iter()
            .map(|row| {
                [
                    int(&row[0]),
                    card_id(&row[1]),
                    int(&row[2]),
                    int(&row[3]),
                    int(&row[4]),
                    int(&row[5]),
                    int(&row[6]),
                    int(&row[7]),
                    int(&row[8]),
                ]
            })
            .collect();
        Self {
            class: case["class"].as_str().unwrap_or("drawn").to_owned(),
            crt: input["crt"].as_i64().unwrap_or(0),
            decks,
            notetypes,
            fields,
            templates,
            notes,
            cards,
            revlog,
        }
    }

    fn home(card: &[Value; 18]) -> i64 {
        let odid = int(&card[15]);
        if odid != 0 { odid } else { int(&card[2]) }
    }

    /// The schema-18 file, written by the engine's own connection.
    fn write_schema_18(&self, path: &Path) {
        let col = CollectionBuilder::new(path)
            .build()
            .expect("the engine creates the collection");
        let db = col.storage.db();
        db.execute_batch(
            "DELETE FROM cards; DELETE FROM notes; DELETE FROM revlog; DELETE FROM decks; \
             DELETE FROM notetypes; DELETE FROM fields; DELETE FROM templates; DELETE FROM graves;",
        )
        .expect("the engine's stock rows are cleared");
        db.execute("UPDATE col SET crt = ?1", [self.crt])
            .expect("the creation stamp");
        for (id, name, usn, _) in &self.decks {
            db.execute(
                "INSERT INTO decks VALUES (?1, ?2, 0, ?3, x'', x'')",
                params![id, name, usn],
            )
            .expect("a deck");
        }
        for (id, name, usn, _, _) in &self.notetypes {
            db.execute(
                "INSERT INTO notetypes VALUES (?1, ?2, 0, ?3, x'')",
                params![id, name, usn],
            )
            .expect("a notetype");
        }
        for (ntid, ord, name) in &self.fields {
            db.execute(
                "INSERT INTO fields VALUES (?1, ?2, ?3, x'')",
                params![ntid, ord, name],
            )
            .expect("a field");
        }
        for (ntid, ord, name, usn, _, _) in &self.templates {
            db.execute(
                "INSERT INTO templates VALUES (?1, ?2, ?3, 0, ?4, x'')",
                params![ntid, ord, name, usn],
            )
            .expect("a template");
        }
        for (id, guid, mid, usn, tags, flds, sfld, csum, flags, data) in &self.notes {
            db.execute(
                "INSERT INTO notes VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![id, guid, mid, usn, tags, flds, sfld, csum, flags, data],
            )
            .expect("a note");
        }
        for card in &self.cards {
            let values: Vec<rusqlite::types::Value> = card
                .iter()
                .map(|value| match value {
                    Value::String(text) => rusqlite::types::Value::Text(text.clone()),
                    other => rusqlite::types::Value::Integer(other.as_i64().unwrap_or(0)),
                })
                .collect();
            db.execute(
                "INSERT INTO cards VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, \
                 ?13, ?14, ?15, ?16, ?17, ?18)",
                rusqlite::params_from_iter(values),
            )
            .expect("a card");
        }
        for row in &self.revlog {
            db.execute(
                "INSERT INTO revlog VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params_from_iter(row.iter()),
            )
            .expect("a review");
        }
        col.close(None).expect("the engine closes the collection");
    }

    /// The store holding the same collection, as an import loads it.
    fn write_store(&self, path: &Path) -> Result<(), String> {
        self.load_store(path)
            .map_err(|error| format!("the store refused the case: {error}"))
    }

    fn load_store(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        drop(Store::open(path, SERVER)?);
        let mut conn = Connection::open(path)?;
        let tx = conn.transaction().expect("a fixture transaction");
        tx.execute("INSERT INTO presets VALUES (1, 'Default', '{}', 0)", [])?;
        for (id, name, _, filtered) in &self.decks {
            tx.execute(
                "INSERT INTO decks VALUES (?1, ?2, ?3, 1, ?4, '{}', 0)",
                params![id, name, name.to_lowercase(), i64::from(*filtered)],
            )?;
        }
        for (id, name, _, kind, css) in &self.notetypes {
            tx.execute(
                "INSERT INTO notetypes VALUES (?1, ?2, ?3, ?4, ?5, 0, '{}', 0)",
                params![id, name, name.to_lowercase(), kind, css],
            )?;
        }
        for (ntid, ord, name) in &self.fields {
            tx.execute(
                "INSERT INTO fields VALUES (?1, ?2, ?3, '{}')",
                params![ntid, ord, name],
            )?;
        }
        for (ntid, ord, name, _, front, back) in &self.templates {
            tx.execute(
                "INSERT INTO templates VALUES (?1, ?2, ?3, ?4, ?5, '{}')",
                params![ntid, ord, name, front, back],
            )?;
        }
        for (id, guid, mid, _, tags, flds, sfld, _, _, _) in &self.notes {
            let fields: Vec<&str> = flds.split('\u{1f}').collect();
            let first = Sha256::digest(fields.first().copied().unwrap_or("").as_bytes());
            tx.execute(
                "INSERT INTO notes VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                params![
                    id,
                    guid,
                    mid,
                    serde_json::to_string(&fields).expect("the fields as JSON"),
                    sfld,
                    first.as_slice()
                ],
            )?;
            for tag in tags.split_whitespace() {
                tx.execute(
                    "INSERT OR IGNORE INTO note_tags VALUES (?1, ?2, ?3)",
                    params![id, tag.to_lowercase(), tag],
                )?;
            }
        }
        for card in &self.cards {
            let kind = int(&card[6]);
            let due = int(&card[8]);
            let new_position = (kind == 0 && due >= 0).then_some(due);
            tx.execute(
                "INSERT INTO cards VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, 0)",
                params![
                    int(&card[0]),
                    int(&card[1]),
                    int(&card[3]),
                    Self::home(card),
                    new_position,
                    i64::from(int(&card[7]) == -1),
                    int(&card[16]) & 7,
                ],
            )?;
        }
        let cards: BTreeMap<i64, &[Value; 18]> = self
            .cards
            .iter()
            .map(|card| (int(&card[0]), card))
            .collect();
        for (sequence, row) in self.revlog.iter().enumerate() {
            let card = cards.get(&row[1]);
            tx.execute(
                "INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, \
                 kind, ease, duration_ms, study_day, server_seq) \
                 VALUES (?1, ?2, ?1, ?3, ?4, ?5, 1, ?6, ?7, ?8, 0, ?9)",
                params![
                    row[0],
                    IMPORTED.as_slice(),
                    row[1],
                    card.map_or(0, |card| int(&card[1])),
                    card.map_or(0, |card| Self::home(card)),
                    row[8],
                    row[3],
                    row[7],
                    i64::try_from(sequence)? + 1,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn deck_names(&self) -> BTreeMap<i64, String> {
        self.decks
            .iter()
            .map(|(id, name, _, _)| (*id, name.clone()))
            .collect()
    }

    fn tags_of(&self, note: i64) -> BTreeSet<String> {
        self.notes
            .iter()
            .find(|row| row.0 == note)
            .map(|row| row.4.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default()
    }

    fn cards_in(&self, scope: &BTreeSet<i64>) -> Vec<CardRead> {
        let mut cards: Vec<CardRead> = self
            .cards
            .iter()
            .filter(|card| scope.contains(&Self::home(card)))
            .map(|card| CardRead {
                id: int(&card[0]),
                note_id: int(&card[1]),
                home_deck_id: Self::home(card),
                suspended: int(&card[7]) == -1,
                tags: self.tags_of(int(&card[1])),
            })
            .collect();
        cards.sort_by_key(|card| card.id);
        cards
    }

    fn scoped_reviews(&self, scope: &BTreeSet<i64>) -> Vec<[i64; 9]> {
        let homes: BTreeMap<i64, i64> = self
            .cards
            .iter()
            .map(|card| (int(&card[0]), Self::home(card)))
            .collect();
        let mut rows: Vec<[i64; 9]> = self
            .revlog
            .iter()
            .filter(|row| homes.get(&row[1]).is_some_and(|home| scope.contains(home)))
            .copied()
            .collect();
        rows.sort_by_key(|row| row[0]);
        rows
    }

    fn study(row: &[i64; 9]) -> bool {
        (0..=3).contains(&row[8]) && row[3] >= 1
    }

    fn study_reviews_after(&self, floor: i64, scope: &BTreeSet<i64>) -> Vec<ReviewRead> {
        self.scoped_reviews(scope)
            .iter()
            .filter(|row| row[0] > floor && Self::study(row))
            .map(|row| ReviewRead {
                id: row[0],
                card_id: row[1],
                ease: row[3],
                duration_ms: row[7],
                kind: row[8],
            })
            .collect()
    }

    fn study_reviews_through(&self, floor: i64, scope: &BTreeSet<i64>) -> i64 {
        let through = self
            .scoped_reviews(scope)
            .iter()
            .filter(|row| row[0] <= floor && Self::study(row))
            .count();
        i64::try_from(through).expect("a count")
    }

    fn notetypes_in(&self, scope: &BTreeSet<i64>) -> BTreeSet<i64> {
        self.cards_in(scope)
            .iter()
            .filter_map(|card| self.notes.iter().find(|row| row.0 == card.note_id))
            .map(|row| row.2)
            .collect()
    }

    fn templates_in(&self, scope: &BTreeSet<i64>) -> Vec<TemplateRead> {
        let used = self.notetypes_in(scope);
        let names: BTreeMap<i64, &String> =
            self.notetypes.iter().map(|row| (row.0, &row.1)).collect();
        let mut templates: Vec<TemplateRead> = self
            .templates
            .iter()
            .filter(|row| used.contains(&row.0))
            .map(|row| TemplateRead {
                notetype_id: row.0,
                ord: row.1,
                notetype_name: names
                    .get(&row.0)
                    .map(|name| (*name).clone())
                    .unwrap_or_default(),
                name: row.2.clone(),
            })
            .collect();
        templates.sort_by_key(|template| (template.notetype_id, template.ord));
        templates
    }

    fn fields_in(&self, scope: &BTreeSet<i64>) -> Vec<FieldRead> {
        let used = self.notetypes_in(scope);
        let mut fields: Vec<FieldRead> = self
            .fields
            .iter()
            .filter(|row| used.contains(&row.0))
            .map(|row| FieldRead {
                notetype_id: row.0,
                ord: row.1,
                name: row.2.clone(),
            })
            .collect();
        fields.sort_by_key(|field| (field.notetype_id, field.ord));
        fields
    }

    fn reviewed_notes(&self, scope: &BTreeSet<i64>) -> BTreeSet<i64> {
        let notes: BTreeMap<i64, i64> = self
            .cards
            .iter()
            .map(|card| (int(&card[0]), int(&card[1])))
            .collect();
        self.scoped_reviews(scope)
            .iter()
            .filter(|row| row[8] == 1 && row[3] >= 1)
            .filter_map(|row| notes.get(&row[1]).copied())
            .collect()
    }

    fn note_fields(&self) -> Vec<NoteRead> {
        let mut notes: Vec<NoteRead> = self
            .notes
            .iter()
            .map(|row| NoteRead {
                id: row.0,
                notetype_id: row.2,
                fields: row.5.split('\u{1f}').map(str::to_owned).collect(),
            })
            .collect();
        notes.sort_by_key(|note| note.id);
        notes
    }

    fn studied_cards_after(&self, after: i64) -> BTreeSet<i64> {
        self.revlog
            .iter()
            .filter(|row| row[0] > after && Self::study(row))
            .map(|row| row[1])
            .collect()
    }

    fn counts(&self) -> LogCounts {
        let count = |n: usize| i64::try_from(n).expect("a count");
        LogCounts {
            cards: count(self.cards.len()),
            notes: count(self.notes.len()),
            review_rows: count(self.revlog.len()),
            reschedule_rows: count(
                self.revlog
                    .iter()
                    .filter(|row| row[8] == 4 && row[3] == 0)
                    .count(),
            ),
        }
    }
}

/// Every answer of `read` over `case`'s scopes and floors, beside the answer the case's rows give.
fn compare(case: &Case, read: &dyn CollectionRead) -> Result<Vec<String>, StoreError> {
    let mut findings = Vec::new();
    let mut check = |what: String, answered: String, expected: String| {
        if answered != expected {
            findings.push(format!("{what}: answered {answered}, expected {expected}"));
        }
    };
    let all: BTreeSet<i64> = case.decks.iter().map(|deck| deck.0).collect();
    let first: BTreeSet<i64> = case
        .decks
        .iter()
        .find(|deck| deck.0 != 1)
        .map(|deck| deck.0)
        .into_iter()
        .collect();
    let mut ids: Vec<i64> = case.revlog.iter().map(|row| row[0]).collect();
    ids.sort_unstable();
    let middle = ids.get(ids.len() / 2).copied().unwrap_or(0);
    check(
        "deck names".into(),
        format!("{:?}", read.deck_names()?),
        format!("{:?}", case.deck_names()),
    );
    for (label, scope) in [("all", &all), ("first", &first), ("none", &BTreeSet::new())] {
        check(
            format!("cards in {label}"),
            format!("{:?}", read.cards_in(scope)?),
            format!("{:?}", case.cards_in(scope)),
        );
        for floor in [0, middle] {
            let expected = case.study_reviews_after(floor, scope);
            check(
                format!("study reviews of {label} above {floor}"),
                format!("{:?}", read.study_reviews_after(floor, scope)?),
                format!("{expected:?}"),
            );
            let first_one: Vec<i64> = expected.iter().take(1).map(|review| review.id).collect();
            check(
                format!("one study review id of {label} above {floor}"),
                format!("{:?}", read.study_review_ids_after(floor, scope, 1)?),
                format!("{first_one:?}"),
            );
            let every: Vec<i64> = expected.iter().map(|review| review.id).collect();
            check(
                format!("study review ids of {label} above {floor}"),
                format!("{:?}", read.study_review_ids_after(floor, scope, 50_000)?),
                format!("{every:?}"),
            );
            check(
                format!("study reviews of {label} through {floor}"),
                format!("{:?}", read.study_reviews_through(floor, scope)?),
                format!("{:?}", case.study_reviews_through(floor, scope)),
            );
        }
        check(
            format!("templates of {label}"),
            format!("{:?}", read.templates_in(scope)?),
            format!("{:?}", case.templates_in(scope)),
        );
        check(
            format!("fields of {label}"),
            format!("{:?}", read.fields_in(scope)?),
            format!("{:?}", case.fields_in(scope)),
        );
        check(
            format!("reviewed notes of {label}"),
            format!("{:?}", read.reviewed_notes(scope)?),
            format!("{:?}", case.reviewed_notes(scope)),
        );
    }
    check(
        "newest review".into(),
        format!("{:?}", read.newest_review()?),
        format!("{:?}", ids.last()),
    );
    let notes: BTreeSet<i64> = case.notes.iter().map(|row| row.0).collect();
    check(
        "note fields".into(),
        format!("{:?}", read.note_fields(&notes)?),
        format!("{:?}", case.note_fields()),
    );
    for after in [0, middle] {
        check(
            format!("cards studied after {after}"),
            format!("{:?}", read.studied_cards_after(after)?),
            format!("{:?}", case.studied_cards_after(after)),
        );
    }
    check(
        "counts".into(),
        format!("{:?}", read.counts()?),
        format!("{:?}", case.counts()),
    );
    Ok(findings)
}

#[test]
fn both_adapters_read_each_golden_collection_as_its_expected_domain_values() {
    let golden = root().join("tools/parity-oracle/goldens/anki.read.reader_fixture.json");
    let golden: Value =
        serde_json::from_str(&fs::read_to_string(golden).expect("the golden reads")).expect("JSON");
    let cases = golden["cases"].as_array().cloned().unwrap_or_default();
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut findings = Vec::new();
    for (index, case) in examined("golden case(s)", cases).iter().enumerate() {
        let case = Case::of(case);
        let anki = dir.path().join(format!("case-{index}.anki2"));
        let store = dir.path().join(format!("case-{index}.store"));
        case.write_schema_18(&anki);
        let loaded = case.write_store(&store);
        let schema18 = Schema18::open(&anki).expect("the adapter opens the case's file");
        match compare(&case, &schema18) {
            Ok(found) => findings.extend(
                found
                    .into_iter()
                    .map(|f| format!("case {index} ({}), schema 18: {f}", case.class)),
            ),
            Err(error) => findings.push(format!("case {index}, schema 18: {error}")),
        }
        if let Err(why) = loaded {
            findings.push(format!("case {index} ({}): {why}", case.class));
            continue;
        }
        let store = Store::open_reader(&store).expect("the reader opens the case's store");
        match store.read(|snapshot| compare(&case, snapshot)) {
            Ok(found) => findings.extend(
                found
                    .into_iter()
                    .map(|f| format!("case {index} ({}), store: {f}", case.class)),
            ),
            Err(error) => findings.push(format!("case {index}, store: {error}")),
        }
    }
    let shown: Vec<&String> = findings.iter().take(12).collect();
    assert!(
        findings.is_empty(),
        "{} finding(s), the first: {shown:#?}",
        findings.len()
    );
}
