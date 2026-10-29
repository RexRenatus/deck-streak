//! The one opener (SPEC-512 R1, R2, R8; ADR-530 D2 to D4): the only file of the workspace that
//! builds a `SQLite` connection.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, TransactionBehavior};

use crate::error::StoreError;

/// The store's `application_id`, `0x44535443`: "DSTC" in ASCII. It is stamped by the first
/// migration and read before anything else at every open.
pub const APPLICATION_ID: i32 = 0x4453_5443;

/// How long a connection waits for a lock before it gives up: the kernel's own
/// `DB_BUSY_TIMEOUT_MS`.
pub const BUSY_TIMEOUT_MS: u64 = 30_000;

/// A store's writer id: 16 bytes, fixed when the store is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Origin([u8; 16]);

impl Origin {
    /// The writer id of these 16 bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// The writer id's 16 bytes.
    #[must_use]
    pub const fn bytes(&self) -> [u8; 16] {
        self.0
    }
}

/// A store's id slot: 0 for the server, 1 to 63 for an enrolled device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slot(u8);

impl Slot {
    /// The server's slot.
    pub const SERVER: Self = Self(0);

    /// A device's slot, 1 to 63, or none outside that range.
    #[must_use]
    pub const fn device(slot: u8) -> Option<Self> {
        if slot >= 1 && slot <= 63 {
            Some(Self(slot))
        } else {
            None
        }
    }

    /// The slot's number.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Whose store a file is: the server's, or one enrolled device's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The server's store, in slot 0.
    Server {
        /// The server's writer id.
        origin: Origin,
    },
    /// A device's store.
    Device {
        /// The device's slot, 1 to 63.
        slot: Slot,
        /// The device's writer id.
        origin: Origin,
    },
}

impl Role {
    /// The role as `store_meta.role` records it: 0 the server, 1 a device.
    #[must_use]
    pub const fn code(&self) -> i64 {
        match self {
            Self::Server { .. } => 0,
            Self::Device { .. } => 1,
        }
    }

    /// The role's slot.
    #[must_use]
    pub const fn slot(&self) -> Slot {
        match self {
            Self::Server { .. } => Slot::SERVER,
            Self::Device { slot, .. } => *slot,
        }
    }

    /// The role's writer id.
    #[must_use]
    pub const fn origin(&self) -> Origin {
        match self {
            Self::Server { origin } | Self::Device { origin, .. } => *origin,
        }
    }
}

/// The settings an open store's connection runs with, read back from `SQLite`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenSettings {
    /// The file's `application_id`.
    pub application_id: i32,
    /// The file's `user_version`: the newest migration applied.
    pub user_version: i64,
    /// The journal mode, as `SQLite` names it.
    pub journal_mode: String,
    /// `synchronous`: 1 `NORMAL`, 2 `FULL`.
    pub synchronous: i64,
    /// Whether foreign keys are enforced.
    pub foreign_keys: bool,
    /// Whether a deleted row's bytes are overwritten.
    pub secure_delete: bool,
    /// Whether the connection refuses every write.
    pub query_only: bool,
    /// The busy timeout, in milliseconds.
    pub busy_timeout_ms: i64,
}

/// What `integrity_check` and `foreign_key_check` read on the store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Health {
    /// `integrity_check`'s lines: `["ok"]` for a sound file.
    pub integrity: Vec<String>,
    /// How many rows `foreign_key_check` reports.
    pub foreign_key_violations: usize,
}

/// One user's collection store.
#[derive(Debug)]
pub struct Store {
    conn: Connection,
    path: PathBuf,
    role: Option<Role>,
}

/// A read of the store, inside one deferred transaction.
#[derive(Debug)]
pub struct Snapshot<'a> {
    conn: &'a Connection,
}

/// A write of the store, inside one `BEGIN IMMEDIATE` transaction.
#[derive(Debug)]
pub struct Tx<'a> {
    tx: rusqlite::Transaction<'a>,
}

impl Snapshot<'_> {
    /// The connection this read runs on.
    pub(crate) const fn conn(&self) -> &Connection {
        self.conn
    }
}

impl Tx<'_> {
    /// The transaction this write runs in.
    pub(crate) fn conn(&self) -> &Connection {
        &self.tx
    }
}

