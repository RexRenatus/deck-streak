//! The erase resets singletons in place and refuses a port that deletes one, leaves no erased value
//! in the database file or its write-ahead log, and rolls every port's work back when one port
//! fails (SPEC-021 A3, A5, A6, R3; CHARTER 13).
//!
//! The ports are synthetic contexts over tables this test creates in a migrated temporary database;
//! none of them is a real context, and no real port runs.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{
    DataRights, DataRightsError, Db, Declaration, Disposition, ExportedTable, KernelError,
    PortFuture, TableRights,
};
use deck_streak_privacy::{PrivacyError, erase};
use serde_json::{Map, Value, json};
use sqlx::SqliteConnection;
use tempfile::TempDir;

/// A value no other test writes: A5 searches the database's files for its bytes.
const MARKER: &str = "synthetic-erase-marker-8f41a2";

/// The synthetic contexts' tables, created beside the migrated schema.
const TABLES: &str = "\
    CREATE TABLE alpha_rows (id INTEGER PRIMARY KEY, label TEXT NOT NULL, \
        created_at INTEGER NOT NULL) STRICT; \
    CREATE TABLE alpha_singleton (id INTEGER PRIMARY KEY CHECK (id = 1), \
        counter INTEGER NOT NULL, created_at INTEGER NOT NULL) STRICT; \
    INSERT INTO alpha_singleton (id, counter, created_at) VALUES (1, 0, 1000); \
    CREATE TABLE alpha_ledger (id INTEGER PRIMARY KEY, fired INTEGER NOT NULL, \
        created_at INTEGER NOT NULL) STRICT; \
    CREATE TABLE beta_notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL, \
        created_at INTEGER NOT NULL) STRICT; \
    CREATE TABLE gamma_rows (id INTEGER PRIMARY KEY, created_at INTEGER NOT NULL) STRICT; \
    CREATE TABLE deleting_singleton (id INTEGER PRIMARY KEY CHECK (id = 1), \
        counter INTEGER NOT NULL, created_at INTEGER NOT NULL) STRICT; \
    INSERT INTO deleting_singleton (id, counter, created_at) VALUES (1, 0, 2000); \
    CREATE TABLE forgetting_rows (id INTEGER PRIMARY KEY, created_at INTEGER NOT NULL) STRICT;";

/// The rows every test starts from: none of them equals what an erase leaves.
const ROWS: &str = "\
    INSERT INTO alpha_rows (id, label, created_at) \
        VALUES (1, 'first', 1001), (2, 'second', 1002), (3, 'third', 1003); \
    UPDATE alpha_singleton SET counter = 5 WHERE id = 1; \
    INSERT INTO alpha_ledger (id, fired, created_at) VALUES (1, 1, 1004), (2, 1, 1005); \
    INSERT INTO beta_notes (id, body, created_at) \
        VALUES (1, 'a synthetic note', 1006), (2, 'another synthetic note', 1007); \
    INSERT INTO gamma_rows (id, created_at) VALUES (1, 1008); \
    UPDATE deleting_singleton SET counter = 9 WHERE id = 1; \
    INSERT INTO forgetting_rows (id, created_at) VALUES (1, 1009), (2, 1010);";

/// The reset row of a synthetic singleton: its counter back to 0.
fn counter_reset() -> Map<String, Value> {
    let mut reset = Map::new();
    reset.insert("counter".to_owned(), Value::from(0));
    reset
}

/// A synthetic context: `alpha_rows` exported and erased, `alpha_singleton` reset in place, and
/// `alpha_ledger` exempt.
struct Alpha;

