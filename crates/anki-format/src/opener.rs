//! The defensive opener (SPEC-602 R2, R3, R4; ADR-622 D2, D3): the one file of this crate that
//! builds a connection, for every foreign file the crate reads.
//!
//! The source is never opened as a database. Its bytes are copied, under a bound, to a private
//! file in a directory the opener made; the copy is opened read-only with every defensive setting
//! on before any statement touches its schema, and it is refused before any row is read when a
//! check does not hold.

use std::cmp::Ordering;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _, Write};
use std::path::{Path, PathBuf};

use rusqlite::config::DbConfig;
use rusqlite::limits::Limit;
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest as _, Sha256};
use unicase::UniCase;

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

/// The collation schema-18 names carry, registered on every copy (ADR-623).
pub const COLLATION: &str = "unicase";

/// The order of [`COLLATION`]: the engine's, comparing names case-folded. Every connection that
/// reads a schema-18 file registers it, so `SQLite` can plan over the indexes that name it.
#[must_use]
pub fn unicase_order(a: &str, b: &str) -> Ordering {
    UniCase::new(a).cmp(&UniCase::new(b))
}

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

impl From<rusqlite::Error> for Refusal {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error.to_string())
    }
}

/// An opened foreign file: the connection over its private copy. Dropping it removes the copy
/// and the directory the opener made for it.
#[derive(Debug)]
pub struct Reader {
    connection: Connection,
    copy: PathBuf,
    dir: PathBuf,
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

impl Drop for Reader {
    fn drop(&mut self) {
        // The copy is private to this reader; nothing else holds the directory.
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// The sha256 of `bytes`, the form a caller records a file's digest in.
#[must_use]
pub fn digest(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0_u8; 32];
    out.copy_from_slice(&Sha256::digest(bytes));
    out
}

/// Opens `source` as a foreign file of `kind`: refuses a non-empty write-ahead log, copies the
/// bytes under the bound to a private file, checks the digest, opens the copy read-only with
/// every defensive setting and the collation, then refuses a planted object, a failed quick
/// check, a table set off the expected one, and for a collection a version or user version off
/// the snapshot's.
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
    refuse_a_wal(source)?;
    let dir = private_dir()?;
    let copy = dir.join("copy.sqlite");
    let opened = copy_and_open(source, &copy, recorded, kind, limits);
    match opened {
        Ok(connection) => Ok(Reader {
            connection,
            copy,
            dir,
        }),
        Err(refusal) => {
            let _ = fs::remove_dir_all(&dir);
            Err(refusal)
        }
    }
}

fn copy_and_open(
    source: &Path,
    copy: &Path,
    recorded: &[u8; 32],
    kind: Kind,
    limits: &Limits,
) -> Result<Connection, Refusal> {
    let copied = copy_bounded(source, copy, limits.max_bytes)?;
    if &copied != recorded {
        return Err(Refusal::Digest);
    }
    let connection = defensive_connection(copy, limits)?;
    refuse_objects(&connection)?;
    quick_check(&connection)?;
    let expected: &[&str] = match kind {
        Kind::Collection => &SNAPSHOT_TABLES,
        Kind::Media => &MEDIA_TABLES,
    };
    refuse_table_set(&connection, expected)?;
    if kind == Kind::Collection {
        refuse_versions(&connection)?;
    }
    Ok(connection)
}

/// (a) A non-empty `-wal` beside the source is refused before any byte is copied.
fn refuse_a_wal(source: &Path) -> Result<(), Refusal> {
    let mut wal = source.as_os_str().to_owned();
    wal.push("-wal");
    match fs::metadata(PathBuf::from(wal)) {
        Ok(meta) if meta.len() > 0 => Err(Refusal::Wal),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Refusal::Io(error.kind().to_string())),
    }
}

/// How many names `private_dir` tries before it refuses: one per reader this process holds at
/// once, and more for a folder a past process of the same id left behind.
const PRIVATE_DIR_NAMES: u32 = 256;

/// The directory the copy lives in, made by the opener, readable by its owner alone. The names
/// are counted from 0 on every call, so the opener keeps no state between calls: a name another
/// reader holds is taken by `mkdir` alone, and the next one is tried.
fn private_dir() -> Result<PathBuf, Refusal> {
    let base = std::env::temp_dir();
    let process = std::process::id();
    for number in 0..PRIVATE_DIR_NAMES {
        let dir = base.join(format!("deck-streak-anki-format-{process}-{number}"));
        match make_dir(&dir).map_err(|error| error.kind()) {
            Ok(()) => return Ok(dir),
            Err(io::ErrorKind::AlreadyExists) => {}
            Err(kind) => return Err(Refusal::Io(kind.to_string())),
        }
    }
    Err(Refusal::Io("no fresh private directory".to_owned()))
}

