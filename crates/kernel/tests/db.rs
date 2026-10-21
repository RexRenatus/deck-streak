//! The repository base opens with the predecessor's pragmas, serialises writers, opens a foreign
//! file read-only, and applies a migration that arrives late (SPEC-020 A17 to A20, R19).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use deck_streak_kernel::Db;
use deck_streak_kernel::db::DB_BUSY_TIMEOUT_MS;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::{ConnectOptions, Connection};

/// Writes each writer makes in the concurrency test.
const WRITES: i64 = 60;

/// A fixture migration set under `tests/fixtures/migrations/`.
fn fixtures(set: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/migrations")
        .join(set)
}

/// The names of the tables `pool`'s schema holds, but sqlx's and `SQLite`'s own.
async fn tables(pool: &SqlitePool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT name FROM sqlite_schema WHERE type = 'table' \
         AND name NOT LIKE 'sqlite%' AND name <> '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .expect("the schema is readable")
}

/// A counter the writers race on, in a table of the test's own.
async fn counter(db: &Db) {
    let mut write = db.write().await.expect("a write");
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS counter \
         (id INTEGER PRIMARY KEY, value INTEGER NOT NULL, created_at INTEGER NOT NULL) STRICT; \
         INSERT OR IGNORE INTO counter (id, value, created_at) VALUES (1, 0, 0);",
    )
    .execute(&mut *write)
    .await
    .expect("the counter is created");
    write.commit().await.expect("the counter is committed");
}

/// Adds one to the counter `times` times, each by a read and then a write inside one write
/// transaction: the shape that loses an update when two transactions interleave.
async fn increment(db: Db, times: i64) -> Result<i64, String> {
    for done in 0..times {
        let mut write = db
            .write()
            .await
            .map_err(|error| format!("write {done}: {error:?}"))?;
        let value: i64 = sqlx::query_scalar("SELECT value FROM counter WHERE id = 1")
            .fetch_one(&mut *write)
            .await
            .map_err(|error| format!("read {done}: {error}"))?;
        sqlx::query("UPDATE counter SET value = ? WHERE id = 1")
            .bind(value + 1)
            .execute(&mut *write)
            .await
            .map_err(|error| format!("update {done}: {error}"))?;
        write
            .commit()
            .await
            .map_err(|error| format!("commit {done}: {error}"))?;
    }
    Ok(times)
}

/// Another program's `SQLite` file, made the way that program makes it: its own connection, the
/// default rollback journal, one table, one row. Only a fixture constructs its own connection.
async fn foreign_file(path: &Path) {
    let mut connection = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .connect()
        .await
        .expect("the foreign file is created");
    sqlx::raw_sql(
        "CREATE TABLE cards (id INTEGER PRIMARY KEY, name TEXT NOT NULL); \
         INSERT INTO cards (name) VALUES ('first card');",
    )
    .execute(&mut connection)
    .await
    .expect("the foreign file is written");
    connection
        .close()
        .await
        .expect("the foreign file is closed");
}

#[tokio::test]
async fn opening_the_database_sets_wal_normal_sync_foreign_keys_and_the_busy_timeout() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    // Two connections at once, so the pragmas are every connection's and not one's.
    let mut first = db.reader().acquire().await.expect("a connection");
    let mut second = db.reader().acquire().await.expect("a second connection");
    for connection in [&mut first, &mut second] {
        let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&mut **connection)
            .await
            .expect("journal_mode");
        assert_eq!(journal, "wal");
        let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
            .fetch_one(&mut **connection)
            .await
            .expect("synchronous");
        assert_eq!(synchronous, 1, "synchronous is NORMAL");
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&mut **connection)
            .await
            .expect("foreign_keys");
        assert_eq!(foreign_keys, 1);
        let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(&mut **connection)
            .await
            .expect("busy_timeout");
        assert_eq!(u64::try_from(busy_timeout).ok(), Some(DB_BUSY_TIMEOUT_MS));
    }
}

#[tokio::test]
async fn two_concurrent_writers_serialise_without_a_lost_update() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("deckstreak.db");
    // Two pools on one file, as two roles of the daemon hold it.
    let first = Db::open(&path).await.expect("the database opens");
    let second = Db::open(&path).await.expect("the database opens twice");
    counter(&first).await;

    let (from_first, from_second) = tokio::join!(
        tokio::spawn(increment(first.clone(), WRITES)),
        tokio::spawn(increment(second.clone(), WRITES)),
    );
    assert_eq!(from_first.expect("the first writer's task"), Ok(WRITES));
    assert_eq!(from_second.expect("the second writer's task"), Ok(WRITES));
    let value: i64 = sqlx::query_scalar("SELECT value FROM counter WHERE id = 1")
        .fetch_one(first.reader())
        .await
        .expect("the counter");
    assert_eq!(value, 2 * WRITES, "every write of both writers is counted");
}

