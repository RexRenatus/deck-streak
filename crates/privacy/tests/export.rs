//! The export is one JSON object keyed by table: every table a port exports or resets, each holding
//! its rows, a singleton's one row included, and no exempt table; and a port whose export differs
//! from its declaration is refused by name (SPEC-021 A7, R2; CHARTER 13).
//!
//! The ports are synthetic contexts over tables this test creates in a migrated temporary database;
//! none of them is a real context, and no real port runs.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;

use deck_streak_kernel::{
    DataRights, DataRightsError, Db, Declaration, Disposition, ExportedTable, PortFuture,
    TableRights,
};
use deck_streak_privacy::{EXPORT_SCHEMA, ExportProblem, PrivacyError, SCHEMA_KEY, export};
use serde_json::{Map, Value, json};
use sqlx::SqliteConnection;
use tempfile::TempDir;

/// The synthetic contexts' tables, created beside the migrated schema, and their rows.
const FIXTURE: &str = "\
    CREATE TABLE alpha_rows (id INTEGER PRIMARY KEY, label TEXT NOT NULL, \
        created_at INTEGER NOT NULL) STRICT; \
    CREATE TABLE alpha_singleton (id INTEGER PRIMARY KEY CHECK (id = 1), \
        counter INTEGER NOT NULL, created_at INTEGER NOT NULL) STRICT; \
    INSERT INTO alpha_singleton (id, counter, created_at) VALUES (1, 0, 1000); \
    CREATE TABLE alpha_ledger (id INTEGER PRIMARY KEY, fired INTEGER NOT NULL, \
        created_at INTEGER NOT NULL) STRICT; \
    CREATE TABLE beta_notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL, \
        created_at INTEGER NOT NULL) STRICT; \
    INSERT INTO alpha_rows (id, label, created_at) \
        VALUES (1, 'first', 1001), (2, 'second', 1002), (3, 'third', 1003); \
    UPDATE alpha_singleton SET counter = 5 WHERE id = 1; \
    INSERT INTO alpha_ledger (id, fired, created_at) VALUES (1, 1, 1004), (2, 1, 1005); \
    INSERT INTO beta_notes (id, body, created_at) \
        VALUES (1, 'a synthetic note', 1006), (2, 'another synthetic note', 1007);";

/// A synthetic context: `alpha_rows` exported and erased, `alpha_singleton` reset in place, and
/// `alpha_ledger` exempt.
struct Alpha;

impl DataRights for Alpha {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        let mut reset = Map::new();
        reset.insert("counter".to_owned(), Value::from(0));
        Declaration::new(
            "alpha",
            vec![
                TableRights {
                    table: "alpha_rows",
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: "alpha_singleton",
                    disposition: Disposition::ResetInPlace { row: reset },
                },
                TableRights {
                    table: "alpha_ledger",
                    disposition: Disposition::Exempt {
                        reason: "synthetic: a ledger an erase must not re-arm",
                    },
                },
            ],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            Ok(vec![
                alpha_rows(&mut *connection).await?,
                alpha_singleton(connection).await?,
            ])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query("DELETE FROM alpha_rows")
                .execute(&mut *connection)
                .await?;
            sqlx::query("UPDATE alpha_singleton SET counter = 0 WHERE id = 1")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}

/// `alpha_rows`, as its port exports it.
async fn alpha_rows(
    connection: &mut SqliteConnection,
) -> Result<ExportedTable, deck_streak_kernel::KernelError> {
    let rows: Vec<(i64, String, i64)> =
        sqlx::query_as("SELECT id, label, created_at FROM alpha_rows ORDER BY id")
            .fetch_all(connection)
            .await?;
    Ok(ExportedTable {
        table: "alpha_rows",
        rows: rows
            .into_iter()
            .map(|(id, label, created_at)| json!({"id": id, "label": label, "created_at": created_at}))
            .collect(),
    })
}

/// `alpha_singleton`, as its port exports it.
async fn alpha_singleton(
    connection: &mut SqliteConnection,
) -> Result<ExportedTable, deck_streak_kernel::KernelError> {
    let rows: Vec<(i64, i64, i64)> =
        sqlx::query_as("SELECT id, counter, created_at FROM alpha_singleton ORDER BY id")
            .fetch_all(connection)
            .await?;
    Ok(ExportedTable {
        table: "alpha_singleton",
        rows: rows
            .into_iter()
            .map(|(id, counter, created_at)| {
                json!({"id": id, "counter": counter, "created_at": created_at})
            })
            .collect(),
    })
}

/// A second synthetic context: `beta_notes` exported and erased.
struct Beta;

impl DataRights for Beta {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            "beta",
            vec![TableRights {
                table: "beta_notes",
                disposition: Disposition::ExportAndErase,
            }],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let rows: Vec<(i64, String, i64)> =
                sqlx::query_as("SELECT id, body, created_at FROM beta_notes ORDER BY id")
                    .fetch_all(connection)
                    .await?;
            Ok(vec![ExportedTable {
                table: "beta_notes",
                rows: rows
                    .into_iter()
                    .map(|(id, body, created_at)| {
                        json!({"id": id, "body": body, "created_at": created_at})
                    })
                    .collect(),
            }])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query("DELETE FROM beta_notes")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}

/// A synthetic context that declares a singleton reset in place, and whose export leaves it out.
struct Omitting;

impl DataRights for Omitting {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        let mut reset = Map::new();
        reset.insert("counter".to_owned(), Value::from(0));
        Declaration::new(
            "omitting",
            vec![
                TableRights {
                    table: "alpha_rows",
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: "alpha_singleton",
                    disposition: Disposition::ResetInPlace { row: reset },
                },
            ],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move { Ok(vec![alpha_rows(connection).await?]) })
    }

