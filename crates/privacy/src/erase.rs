//! The erase: every port's erase in one transaction, and nothing an erased row held left in the
//! database's file or its write-ahead log (SPEC-021 R3; CHARTER 13;
//! `docs/schematics/data-rights-export-and-erase.md`).
//!
//! The erase runs in ONE `BEGIN IMMEDIATE` transaction ([`Db::write`]) on a connection with
//! `secure_delete` on, so every page a delete frees is written back zeroed. Every port erases in it,
//! and then every port's own export is read in it and checked against the port's declaration: each
//! table the port exports is empty, and each singleton holds exactly its one row with its declared
//! reset values. Only then does the transaction commit. A port that fails, or leaves a table as its
//! declaration forbids, drops the transaction, and every port's work rolls back with it. After the
//! commit, `VACUUM` rebuilds the file from the rows that remain and `PRAGMA
//! wal_checkpoint(TRUNCATE)` copies the rebuilt pages over the file and empties the log, so neither
//! holds an erased value.
//!
//! The connection keeps `secure_delete` on after the erase returns it to the pool, so every later
//! delete on it also zeroes what it frees.

use std::collections::BTreeMap;

use deck_streak_kernel::{DataRights, Db, Declaration, Disposition, KernelError};
use serde_json::Value;

use crate::{PrivacyError, declarations, exported};

/// What an erase did, table by table, in the ports' order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Erasure {
    /// The tables emptied.
    pub emptied: Vec<&'static str>,
    /// The singletons whose one row now holds its declared reset values.
    pub reset: Vec<&'static str>,
    /// The exempt tables, left as they were.
    pub kept: Vec<&'static str>,
    /// Whether the checkpoint after the erase met a reader, which held the write-ahead log: its
    /// older frames then remain until the next checkpoint truncates it.
    pub checkpoint_busy: bool,
}

/// Erases every table `ports` declare exported or reset, in one transaction, then compacts the
/// database.
///
/// # Errors
///
/// Every refusal of [`PrivacyError`]. [`PrivacyError::Compaction`] alone means the erase
/// committed; every other refusal means nothing was erased.
pub async fn erase(db: &Db, ports: &[&dyn DataRights]) -> Result<Erasure, PrivacyError> {
    let mut erasure = erase_rows(db, ports).await?;
    erasure.checkpoint_busy = compact(db).await.map_err(PrivacyError::Compaction)?;
    Ok(erasure)
}

/// Runs every port's erase in one `BEGIN IMMEDIATE` transaction with `secure_delete` on, checks
/// what each left against its declaration, and commits.
async fn erase_rows(db: &Db, ports: &[&dyn DataRights]) -> Result<Erasure, PrivacyError> {
    let declarations = declarations(ports)?;
    let database = |error: sqlx::Error| PrivacyError::Database(error.into());
    let mut write = db.write().await.map_err(PrivacyError::Database)?;
    let answer: i64 = sqlx::query_scalar("PRAGMA secure_delete = ON")
        .fetch_one(&mut *write)
        .await
        .map_err(database)?;
    secure_delete_is_on(answer)?;
    for (port, declaration) in ports.iter().zip(&declarations) {
        port.erase(&mut write)
            .await
            .map_err(|source| PrivacyError::Port {
                context: declaration.context(),
                source,
            })?;
    }
    let mut erasure = Erasure::default();
    for (port, declaration) in ports.iter().zip(&declarations) {
        let tables = port
            .export(&mut write)
            .await
            .map_err(|source| PrivacyError::Port {
                context: declaration.context(),
                source,
            })?;
        check(declaration, &exported(declaration, tables)?, &mut erasure)?;
    }
    write.commit().await.map_err(database)?;
    Ok(erasure)
}

/// Whether `SQLite`'s answer to `PRAGMA secure_delete = ON` says the pragma is on: 1. `FAST` (2)
/// leaves freed bytes in the freelist and is refused like off (0).
fn secure_delete_is_on(answer: i64) -> Result<(), PrivacyError> {
    if answer == 1 {
        Ok(())
    } else {
        Err(PrivacyError::SecureDeleteOff)
    }
}

/// Checks what a port's erase left, `after` being its export inside the erase's transaction: every
/// table it exports is empty, and every singleton holds exactly one row carrying its reset values.
/// Each table is added to `erasure` under what was done to it.
fn check(
    declaration: &Declaration,
    after: &BTreeMap<&'static str, Vec<Value>>,
    erasure: &mut Erasure,
) -> Result<(), PrivacyError> {
    for rights in declaration.tables() {
        let rows = after.get(rights.table).map_or(&[][..], Vec::as_slice);
        let done = match &rights.disposition {
            Disposition::ExportAndErase => rows.is_empty().then_some(&mut erasure.emptied),
            Disposition::ResetInPlace { row } => match rows {
                [only] => row
                    .iter()
                    .all(|(column, value)| only.get(column) == Some(value))
                    .then_some(&mut erasure.reset),
                _ => None,
            },
            Disposition::Exempt { .. } => Some(&mut erasure.kept),
        };
        let Some(list) = done else {
            return Err(PrivacyError::EraseIncomplete {
                context: declaration.context(),
                table: rights.table,
            });
        };
        list.push(rights.table);
    }
    Ok(())
}

