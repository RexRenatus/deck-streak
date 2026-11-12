//! Every table of the migrated schema has `created_at` and is `STRICT`, the embedded migrations
//! are exactly the files in `migrations/`, and every migration names the context that owns its
//! tables in the ownership register (SPEC-020 A21 to A23, R18; ADR-020).

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Db, MIGRATOR};
use sqlx::migrate::Migrator;
use sqlx::sqlite::SqlitePool;

/// sqlx's record of the applied migrations: the schema version table sqlx creates and shapes,
/// which the ownership register records and the census leaves to it.
const SCHEMA_VERSION_TABLE: &str = "_sqlx_migrations";

/// Prints how many items a check examined and refuses zero: a census that stopped finding its
/// population must fail, never pass over the empty set (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The census of a schema: every table it examined, and why each one it refused was refused.
struct Census {
    tables: Vec<String>,
    refused: Vec<String>,
}

/// Every table of `pool`'s schema but `SQLite`'s own and sqlx's, each refused when it has no
/// `created_at` column or is not `STRICT`.
async fn census(pool: &SqlitePool) -> Census {
    let tables: Vec<(String, i64)> = sqlx::query_as(
        "SELECT name, strict FROM pragma_table_list \
         WHERE schema = 'main' AND type = 'table' AND name NOT LIKE 'sqlite%' AND name <> ? \
         ORDER BY name",
    )
    .bind(SCHEMA_VERSION_TABLE)
    .fetch_all(pool)
    .await
    .expect("the table list");
    let mut census = Census {
        tables: Vec::new(),
        refused: Vec::new(),
    };
    for (table, strict) in tables {
        let created_at: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pragma_table_info(?) WHERE name = 'created_at'",
        )
        .bind(&table)
        .fetch_one(pool)
        .await
        .expect("the table's columns");
        if created_at == 0 {
            census.refused.push(format!("{table} has no created_at"));
        }
        if strict == 0 {
            census.refused.push(format!("{table} is not STRICT"));
        }
        census.tables.push(table);
    }
    census
}

/// Every file in `migrations/`, as sqlx names a migration: its version, and its description, the
/// rest of its name with each `_` a space.
fn migration_files() -> Vec<(i64, String)> {
    let mut found = Vec::new();
    for entry in fs::read_dir(root().join("migrations")).expect("migrations/ is readable") {
        let name = entry
            .expect("a directory entry")
            .file_name()
            .into_string()
            .expect("a UTF-8 file name");
        let stem = name
            .strip_suffix(".sql")
            .unwrap_or_else(|| panic!("migrations/{name} is not a .sql migration"));
        let (version, description) = stem
            .split_once('_')
            .unwrap_or_else(|| panic!("migrations/{name} is not <version>_<description>.sql"));
        assert!(
            version.len() == 6 && version.bytes().all(|byte| byte.is_ascii_digit()),
            "migrations/{name}: a version is a SPEC number and a sequence, six digits"
        );
        found.push((
            version.parse().expect("a numeric version"),
            description.replace('_', " "),
        ));
    }
    found.sort();
    found
}

/// Every context of the map's fence in docs/CONTEXT-MAP.md, by its short name.
fn contexts() -> BTreeSet<String> {
    let map = fs::read_to_string(root().join("docs/CONTEXT-MAP.md")).expect("the context map");
    let fence = map
        .split("```context-map\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("the context map's fence");
    fence
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(|name| name.trim_start_matches("deck-streak-").to_owned())
        .collect()
}

/// The ownership register's section for the workspace's own tables: table to owning context.
fn register() -> BTreeMap<String, String> {
    let map = fs::read_to_string(root().join("docs/CONTEXT-MAP.md")).expect("the context map");
    let Some(section) = map.split("### DeckStreak's own tables\n").nth(1) else {
        return BTreeMap::new();
    };
    section
        .lines()
        .take_while(|line| !line.starts_with('#'))
        .filter(|line| line.starts_with("| `"))
        .filter_map(|line| {
            let mut cells = line
                .split('|')
                .skip(1)
                .map(|cell| cell.trim().trim_matches('`'));
            Some((cells.next()?.to_owned(), cells.next()?.to_owned()))
        })
        .collect()
}

#[tokio::test]
async fn every_table_of_the_schema_has_created_at_and_is_strict() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    let census = census(db.reader()).await;
    let tables = examined("table(s) of the migrated schema", census.tables);
    assert!(
        tables.iter().any(|table| table == "settings_generation"),
        "{tables:?}"
    );
    assert_eq!(census.refused, Vec::<String>::new());
    // The embedded migrations are exactly the files in migrations/: a file sqlx would skip, or a
    // migration added without the kernel rebuilding, reads here.
    let files = examined("migration file(s)", migration_files());
    let embedded: Vec<(i64, String)> = MIGRATOR
        .iter()
        .map(|migration| (migration.version, migration.description.to_string()))
        .collect();
    assert_eq!(embedded, files);
}

#[tokio::test]
async fn a_planted_table_without_created_at_is_refused() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let planted = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/migrations/planted");
    let migrator = Migrator::new(planted.as_path())
        .await
        .expect("the planted migrations");
    let db = Db::open_with(&directory.path().join("planted.db"), &migrator)
        .await
        .expect("the planted schema opens");
    let census = census(db.reader()).await;
    examined("table(s) of the planted schema", census.tables);
    assert_eq!(
        census.refused,
        [
            "planted_not_strict is not STRICT",
            "planted_without_created_at has no created_at",
        ]
    );
}

#[tokio::test]
async fn every_migration_names_the_context_that_owns_its_tables() {
    let register = register();
    let contexts = contexts();
    let directory = tempfile::tempdir().expect("a temporary directory");
    let empty = directory.path().join("no-migrations");
    fs::create_dir(&empty).expect("an empty migrations directory");
    let db = Db::open_with(
        &directory.path().join("stepwise.db"),
        &Migrator::new(empty.as_path())
            .await
            .expect("an empty migrator"),
    )
    .await
    .expect("an empty database opens");
    // Apply the embedded migrations one at a time, and read which tables each one created.
    let mut owned = Vec::new();
    for migration in MIGRATOR.iter() {
        let context = migration
            .description
            .split(' ')
            .next()
            .unwrap_or_default()
            .to_owned();
        assert!(
            contexts.contains(&context),
            "migration {} names {context:?}, which is not a context of the map",
            migration.version
        );
        let before = census(db.reader()).await.tables;
        sqlx::raw_sql(migration.sql.clone())
            .execute(db.reader())
            .await
            .expect("the migration applies");
        for table in census(db.reader()).await.tables {
            if !before.contains(&table) {
                owned.push((migration.version, table, context.clone()));
            }
        }
    }
    // A positive control: the decks kept away from AI are created by their own migration and owned
    // by ingest (SPEC-381 R1).
    assert!(
        owned.contains(&(38_101, "sensitive_decks".to_owned(), "ingest".to_owned())),
        "migration 38101 creates sensitive_decks for ingest: {owned:?}"
    );
    for (version, table, context) in examined("table(s) the migrations create", owned) {
        assert_eq!(
            register.get(&table),
            Some(&context),
            "migration {version} creates {table}, which the ownership register must give to {context}"
        );
    }
}