impl DataRights for Alpha {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            "alpha",
            vec![
                TableRights {
                    table: "alpha_rows",
                    disposition: Disposition::ExportAndErase,
                },
                TableRights {
                    table: "alpha_singleton",
                    disposition: Disposition::ResetInPlace {
                        row: counter_reset(),
                    },
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
            let rows: Vec<(i64, String, i64)> =
                sqlx::query_as("SELECT id, label, created_at FROM alpha_rows ORDER BY id")
                    .fetch_all(&mut *connection)
                    .await?;
            Ok(vec![
                ExportedTable {
                    table: "alpha_rows",
                    rows: rows
                        .into_iter()
                        .map(|(id, label, created_at)| {
                            json!({"id": id, "label": label, "created_at": created_at})
                        })
                        .collect(),
                },
                singleton("alpha_singleton", connection).await?,
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

/// A synthetic singleton's rows, as its port exports them.
async fn singleton(
    table: &'static str,
    connection: &mut SqliteConnection,
) -> Result<ExportedTable, KernelError> {
    let sql = match table {
        "alpha_singleton" => "SELECT id, counter, created_at FROM alpha_singleton ORDER BY id",
        _ => "SELECT id, counter, created_at FROM deleting_singleton ORDER BY id",
    };
    let rows: Vec<(i64, i64, i64)> = sqlx::query_as(sql).fetch_all(connection).await?;
    Ok(ExportedTable {
        table,
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

/// A synthetic context whose erase fails: it deletes from a table no migration created.
struct Gamma;

impl DataRights for Gamma {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            "gamma",
            vec![TableRights {
                table: "gamma_rows",
                disposition: Disposition::ExportAndErase,
            }],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let rows: Vec<(i64, i64)> =
                sqlx::query_as("SELECT id, created_at FROM gamma_rows ORDER BY id")
                    .fetch_all(connection)
                    .await?;
            Ok(vec![ExportedTable {
                table: "gamma_rows",
                rows: rows
                    .into_iter()
                    .map(|(id, created_at)| json!({"id": id, "created_at": created_at}))
                    .collect(),
            }])
        })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query("DELETE FROM gamma_missing")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}

/// A synthetic context that declares its singleton reset in place and deletes its row instead.
struct Deleting;

impl DataRights for Deleting {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            "deleting",
            vec![TableRights {
                table: "deleting_singleton",
                disposition: Disposition::ResetInPlace {
                    row: counter_reset(),
                },
            }],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move { Ok(vec![singleton("deleting_singleton", connection).await?]) })
    }

    fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query("DELETE FROM deleting_singleton")
                .execute(connection)
                .await?;
            Ok(())
        })
    }
}

/// A synthetic context that declares its table exported and erased, and whose erase leaves it.
struct Forgetting;

impl DataRights for Forgetting {
    fn declaration(&self) -> Result<Declaration, DataRightsError> {
        Declaration::new(
            "forgetting",
            vec![TableRights {
                table: "forgetting_rows",
                disposition: Disposition::ExportAndErase,
            }],
        )
    }

    fn export<'a>(
        &'a self,
        connection: &'a mut SqliteConnection,
    ) -> PortFuture<'a, Vec<ExportedTable>> {
        Box::pin(async move {
            let rows: Vec<(i64, i64)> =
                sqlx::query_as("SELECT id, created_at FROM forgetting_rows ORDER BY id")
                    .fetch_all(connection)
                    .await?;
            Ok(vec![ExportedTable {
                table: "forgetting_rows",
                rows: rows
                    .into_iter()
                    .map(|(id, created_at)| json!({"id": id, "created_at": created_at}))
                    .collect(),
            }])
        })
    }

    fn erase<'a>(&'a self, _connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}

/// A migrated temporary database holding the synthetic contexts' tables and `rows`.
async fn fixture(rows: &'static str) -> (TempDir, PathBuf, Db) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("deck_streak.db");
    let db = Db::open(&path).await.expect("the database opens");
    let mut write = db.write().await.expect("a write");
    sqlx::raw_sql(TABLES)
        .execute(&mut *write)
        .await
        .expect("the synthetic tables");
    sqlx::raw_sql(rows)
        .execute(&mut *write)
        .await
        .expect("the synthetic rows");
    write.commit().await.expect("committed");
    (directory, path, db)
}

/// Every row of a synthetic table, as `(id, second column)` pairs in id order.
async fn pairs(db: &Db, sql: &'static str) -> Vec<(i64, Value)> {
    let rows: Vec<(i64, String)> = sqlx::query_as(sql)
        .fetch_all(db.reader())
        .await
        .expect("the table reads");
    rows.into_iter()
        .map(|(id, value)| (id, Value::from(value)))
        .collect()
}

/// `alpha_singleton`'s one row: its id, counter and creation instant.
async fn alpha_singleton_row(db: &Db) -> Vec<(i64, i64, i64)> {
    sqlx::query_as("SELECT id, counter, created_at FROM alpha_singleton")
        .fetch_all(db.reader())
        .await
        .expect("the singleton reads")
}

/// The write-ahead log beside the database at `path`, as `SQLite` names it.
fn log_of(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push("-wal");
    PathBuf::from(name)
}

/// Whether the file at `path` holds the marker's bytes; a missing file holds none.
fn holds(path: &Path) -> bool {
    std::fs::read(path).is_ok_and(|bytes| {
        bytes
            .windows(MARKER.len())
            .any(|window| window == MARKER.as_bytes())
    })
}

