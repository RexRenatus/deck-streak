//! A5 (SPEC-602 R5): every column of the 14 tables is kept in `DeckStreak`'s own row types, and each
//! table reads as many rows as `SELECT count(*)` counts.

#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::fs;
use std::path::Path;

use deck_streak_anki_format::{Kind, read};
use rusqlite::OpenFlags;
use support::{case_collection, engine_media, examined, golden_cases, open_file, plain, root};

/// Each table, its row type in `src/read.rs`, and whether it lives in the media database.
const TABLES: [(&str, &str, bool); 14] = [
    ("cards", "CardRow", false),
    ("col", "ColRow", false),
    ("config", "ConfigRow", false),
    ("deck_config", "DeckConfigRow", false),
    ("decks", "DeckRow", false),
    ("fields", "FieldRow", false),
    ("graves", "GraveRow", false),
    ("notes", "NoteRow", false),
    ("notetypes", "NotetypeRow", false),
    ("revlog", "RevlogRow", false),
    ("tags", "TagRow", false),
    ("templates", "TemplateRow", false),
    ("media", "MediaRow", true),
    ("meta", "MetaRow", true),
];

/// The field names of `pub struct <name>` in `source`, a raw identifier's `r#` dropped.
fn struct_fields(source: &str, name: &str) -> Option<Vec<String>> {
    let header = format!("pub struct {name} ");
    let start = source.find(&header)?;
    let mut lines = source[start..].lines();
    if lines.next()?.trim_end().ends_with("{}") {
        return Some(Vec::new());
    }
    Some(
        lines
            .take_while(|line| line.trim() != "}")
            .filter_map(|line| {
                let field = line.trim().strip_prefix("pub ")?;
                let (field, _) = field.split_once(':')?;
                Some(field.trim_start_matches("r#").to_owned())
            })
            .collect(),
    )
}

/// A column or field name with case and underscores folded, so `lastIvl` meets `last_ivl`.
fn folded(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn columns(path: &Path, table: &str) -> Vec<String> {
    let connection = plain(path, OpenFlags::SQLITE_OPEN_READ_ONLY);
    let mut statement = connection
        .prepare(&format!(
            "SELECT name FROM pragma_table_info('{table}') ORDER BY cid"
        ))
        .expect("the table's columns");
    statement
        .query_map([], |row| row.get(0))
        .expect("the columns read")
        .collect::<Result<_, _>>()
        .expect("each column's name")
}

/// The table's row count on the test's own connection. A table keyed by a `unicase` column
/// cannot be scanned without that collation, so the connection registers a stand-in.
fn count(path: &Path, table: &str) -> usize {
    let connection = plain(path, OpenFlags::SQLITE_OPEN_READ_ONLY);
    connection
        .create_collation("unicase", |a, b| a.to_lowercase().cmp(&b.to_lowercase()))
        .expect("a stand-in collation for the count");
    connection
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("the table's count")
}

#[test]
fn every_column_of_the_fourteen_tables_is_kept() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let cases = golden_cases("anki.read.collection_view");
    let collection = case_collection(dir.path(), &cases[0]);
    let media = engine_media(dir.path(), &["a.png"]);
    let source = fs::read_to_string(root().join("crates/anki-format/src/read.rs"))
        .expect("the row types' source");
    let mut findings = Vec::new();
    for (table, row_type, in_media) in examined("table(s)", TABLES.to_vec()) {
        let path = if in_media { &media } else { &collection };
        let wanted: Vec<String> = columns(path, table).iter().map(|c| folded(c)).collect();
        match struct_fields(&source, row_type) {
            None => findings.push(format!("{table}: no {row_type}")),
            Some(fields) => {
                let found: Vec<String> = fields.iter().map(|f| folded(f)).collect();
                if found != wanted {
                    findings.push(format!(
                        "{table}: {row_type} holds {found:?}, not {wanted:?}"
                    ));
                }
            }
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the columns kept");
}

#[test]
fn each_table_count_equals_its_select_count() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let cases = golden_cases("anki.read.collection_view");
    let collection = case_collection(dir.path(), &cases[0]);
    let media = engine_media(dir.path(), &["a.png", "b.mp3"]);
    let rows = read::collection(&open_file(&collection, Kind::Collection).expect("it opens"))
        .expect("the collection reads");
    let files = read::media(&open_file(&media, Kind::Media).expect("it opens"))
        .expect("the media database reads");
    let read_counts = [
        ("cards", rows.cards.len()),
        ("col", rows.col.len()),
        ("config", rows.config.len()),
        ("deck_config", rows.deck_config.len()),
        ("decks", rows.decks.len()),
        ("fields", rows.fields.len()),
        ("graves", rows.graves.len()),
        ("notes", rows.notes.len()),
        ("notetypes", rows.notetypes.len()),
        ("revlog", rows.revlog.len()),
        ("tags", rows.tags.len()),
        ("templates", rows.templates.len()),
        ("media", files.media.len()),
        ("meta", files.meta.len()),
    ];
    let mut findings = Vec::new();
    for (table, read_count) in examined("table(s)", read_counts.to_vec()) {
        let path = if table == "media" || table == "meta" {
            &media
        } else {
            &collection
        };
        let counted = count(path, table);
        if read_count != counted {
            findings.push(format!("{table}: read {read_count}, counted {counted}"));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the counts");
}

#[test]
fn a_grave_row_and_a_tag_row_are_each_read() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let collection = support::engine_collection(dir.path());
    support::with_engine(&collection, |connection| {
        connection
            .execute_batch(
                "INSERT INTO graves (oid, type, usn) VALUES (7, 1, -1); \
                 INSERT INTO tags (tag, usn, collapsed, config) VALUES ('kept', 0, 0, NULL);",
            )
            .expect("one grave and one tag");
    });
    let rows = read::collection(&open_file(&collection, Kind::Collection).expect("it opens"))
        .expect("the collection reads");
    let graves: Vec<(i64, i64, i64)> = rows
        .graves
        .iter()
        .map(|grave| (grave.oid, grave.r#type, grave.usn))
        .collect();
    assert_eq!(graves, [(7, 1, -1)], "the one grave");
    let tags: Vec<&str> = rows.tags.iter().map(|tag| tag.tag.as_str()).collect();
    assert_eq!(tags, ["kept"], "the one tag");
    assert_eq!(
        [rows.graves.len(), rows.tags.len()],
        [count(&collection, "graves"), count(&collection, "tags")],
        "each read count is the table's own"
    );
}