#[tokio::test]
async fn a_foreign_sqlite_file_opened_read_only_refuses_a_write() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("collection.anki2");
    foreign_file(&path).await;

    let foreign = Db::open_foreign_read_only(&path)
        .await
        .expect("the foreign file opens");
    let names: Vec<String> = sqlx::query_scalar("SELECT name FROM cards ORDER BY id")
        .fetch_all(foreign.reader())
        .await
        .expect("the foreign file reads");
    assert_eq!(names, ["first card"]);
    let write = sqlx::query("INSERT INTO cards (name) VALUES ('second card')")
        .execute(foreign.reader())
        .await;
    assert!(
        write
            .as_ref()
            .is_err_and(|error| error.to_string().contains("readonly")),
        "a write through the read-only opener was not refused as read-only: {write:?}"
    );
    // Its journal mode is the one its own program chose.
    let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(foreign.reader())
        .await
        .expect("journal_mode");
    assert_eq!(journal, "delete");
    foreign.close().await;
}

#[tokio::test]
async fn a_lower_numbered_migration_added_after_a_higher_one_is_applied() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let migrations = directory.path().join("migrations");
    fs::create_dir(&migrations).expect("a migrations directory");
    let path = directory.path().join("deckstreak.db");
    let late = fixtures("late");
    let ship = |name: &str| {
        fs::copy(late.join(name), migrations.join(name)).expect("a fixture migration");
    };

    // One delivery ships the first and the third.
    ship("999901_kernel_late_first.sql");
    ship("999903_kernel_late_third.sql");
    let migrator = Migrator::new(migrations.as_path())
        .await
        .expect("a migrator");
    let db = Db::open_with(&path, &migrator)
        .await
        .expect("the first delivery's migrations apply");
    assert_eq!(tables(db.reader()).await, ["late_first", "late_third"]);
    db.close().await;

    // A sibling's delivery lands after it with the lower number.
    ship("999902_kernel_late_second.sql");
    let migrator = Migrator::new(migrations.as_path())
        .await
        .expect("a migrator");
    let db = Db::open_with(&path, &migrator)
        .await
        .expect("the late migration is applied, not refused");
    assert_eq!(
        tables(db.reader()).await,
        ["late_first", "late_second", "late_third"]
    );
    let applied: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(db.reader())
            .await
            .expect("the applied migrations");
    assert_eq!(applied, [999_901, 999_902, 999_903]);
}

#[tokio::test]
async fn the_settings_generation_moves_only_inside_a_write() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    let before = db.settings_generation().await.expect("the generation");

    let mut write = db.write().await.expect("a write");
    let bumped = Db::bump_settings_generation(&mut write)
        .await
        .expect("a bump");
    assert_eq!(bumped, before + 1);
    // Rolled back with its write, the bump never happened.
    drop(write);
    assert_eq!(
        db.settings_generation().await.expect("the generation"),
        before
    );

    let mut write = db.write().await.expect("a write");
    Db::bump_settings_generation(&mut write)
        .await
        .expect("a bump");
    Db::bump_settings_generation(&mut write)
        .await
        .expect("a second bump");
    write.commit().await.expect("committed");
    assert_eq!(
        db.settings_generation().await.expect("the generation"),
        before + 2
    );
}

#[tokio::test]
async fn a_closed_database_and_a_closed_foreign_file_refuse_a_read() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("gamify.db"))
        .await
        .expect("the database opens");
    let open: i64 = sqlx::query_scalar("SELECT 1")
        .fetch_one(db.reader())
        .await
        .expect("an open pool reads");
    assert_eq!(open, 1);
    db.close().await;
    assert!(db.reader().is_closed());

    let path = directory.path().join("collection.anki2");
    foreign_file(&path).await;
    let foreign = Db::open_foreign_read_only(&path)
        .await
        .expect("the foreign file opens");
    assert!(!foreign.reader().is_closed());
    foreign.close().await;
    assert!(foreign.reader().is_closed());
}