#[cfg(not(target_family = "wasm"))]
fn make_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    fs::DirBuilder::new().mode(0o700).create(dir)
}

#[cfg(target_family = "wasm")]
fn make_dir(dir: &Path) -> io::Result<()> {
    fs::DirBuilder::new().create(dir)
}

#[cfg(not(target_family = "wasm"))]
fn create_private(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

#[cfg(target_family = "wasm")]
fn create_private(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// A writer that hashes and counts what passes through it on its way to the copy.
struct Hashing {
    file: File,
    hasher: Sha256,
}

impl Write for Hashing {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.file.write(buf)?;
        self.hasher.update(&buf[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// (b) The source's bytes, counted against `max_bytes`, into the private copy; its digest.
fn copy_bounded(source: &Path, copy: &Path, max_bytes: u64) -> Result<[u8; 32], Refusal> {
    let io_refusal = |error: io::Error| Refusal::Io(error.kind().to_string());
    let input = File::open(source).map_err(io_refusal)?;
    let mut output = Hashing {
        file: create_private(copy).map_err(io_refusal)?,
        hasher: Sha256::new(),
    };
    let copied =
        io::copy(&mut input.take(max_bytes.saturating_add(1)), &mut output).map_err(io_refusal)?;
    if copied > max_bytes {
        return Err(Refusal::OverBound);
    }
    output.flush().map_err(io_refusal)?;
    output.file.sync_all().map_err(io_refusal)?;
    let mut out = [0_u8; 32];
    out.copy_from_slice(&output.hasher.finalize());
    Ok(out)
}

/// (c) and (d): the copy opened read-only, every defensive setting on and the limits set before
/// any statement touches the schema, then the collation registered.
fn defensive_connection(copy: &Path, limits: &Limits) -> Result<Connection, Refusal> {
    let connection = Connection::open_with_flags(
        copy,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
    connection.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)?;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, limits.max_length)?;
    connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, limits.max_sql_length)?;
    connection.create_collation(COLLATION, unicase_order)?;
    connection.execute_batch(
        "PRAGMA query_only = ON; PRAGMA cell_size_check = ON; PRAGMA mmap_size = 0;",
    )?;
    Ok(connection)
}

/// A view, a trigger or a virtual table in the schema is refused by its kind and name.
fn refuse_objects(connection: &Connection) -> Result<(), Refusal> {
    let mut statement = connection.prepare("SELECT type, name, sql FROM sqlite_schema")?;
    let objects = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (kind, name, sql) in objects {
        let planted = match kind.as_str() {
            "view" => Some("view"),
            "trigger" => Some("trigger"),
            "table" if is_virtual(&sql) => Some("virtual table"),
            _ => None,
        };
        if let Some(kind) = planted {
            return Err(Refusal::Object { kind, name });
        }
    }
    Ok(())
}

fn is_virtual(sql: &str) -> bool {
    let words: Vec<String> = sql
        .split_whitespace()
        .take(3)
        .map(str::to_ascii_lowercase)
        .collect();
    words.len() == 3 && words[0] == "create" && words[1] == "virtual" && words[2] == "table"
}

/// (e) `PRAGMA quick_check` must answer `ok` alone; a corrupt copy that errors is refused here too.
fn quick_check(connection: &Connection) -> Result<(), Refusal> {
    let answers = connection
        .prepare("PRAGMA quick_check")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| Refusal::QuickCheck)?;
    if answers == ["ok"] {
        Ok(())
    } else {
        Err(Refusal::QuickCheck)
    }
}

/// The tables, `sqlite_` tables excluded, must be exactly `expected`; the first table outside the
/// set, else the first one missing, is named.
fn refuse_table_set(connection: &Connection, expected: &[&str]) -> Result<(), Refusal> {
    let mut statement = connection.prepare(
        "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' \
         ORDER BY name",
    )?;
    let found = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(extra) = found.iter().find(|name| !expected.contains(&name.as_str())) {
        return Err(Refusal::TableSet {
            table: extra.clone(),
        });
    }
    if let Some(missing) = expected
        .iter()
        .find(|name| !found.iter().any(|f| f == *name))
    {
        return Err(Refusal::TableSet {
            table: (*missing).to_owned(),
        });
    }
    Ok(())
}

/// A collection's one `col` row must read version 18, and its user version the pin.
fn refuse_versions(connection: &Connection) -> Result<(), Refusal> {
    let mut statement = connection.prepare("SELECT ver FROM col")?;
    let versions = statement
        .query_map([], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if versions != [COLLECTION_VERSION] {
        return Err(Refusal::Version);
    }
    let user_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if user_version != USER_VERSION {
        return Err(Refusal::UserVersion);
    }
    Ok(())
}
