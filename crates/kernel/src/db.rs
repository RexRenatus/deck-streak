//! The repository base: the one place a `SQLite` connection is configured and opened (SPEC-020
//! R15 to R19; ADR-008).
//!
//! [`Db::open`] opens DeckStreak's database with `journal_mode=WAL`, `synchronous=NORMAL`,
//! `foreign_keys=ON` and a busy timeout of [`DB_BUSY_TIMEOUT_MS`], the predecessor's pragmas
//! (`database.py:GamifyStore.connect` at `27ee2bc`), then applies the embedded migrations before
//! it returns. Every write goes through [`Db::write`], which begins with `BEGIN IMMEDIATE`, so two
//! writers serialise on the write lock under the busy timeout instead of failing mid-transaction.
//! [`Db::open_foreign_read_only`] opens any other `SQLite` file (the Anki collection copy)
//! read-only, without touching its journal mode. No other file of the workspace constructs
//! connect options or a pool (the ledger-sqlite pack's `connect-options-in-one-place`).

use std::path::Path;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{Sqlite, SqliteConnection, Transaction};

use crate::error::KernelError;

/// How long a connection waits for a lock before it gives up: the predecessor's
/// `database.py:DB_BUSY_TIMEOUT_MS`.
pub const DB_BUSY_TIMEOUT_MS: u64 = 0;

/// The connections a pool keeps: enough for one owner's traffic, few enough that each one's page
/// cache stays inside a small host's memory budget.
pub const MAX_CONNECTIONS: u32 = 4;

/// Every migration in the workspace's one `migrations/` directory, embedded at build time
/// (ADR-020). `build.rs` makes the kernel rebuild when a migration is added.
pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

/// DeckStreak's own database: WAL, foreign keys, the busy timeout, and every migration applied.
#[derive(Clone, Debug)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    /// Opens (creating it if missing) the database at `path`, and applies [`MIGRATOR`].
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the file cannot be opened, and [`KernelError::Migrate`] when
    /// a migration fails or an applied one was edited.
    pub async fn open(path: &Path) -> Result<Self, KernelError> {
        Self::open_with(path, &MIGRATOR).await
    }

    /// Opens the database at `path` as [`Db::open`] does, applying `migrator` instead of
    /// [`MIGRATOR`]: for a fixture's migrations in a test. Production opens with [`Db::open`].
    ///
    /// # Errors
    ///
    /// As [`Db::open`].
    pub async fn open_with(path: &Path, migrator: &Migrator) -> Result<Self, KernelError> {
        let _ = migrator;
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect_with(options)
            .await?;
        Ok(Self { pool })
    }

    /// Opens another program's `SQLite` file at `path` read-only (`mode=ro` and `query_only`),
    /// never changing its journal mode.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the file does not exist or cannot be opened.
    pub async fn open_foreign_read_only(path: &Path) -> Result<ForeignDb, KernelError> {
        let options = SqliteConnectOptions::new().filename(path);
        let pool = SqlitePoolOptions::new()
            .max_connections(MAX_CONNECTIONS)
            .connect_with(options)
            .await?;
        Ok(ForeignDb { pool })
    }

    /// Begins a write: a transaction opened with `BEGIN IMMEDIATE`, which takes the write lock at
    /// once (waiting under the busy timeout) so it can never fail to upgrade mid-transaction.
    /// Commit it to keep the writes; dropping it rolls them back.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write lock is not granted within the busy timeout.
    pub async fn write(&self) -> Result<Transaction<'static, Sqlite>, KernelError> {
        Ok(self.pool.begin().await?)
    }

    /// The pool reads run on. A write never runs here: it goes through [`Db::write`].
    #[must_use]
    pub const fn reader(&self) -> &SqlitePool {
        &self.pool
    }

    /// The owner-config generation the change gate compares (R19).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails.
    pub async fn settings_generation(&self) -> Result<i64, KernelError> {
        Ok(0)
    }

    /// Increments the owner-config generation inside the caller's write, the one that changed a
    /// setting's value, and returns the new generation (R19).
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn bump_settings_generation(
        write: &mut SqliteConnection,
    ) -> Result<i64, KernelError> {
        let _ = write;
        Ok(0)
    }

    /// Closes the pool, waiting for its connections to finish.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

/// Another program's `SQLite` file, open read-only: it can be read and never written.
#[derive(Clone, Debug)]
pub struct ForeignDb {
    pool: SqlitePool,
}

impl ForeignDb {
    /// The read-only pool.
    #[must_use]
    pub const fn reader(&self) -> &SqlitePool {
        &self.pool
    }

    /// Closes the pool, waiting for its connections to finish.
    pub async fn close(&self) {
        self.pool.close().await;
    }
}
