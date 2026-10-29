//! The one opener (SPEC-512 R1, R2, R8; ADR-530 D2 to D4): the only file of the workspace that
//! builds a `SQLite` connection.
//!
//! An open reads the file's `application_id` and `user_version` before it changes anything: from
//! the file's first 100 bytes on a native target, before any connection exists, then again through
//! `SQLite`, which also sees a committed write the log has not yet moved into the file. A file it
//! refuses is left byte for byte as it was.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, TransactionBehavior};

use crate::error::StoreError;
use crate::migrate::{self, HEAD};
use crate::sql;

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
        if matches!(slot, 1..=63) {
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

    /// The `synchronous` level the role runs at: `FULL` for the server, which answers only after
    /// its commit returns, and `NORMAL` for a device, which keeps an operation pending until the
    /// server has judged it.
    const fn synchronous(&self) -> &'static str {
        match self {
            Self::Server { .. } => sql::SET_SYNCHRONOUS_FULL,
            Self::Device { .. } => sql::SET_SYNCHRONOUS_NORMAL,
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

/// What a file's header says: its `application_id` and `user_version`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Header {
    application_id: i32,
    user_version: i64,
}

/// The header of the file at `path`, read from its first 100 bytes before any connection exists,
/// or none when the path holds no file or an empty one. A file that is not `SQLite`'s is refused.
#[cfg(not(target_family = "wasm"))]
fn file_header(path: &Path) -> Result<Option<Header>, StoreError> {
    use std::io::Read;
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(StoreError::File(error)),
    };
    let mut bytes = Vec::with_capacity(100);
    file.by_ref()
        .take(100)
        .read_to_end(&mut bytes)
        .map_err(StoreError::File)?;
    if bytes.is_empty() {
        return Ok(None);
    }
    let field = |at: usize| -> Option<i32> {
        let word: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
        Some(i32::from_be_bytes(word))
    };
    match (
        bytes.starts_with(b"SQLite format 3\0"),
        field(60),
        field(68),
    ) {
        (true, Some(user_version), Some(application_id)) => Ok(Some(Header {
            application_id,
            user_version: i64::from(user_version),
        })),
        _ => Err(StoreError::ForeignFile(0)),
    }
}

/// On `wasm32` the file lives behind the web engine's file system, so the header is read through
/// `SQLite` alone.
#[cfg(target_family = "wasm")]
#[allow(clippy::unnecessary_wraps)]
const fn file_header(_path: &Path) -> Result<Option<Header>, StoreError> {
    Ok(None)
}

/// The header as `SQLite` reads it, with how many objects the schema holds.
fn sqlite_header(conn: &Connection) -> Result<(Header, i64), StoreError> {
    let application_id = conn.query_row(sql::READ_APPLICATION_ID, [], |row| row.get(0))?;
    let user_version = conn.query_row(sql::READ_USER_VERSION, [], |row| row.get(0))?;
    let objects = conn.query_row(sql::READ_SCHEMA_SIZE, [], |row| row.get(0))?;
    Ok((
        Header {
            application_id,
            user_version,
        },
        objects,
    ))
}

/// Whether a writer may open a file of `header`: a store at or below head. A header with no
/// `application_id` is judged again once `SQLite` has counted its schema.
const fn admit_writer(header: Header) -> Result<(), StoreError> {
    match header.application_id {
        APPLICATION_ID if header.user_version > HEAD => Err(StoreError::NewerSchema {
            found: header.user_version,
            newest: HEAD,
        }),
        APPLICATION_ID | 0 => Ok(()),
        other => Err(StoreError::ForeignFile(other)),
    }
}