    fn erase<'a>(&'a self, _connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}

/// A synthetic context that declares a table named for the export's schema key.
struct Reserved;

impl DataRights for Reserved {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            "reserved",
            vec![TableRights {
                table: "schema",
                disposition: Disposition::ExportAndErase,
            }],
        )
    }

    fn export<'a>(
        &'a self,
        _connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async {
            Ok(vec![ExportedTable {
                table: "schema",
                rows: vec![json!({"id": 1})],
            }])
        })
    }

    fn erase<'a>(&'a self, _connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}

/// A migrated temporary database holding the synthetic contexts' tables and rows.
async fn fixture() -> (TempDir, Db) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let mut write = db.write().await.expect("a write");
    sqlx::raw_sql(FIXTURE)
        .execute(&mut *write)
        .await
        .expect("the synthetic tables and rows");
    write.commit().await.expect("committed");
    (directory, db)
}

#[tokio::test]
async fn the_export_is_one_json_object_per_table() {
    let (_directory, db) = fixture().await;
    let exported = export(&db, &[&Alpha, &Beta]).await.expect("the export");
    let document = exported.as_json();
    let Value::Object(object) = document else {
        panic!("the export is one JSON object, not {document}");
    };
    assert_eq!(object.get(SCHEMA_KEY), Some(&Value::from(EXPORT_SCHEMA)));
    let tables: BTreeSet<&str> = object
        .keys()
        .map(String::as_str)
        .filter(|key| *key != SCHEMA_KEY)
        .collect();
    println!("examined {} table(s) in the export", tables.len());
    assert_eq!(
        tables,
        BTreeSet::from(["alpha_rows", "alpha_singleton", "beta_notes"]),
        "one key per table a port exports or resets"
    );
    // Each table holds its rows, whole.
    assert_eq!(
        object["alpha_rows"],
        json!([
            {"id": 1, "label": "first", "created_at": 1001},
            {"id": 2, "label": "second", "created_at": 1002},
            {"id": 3, "label": "third", "created_at": 1003},
        ])
    );
    assert_eq!(
        object["beta_notes"],
        json!([
            {"id": 1, "body": "a synthetic note", "created_at": 1006},
            {"id": 2, "body": "another synthetic note", "created_at": 1007},
        ])
    );
    // The singleton's one row is included.
    assert_eq!(
        object["alpha_singleton"],
        json!([{"id": 1, "counter": 5, "created_at": 1000}])
    );
    // The exempt ledger holds rows, and none of them reaches the export.
    let ledger: i64 = sqlx::query_scalar("SELECT count(*) FROM alpha_ledger")
        .fetch_one(db.reader())
        .await
        .expect("the ledger counts");
    assert_eq!(ledger, 2);
    assert!(
        !object.contains_key("alpha_ledger"),
        "an exempt table is absent from the export"
    );
    // One document on one line: the text `deckstreakd data export` writes reads back equal.
    let line = exported.to_line();
    assert_eq!(line.lines().count(), 1, "{line}");
    assert_eq!(
        &serde_json::from_str::<Value>(&line).expect("the line parses"),
        document
    );
    db.close().await;
}

#[tokio::test]
async fn a_port_whose_export_leaves_out_a_declared_table_is_refused() {
    let (_directory, db) = fixture().await;
    let refused = export(&db, &[&Omitting]).await;
    assert!(
        matches!(
            refused,
            Err(PrivacyError::ExportMismatch {
                context: "omitting",
                table: "alpha_singleton",
                problem: ExportProblem::Omitted,
            })
        ),
        "{refused:?}"
    );
    db.close().await;
}

#[tokio::test]
async fn two_ports_declaring_one_table_or_the_schema_key_are_refused() {
    let (_directory, db) = fixture().await;
    let twice = export(&db, &[&Alpha, &Beta, &Alpha]).await;
    assert!(
        matches!(
            twice,
            Err(PrivacyError::DeclaredTwice {
                table: "alpha_rows",
                first: "alpha",
                second: "alpha",
            })
        ),
        "{twice:?}"
    );
    let reserved = export(&db, &[&Beta, &Reserved]).await;
    assert!(
        matches!(
            reserved,
            Err(PrivacyError::ReservedName {
                context: "reserved",
                table: "schema",
            })
        ),
        "{reserved:?}"
    );
    db.close().await;
}

#[test]
fn each_way_an_export_differs_from_its_declaration_reads_in_words() {
    assert_eq!(
        ExportProblem::Undeclared.to_string(),
        "returned a table it does not declare exported"
    );
    assert_eq!(ExportProblem::Twice.to_string(), "returned a table twice");
    assert_eq!(
        ExportProblem::Omitted.to_string(),
        "left out a table it declares exported"
    );
    let refused = PrivacyError::ExportMismatch {
        context: "omitting",
        table: "alpha_singleton",
        problem: ExportProblem::Omitted,
    };
    assert_eq!(
        refused.to_string(),
        "the context omitting's export left out a table it declares exported: alpha_singleton"
    );
}
