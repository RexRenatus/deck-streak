//! Wiring the contexts' adapters for the roles: here, the one database every role opens (SPEC-025
//! R11; ADR-008, ADR-025).
//!
//! The database is `deck_streak.db` in the directory systemd gives the units (`StateDirectory=`,
//! passed as `$STATE_DIRECTORY`). Every role opens it through [`open_database`], because sqlx's
//! `SQLite` migrator takes no lock: two roles starting at once could both find a migration
//! unapplied and both apply it, or collide switching a fresh file to WAL, and one of them would
//! fail to start. [`open_database`] holds an exclusive lock on a file beside the database while
//! the kernel's `Db::open` sets the pragmas and applies the migrations, so a second role waits and
//! then finds every migration applied. The race is closed, not made rarer
//! (`docs/schematics/service-lifecycle.md`).

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Db, Environment, KernelError, Offload, Setting, SettingsError};

/// The directory systemd gives a unit for its state (`StateDirectory=`), where the database lives.
pub const STATE_DIRECTORY: &str = "STATE_DIRECTORY";
/// The database's file name in the state directory (ADR-008; the deployment schematic).
pub const DATABASE_FILE: &str = "deck_streak.db";
/// The lock file a role holds while it opens and migrates the database: a file of its own beside
/// the database, never the database itself, because closing any descriptor of the database file
/// drops every POSIX lock the process holds on it, `SQLite`'s included.
pub const OPEN_LOCK_FILE: &str = "deck_streak.db-open.lock";

/// The directory the database lives in: an absolute path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDirectory(PathBuf);

impl StateDirectory {
    /// The directory at `path`, or `None` when the path is not absolute.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Option<Self> {
        let path = path.into();
        path.is_absolute().then_some(Self(path))
    }

    /// The directory [`STATE_DIRECTORY`] names.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Missing`] when it is unset or blank, and [`SettingsError::Malformed`] when
    /// it is not an absolute path.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        env.required(STATE_DIRECTORY)
    }

    /// The directory's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// The database's path.
    #[must_use]
    pub fn database(&self) -> PathBuf {
        self.0.join(DATABASE_FILE)
    }
}

impl Setting for StateDirectory {
    const SHAPE: &'static str = "an absolute directory path";

    fn parse(text: &str) -> Option<Self> {
        Self::new(text)
    }
}

/// Why the database could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum WiringError {
    /// The open lock could not be taken or released; the source says why.
    #[error("the database's open lock could not be taken or released")]
    OpenLock(#[source] io::Error),
    /// The kernel refused to open or migrate the database; the source says why.
    #[error(transparent)]
    Kernel(#[from] KernelError),
}

/// Opens the database in `state` and applies every migration, holding the open lock throughout,
/// so no two roles ever migrate the same file at once. The wait for the lock runs on `offload`, off
/// the async runtime, and a wait of a second or more is logged as a slow operation.
///
/// # Errors
///
/// [`WiringError::OpenLock`] when the lock file cannot be opened, locked or unlocked, and
/// [`WiringError::Kernel`] when the database cannot be opened or migrated.
pub async fn open_database(offload: &Offload, state: &StateDirectory) -> Result<Db, WiringError> {
    let lock_path = state.path().join(OPEN_LOCK_FILE);
    let lock = offload
        .run("take the database's open lock", move || {
            take_open_lock(&lock_path)
        })
        .await?
        .map_err(WiringError::OpenLock)?;
    let opened = Db::open(&state.database()).await;
    // Released explicitly, never only by closing: a descriptor a child process inherited would
    // keep a lock that only a close releases held.
    let released = lock.unlock();
    drop(lock);
    let database = opened?;
    released.map_err(WiringError::OpenLock)?;
    Ok(database)
}

/// Opens (creating it if missing) the lock file at `path` and waits for its exclusive lock.
fn take_open_lock(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.lock()?;
    Ok(file)
}
