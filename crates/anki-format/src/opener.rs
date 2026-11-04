//! The defensive opener (SPEC-602 R2, R3, R4; ADR-622 D2, D3): the one file of this crate that
//! builds a connection, for every foreign file the crate reads.
//!
//! Red-first stand-in: this opener opens the source in place, with no setting and no refusal.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use sha2::{Digest as _, Sha256};

/// The twelve collection tables of the schema snapshot, `sqlite_` tables excluded.
pub const SNAPSHOT_TABLES: [&str; 12] = [
    "cards",
    "col",
    "config",
    "deck_config",
    "decks",
    "fields",
    "graves",
    "notes",
    "notetypes",
    "revlog",
    "tags",
    "templates",
];

/// The media database's two tables.
pub const MEDIA_TABLES: [&str; 2] = ["media", "meta"];

/// The collection version the snapshot pins.
pub const COLLECTION_VERSION: i64 = 18;

/// The `PRAGMA user_version` the snapshot pins.
pub const USER_VERSION: i64 = 0;

/// Which foreign file is opened, and so which table set it must hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A schema-18 collection: the snapshot's twelve tables, `col.ver` 18, the pinned user version.
    Collection,
    /// The media database: `media` and `meta`.
    Media,
}

/// The bounds a foreign file is held to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes the private copy may hold; a longer source is refused while it is copied.
    pub max_bytes: u64,
    /// `SQLITE_LIMIT_LENGTH` on the opened copy.
    pub max_length: i32,
    /// `SQLITE_LIMIT_SQL_LENGTH` on the opened copy.
    pub max_sql_length: i32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 1 << 30,
            max_length: 1 << 26,
            max_sql_length: 1 << 20,
        }
    }
}

/// Why a foreign file is not read. A refusal names the check and the object kind or table,
/// never a value of the file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// A non-empty write-ahead log sits beside the source.
    #[error("refused before the copy: a non-empty write-ahead log sits beside the source")]
    Wal,
    /// The source holds more bytes than its bound.
    #[error("refused while copying: the source is over its bound")]
    OverBound,
    /// The copy's sha256 differs from the caller's recorded one.
    #[error("refused: the copy's digest differs from the recorded one")]
    Digest,
    /// `PRAGMA quick_check` answered anything but `ok`.
    #[error("refused: the quick check of the copy did not read ok")]
    QuickCheck,
    /// A view, a trigger or a virtual table sits in the schema.
    #[error("refused: a planted {kind} ({name})")]
    Object {
        /// `view`, `trigger` or `virtual table`.
        kind: &'static str,
        /// The object's name in the schema.
        name: String,
    },
    /// The table set differs from the expected one at this table.
    #[error("refused: the table set differs from the snapshot at table {table}")]
    TableSet {
        /// The table missing or outside the set.
        table: String,
    },
    /// `col.ver` is not the snapshot's.
    #[error("refused: the collection version is not the snapshot's")]
    Version,
    /// `PRAGMA user_version` is not the snapshot's pin.
    #[error("refused: the user version is not the snapshot's pin")]
    UserVersion,
    /// The private copy could not be made.
    #[error("refused: the private copy could not be made ({0})")]
    Io(String),
    /// The copy could not be read as SQLite.
    #[error("refused: the copy could not be read ({0})")]
    Sql(String),
}

/// An opened foreign file: the connection over its private copy.
#[derive(Debug)]
pub struct Reader {
    connection: Connection,
    copy: PathBuf,
}

impl Reader {
    /// The read-only connection over the private copy.
    #[must_use]
    pub const fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The private copy's path.
    #[must_use]
    pub fn copy_path(&self) -> &Path {
        &self.copy
    }
}

/// The sha256 of `bytes`, the form a caller records a file's digest in.
#[must_use]
pub fn digest(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0_u8; 32];
    out.copy_from_slice(&Sha256::digest(bytes));
    out
}

/// Opens `source` as a foreign file of `kind`.
///
/// # Errors
///
/// A [`Refusal`] naming the check that refused the file.
pub fn open(
    source: &Path,
    recorded: &[u8; 32],
    kind: Kind,
    limits: &Limits,
) -> Result<Reader, Refusal> {
    let _ = (recorded, kind, limits);
    let connection = Connection::open(source).map_err(|error| Refusal::Sql(error.to_string()))?;
    Ok(Reader {
        connection,
        copy: source.to_path_buf(),
    })
}