impl Store {
    /// Opens the store at `path` in `role`, creating it when the path holds nothing, and migrates it
    /// to head.
    ///
    /// # Errors
    ///
    /// [`StoreError::ForeignFile`] for a file of another application, [`StoreError::NewerSchema`]
    /// for a store a newer app wrote, [`StoreError::RoleMismatch`] for a store made in another role,
    /// and [`StoreError::Sqlite`] when `SQLite` fails.
    pub fn open(path: &Path, role: Role) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        Ok(Self {
            conn,
            path: path.to_path_buf(),
            role: Some(role),
        })
    }

    /// Opens the store at `path` to read: read-only and `query_only`.
    ///
    /// # Errors
    ///
    /// As [`Store::open`], and [`StoreError::NotAtHead`] for a store below head.
    pub fn open_reader(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        Ok(Self {
            conn,
            path: path.to_path_buf(),
            role: None,
        })
    }

    /// The store's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The role the store was opened in, or none for a reader.
    #[must_use]
    pub const fn role(&self) -> Option<Role> {
        self.role
    }

    /// The connection, for the crate's own ports.
    pub(crate) const fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Runs `read` inside one deferred transaction, so every statement it runs sees one state.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when the transaction cannot begin, and whatever `read` returns.
    pub fn read<T>(
        &self,
        read: impl FnOnce(&Snapshot<'_>) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        let answer = read(&Snapshot { conn: &tx })?;
        tx.commit()?;
        Ok(answer)
    }

    /// Runs `write` inside one `BEGIN IMMEDIATE` transaction, committed when it returns `Ok`.
    ///
    /// # Errors
    ///
    /// [`StoreError::ReadOnly`] for a reader, [`StoreError::Sqlite`] when the transaction cannot
    /// begin or commit, and whatever `write` returns.
    pub fn write<T>(
        &mut self,
        write: impl FnOnce(&mut Tx<'_>) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut tx = Tx { tx };
        let answer = write(&mut tx)?;
        tx.tx.commit()?;
        Ok(answer)
    }

    /// The settings the store's connection runs with.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when a pragma cannot be read.
    pub fn settings(&self) -> Result<OpenSettings, StoreError> {
        let conn = &self.conn;
        Ok(OpenSettings {
            application_id: conn
                .query_row(crate::sql::READ_APPLICATION_ID, [], |row| row.get(0))?,
            user_version: conn.query_row(crate::sql::READ_USER_VERSION, [], |row| row.get(0))?,
            journal_mode: conn.query_row(crate::sql::READ_JOURNAL_MODE, [], |row| row.get(0))?,
            synchronous: conn.query_row(crate::sql::READ_SYNCHRONOUS, [], |row| row.get(0))?,
            foreign_keys: conn.query_row(crate::sql::READ_FOREIGN_KEYS, [], |row| row.get(0))?,
            secure_delete: conn.query_row(crate::sql::READ_SECURE_DELETE, [], |row| row.get(0))?,
            query_only: conn.query_row(crate::sql::READ_QUERY_ONLY, [], |row| row.get(0))?,
            busy_timeout_ms: conn.query_row(crate::sql::READ_BUSY_TIMEOUT, [], |row| row.get(0))?,
        })
    }

    /// What `integrity_check` and `foreign_key_check` read on the store.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when either check cannot run.
    pub fn health(&self) -> Result<Health, StoreError> {
        let mut integrity = self.conn.prepare_cached(crate::sql::INTEGRITY_CHECK)?;
        let integrity = integrity
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut keys = self.conn.prepare_cached(crate::sql::FOREIGN_KEY_CHECK)?;
        let foreign_key_violations = keys.query_map([], |_| Ok(()))?.count();
        Ok(Health {
            integrity,
            foreign_key_violations,
        })
    }

    /// Prepares `statement` against the store without running it: the statement census's check.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when `SQLite` cannot prepare it.
    pub fn check_statement(&self, statement: &str) -> Result<(), StoreError> {
        self.conn
            .prepare(statement)
            .map(drop)
            .map_err(StoreError::from)
    }
}

/// Another application's `SQLite` file, open read-only and `query_only`: the one way the
/// workspace's adapters open a file that is not a store.
#[derive(Debug)]
pub struct ForeignFile {
    conn: Connection,
}

impl ForeignFile {
    /// Opens the file at `path` read-only and `query_only`, refusing a collection store.
    ///
    /// # Errors
    ///
    /// [`StoreError::StoreFile`] for a collection store, and [`StoreError::Sqlite`] when the file
    /// cannot be opened.
    pub fn open_read_only(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    /// The read-only connection: `SQLite` refuses every write through it.
    #[must_use]
    pub const fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Prepares `statement` against the file without running it: the statement census's check.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when `SQLite` cannot prepare it.
    pub fn check_statement(&self, statement: &str) -> Result<(), StoreError> {
        self.conn
            .prepare(statement)
            .map(drop)
            .map_err(StoreError::from)
    }
}