#[tokio::test]
async fn erase_resets_singletons_in_place() {
    let (_directory, _path, db) = fixture(ROWS).await;
    assert_eq!(alpha_singleton_row(&db).await, [(1, 5, 1000)]);
    let erasure = erase(&db, &[&Alpha, &Beta]).await.expect("the erase");
    // The singleton keeps its row, its id and when it was made, with its counter reset.
    assert_eq!(alpha_singleton_row(&db).await, [(1, 0, 1000)]);
    let ledger: i64 = sqlx::query_scalar("SELECT count(*) FROM alpha_ledger")
        .fetch_one(db.reader())
        .await
        .expect("the ledger counts");
    assert_eq!(ledger, 2, "the exempt ledger is left as it was");
    assert_eq!(erasure.emptied, ["alpha_rows", "beta_notes"]);
    assert_eq!(erasure.reset, ["alpha_singleton"]);
    assert_eq!(erasure.kept, ["alpha_ledger"]);

    // A port that deletes the singleton it declares reset in place is refused, and its row stays.
    let refused = erase(&db, &[&Deleting]).await;
    assert!(
        matches!(
            refused,
            Err(PrivacyError::EraseIncomplete {
                context: "deleting",
                table: "deleting_singleton",
            })
        ),
        "{refused:?}"
    );
    let kept: Vec<(i64, i64, i64)> =
        sqlx::query_as("SELECT id, counter, created_at FROM deleting_singleton")
            .fetch_all(db.reader())
            .await
            .expect("the singleton reads");
    assert_eq!(kept, [(1, 9, 2000)], "the refused erase rolled back");
    db.close().await;
}

#[tokio::test]
async fn an_erased_value_is_absent_from_the_database_file_and_its_wal() {
    let marked: &'static str = "\
        INSERT INTO alpha_rows (id, label, created_at) \
            VALUES (1, 'synthetic-erase-marker-8f41a2', 1001), (2, 'second', 1002); \
        UPDATE alpha_singleton SET counter = 5 WHERE id = 1;";
    assert!(
        marked.contains(MARKER),
        "the planted row carries the marker"
    );
    let (_directory, path, db) = fixture(marked).await;
    let log = log_of(&path);
    // The value was written: before the erase, the file or its log holds its bytes.
    assert!(
        holds(&path) || holds(&log),
        "the planted value is in neither the database file nor its log"
    );

    let erasure = erase(&db, &[&Alpha, &Beta]).await.expect("the erase");
    assert!(
        !holds(&path),
        "the database file still holds the erased value"
    );
    assert!(
        !holds(&log),
        "the write-ahead log still holds the erased value"
    );
    assert_eq!(erasure.emptied, ["alpha_rows", "beta_notes"]);
    assert!(!erasure.checkpoint_busy, "no reader held the log");
    db.close().await;
}

#[tokio::test]
async fn a_failing_port_rolls_the_whole_erase_back() {
    let (_directory, _path, db) = fixture(ROWS).await;
    // Gamma fails last, after alpha's and beta's erases ran in the same transaction.
    let failed = erase(&db, &[&Alpha, &Beta, &Gamma]).await;
    assert!(
        matches!(
            failed,
            Err(PrivacyError::Port {
                context: "gamma",
                ..
            })
        ),
        "{failed:?}"
    );
    // Every port's work rolled back: each table holds exactly the rows it started with.
    assert_eq!(
        pairs(&db, "SELECT id, label FROM alpha_rows ORDER BY id").await,
        [
            (1, Value::from("first")),
            (2, Value::from("second")),
            (3, Value::from("third"))
        ]
    );
    assert_eq!(alpha_singleton_row(&db).await, [(1, 5, 1000)]);
    assert_eq!(
        pairs(&db, "SELECT id, body FROM beta_notes ORDER BY id").await,
        [
            (1, Value::from("a synthetic note")),
            (2, Value::from("another synthetic note"))
        ]
    );
    let gamma: i64 = sqlx::query_scalar("SELECT count(*) FROM gamma_rows")
        .fetch_one(db.reader())
        .await
        .expect("gamma counts");
    assert_eq!(gamma, 1);
    db.close().await;
}

#[tokio::test]
async fn a_port_whose_erase_leaves_rows_is_refused_and_rolled_back() {
    let (_directory, _path, db) = fixture(ROWS).await;
    // Alpha erases first, in the same transaction, and the refusal rolls its work back too.
    let refused = erase(&db, &[&Alpha, &Forgetting]).await;
    assert!(
        matches!(
            refused,
            Err(PrivacyError::EraseIncomplete {
                context: "forgetting",
                table: "forgetting_rows",
            })
        ),
        "{refused:?}"
    );
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM forgetting_rows")
        .fetch_one(db.reader())
        .await
        .expect("the table counts");
    assert_eq!(left, 2);
    assert_eq!(alpha_singleton_row(&db).await, [(1, 5, 1000)]);
    db.close().await;
}