/// Rebuilds the database's file from the rows that remain, then copies the rebuilt pages over the
/// file and empties the write-ahead log. Returns whether the checkpoint met a reader, which holds
/// the log's older frames until the next checkpoint; that is not an error.
async fn compact(db: &Db) -> Result<bool, KernelError> {
    // Outside any transaction: neither statement can run inside one.
    sqlx::query("VACUUM").execute(db.reader()).await?;
    let (busy, _log_frames, _checkpointed): (i64, i64, i64) =
        sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(db.reader())
            .await?;
    Ok(busy != 0)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use deck_streak_kernel::{
        DataRights, DataRightsError, Db, Declaration, Disposition, ExportedTable, PortFuture,
        TableRights,
    };
    use serde_json::json;
    use sqlx::SqliteConnection;

    use super::{compact, erase_rows, secure_delete_is_on};
    use crate::PrivacyError;

    /// A value no other test writes: the tests search the database's files for its bytes.
    const MARKER: &str = "synthetic-unit-marker-5d3c1b";

    /// A synthetic context owning one table, exported and erased.
    struct Notes;

    impl DataRights for Notes {
        fn declaration(&self) -> Result<Declaration, DataRightsError> {
            Declaration::new(
                "notes",
                vec![TableRights {
                    table: "unit_notes",
                    disposition: Disposition::ExportAndErase,
                }],
            )
        }

        fn export<'a>(
            &'a self,
            connection: &'a mut SqliteConnection,
        ) -> PortFuture<'a, Vec<ExportedTable>> {
            Box::pin(async move {
                let rows: Vec<(i64, String)> =
                    sqlx::query_as("SELECT id, body FROM unit_notes ORDER BY id")
                        .fetch_all(connection)
                        .await?;
                Ok(vec![ExportedTable {
                    table: "unit_notes",
                    rows: rows
                        .into_iter()
                        .map(|(id, body)| json!({"id": id, "body": body}))
                        .collect(),
                }])
            })
        }

        fn erase<'a>(&'a self, connection: &'a mut SqliteConnection) -> PortFuture<'a, ()> {
            Box::pin(async move {
                sqlx::query("DELETE FROM unit_notes")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        }
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

    /// A migrated database with one row whose body is the marker, checkpointed, so the row is in
    /// the database's file and its log is empty.
    async fn planted(path: &Path) -> Db {
        let db = Db::open(path).await.expect("the database opens");
        let mut write = db.write().await.expect("a write");
        sqlx::query(
            "CREATE TABLE unit_notes (id INTEGER PRIMARY KEY, body TEXT NOT NULL, \
             created_at INTEGER NOT NULL) STRICT",
        )
        .execute(&mut *write)
        .await
        .expect("the synthetic table");
        sqlx::query("INSERT INTO unit_notes (id, body, created_at) VALUES (1, ?1, 1000)")
            .bind(MARKER)
            .execute(&mut *write)
            .await
            .expect("the planted row");
        write.commit().await.expect("committed");
        let (busy, _, _): (i64, i64, i64) = sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(db.reader())
            .await
            .expect("a checkpoint");
        assert_eq!(busy, 0, "no reader holds the planted database");
        db
    }

    #[test]
    fn only_an_answer_of_one_turns_secure_delete_on() {
        assert!(secure_delete_is_on(1).is_ok());
        for answer in [0, 2] {
            assert!(
                matches!(
                    secure_delete_is_on(answer),
                    Err(PrivacyError::SecureDeleteOff)
                ),
                "{answer}"
            );
        }
    }

    #[tokio::test]
    async fn the_erase_transaction_zeroes_the_rows_it_deletes() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("unit.db");
        let db = planted(&path).await;
        // SQLite's own default leaves a deleted row's bytes in the page it frees, so the zeroing
        // this test sees is the erase's own.
        let default: i64 = sqlx::query_scalar("PRAGMA secure_delete")
            .fetch_one(db.reader())
            .await
            .expect("the pragma reads");
        assert_eq!(
            default, 0,
            "secure_delete is off unless the erase turns it on"
        );
        assert!(holds(&path), "the planted row is in the database file");

        let erasure = erase_rows(&db, &[&Notes]).await.expect("the erase");
        // The page the erase wrote to the log holds none of the row; the file keeps its old page
        // until the compaction copies the new one over it.
        assert!(
            !holds(&log_of(&path)),
            "the erase's page in the write-ahead log still holds the deleted row"
        );
        assert!(
            holds(&path),
            "the compaction, not the transaction, rewrites the file"
        );
        assert_eq!(erasure.emptied, ["unit_notes"]);
        db.close().await;
    }

    #[tokio::test]
    async fn the_compaction_leaves_no_erased_value_in_the_file_or_its_log() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("unit.db");
        let db = planted(&path).await;
        // A plain delete, with secure_delete off, leaves the row's bytes in the page it frees:
        // in the log, and in the file's old page.
        let mut write = db.write().await.expect("a write");
        sqlx::query("PRAGMA secure_delete = OFF")
            .execute(&mut *write)
            .await
            .expect("the pragma");
        sqlx::query("DELETE FROM unit_notes")
            .execute(&mut *write)
            .await
            .expect("a plain delete");
        write.commit().await.expect("committed");
        let log = log_of(&path);
        assert!(
            holds(&log),
            "the plain delete's page in the log still holds the row"
        );
        assert!(holds(&path), "the file's old page still holds the row");

        let busy = compact(&db).await.expect("the compaction");
        assert!(!busy, "no reader held the log");
        assert!(
            !holds(&path),
            "the database file still holds the erased row"
        );
        assert!(
            !holds(&log),
            "the write-ahead log still holds the erased row"
        );
        let length = std::fs::metadata(&log).map_or(0, |metadata| metadata.len());
        assert_eq!(length, 0, "the TRUNCATE checkpoint leaves an empty log");
        db.close().await;
    }
}