/// Whether a reader may open a file of `header`: a store, never above head.
const fn admit_reader(header: Header) -> Result<(), StoreError> {
    match header.application_id {
        APPLICATION_ID if header.user_version > HEAD => Err(StoreError::NewerSchema {
            found: header.user_version,
            newest: HEAD,
        }),
        APPLICATION_ID => Ok(()),
        other => Err(StoreError::ForeignFile(other)),
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
        if let Some(header) = file_header(path)? {
            admit_writer(header)?;
        }
        let mut conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))?;
        let (header, objects) = sqlite_header(&conn)?;
        admit_writer(header)?;
        let empty = header.application_id == 0 && header.user_version == 0 && objects == 0;
        if header.application_id == 0 && !empty {
            return Err(StoreError::ForeignFile(0));
        }
        conn.execute_batch(sql::SET_FOREIGN_KEYS)?;
        conn.execute_batch(sql::SET_SECURE_DELETE)?;
        #[cfg(not(target_family = "wasm"))]
        conn.execute_batch(sql::SET_JOURNAL_WAL)?;
        conn.execute_batch(role.synchronous())?;
        migrate::apply(&mut conn)?;
        record_role(&mut conn, role)?;
        Ok(Self {
            conn,
            path: path.to_path_buf(),
            role: Some(role),
        })
    }

    /// Opens the store at `path` to read: read-only and `query_only`. It never makes a file.
    ///
    /// # Errors
    ///
    /// As [`Store::open`], and [`StoreError::NotAtHead`] for a store below head.
    pub fn open_reader(path: &Path) -> Result<Self, StoreError> {
        if let Some(header) = file_header(path)? {
            admit_reader(header)?;
        }
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))?;
        let (header, _) = sqlite_header(&conn)?;
        admit_reader(header)?;
        if header.user_version < HEAD {
            return Err(StoreError::NotAtHead {
                found: header.user_version,
                newest: HEAD,
            });
        }
        conn.execute_batch(sql::SET_QUERY_ONLY)?;
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

    /// The connection and the path, given up by a device's erase.
    pub(crate) fn into_parts(self) -> (Connection, PathBuf) {
        (self.conn, self.path)
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
        if self.role.is_none() {
            return Err(StoreError::ReadOnly);
        }
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
            application_id: conn.query_row(sql::READ_APPLICATION_ID, [], |row| row.get(0))?,
            user_version: conn.query_row(sql::READ_USER_VERSION, [], |row| row.get(0))?,
            journal_mode: conn.query_row(sql::READ_JOURNAL_MODE, [], |row| row.get(0))?,
            synchronous: conn.query_row(sql::READ_SYNCHRONOUS, [], |row| row.get(0))?,
            foreign_keys: conn.query_row(sql::READ_FOREIGN_KEYS, [], |row| row.get(0))?,
            secure_delete: conn.query_row(sql::READ_SECURE_DELETE, [], |row| row.get(0))?,
            query_only: conn.query_row(sql::READ_QUERY_ONLY, [], |row| row.get(0))?,
            busy_timeout_ms: conn.query_row(sql::READ_BUSY_TIMEOUT, [], |row| row.get(0))?,
        })
    }

    /// What `integrity_check` and `foreign_key_check` read on the store.
    ///
    /// # Errors
    ///
    /// [`StoreError::Sqlite`] when either check cannot run.
    pub fn health(&self) -> Result<Health, StoreError> {
        let mut integrity = self.conn.prepare_cached(sql::INTEGRITY_CHECK)?;
        let integrity = integrity
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut keys = self.conn.prepare_cached(sql::FOREIGN_KEY_CHECK)?;
        let foreign_key_violations = keys
            .query_map([], |_| Ok(()))?
            .collect::<Result<Vec<()>, _>>()?
            .len();
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

/// Writes the store's record once, in its own `BEGIN IMMEDIATE` transaction, and refuses a store
/// whose record names another role, writer id or slot.
fn record_role(conn: &mut Connection, role: Role) -> Result<(), StoreError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let origin = role.origin().bytes();
    let slot = i64::from(role.slot().get());
    tx.execute(sql::INSERT_STORE_RECORD, (role.code(), &origin[..], slot))?;
    let recorded: (i64, Vec<u8>, i64) = tx.query_row(sql::READ_STORE_RECORD, [], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    if recorded != (role.code(), origin.to_vec(), slot) {
        return Err(StoreError::RoleMismatch);
    }
    tx.commit()?;
    Ok(())
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
        if file_header(path)?.is_some_and(|header| header.application_id == APPLICATION_ID) {
            return Err(StoreError::StoreFile);
        }
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.busy_timeout(Duration::from_millis(BUSY_TIMEOUT_MS))?;
        let (header, _) = sqlite_header(&conn)?;
        if header.application_id == APPLICATION_ID {
            return Err(StoreError::StoreFile);
        }
        conn.execute_batch(sql::SET_QUERY_ONLY)?;
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
