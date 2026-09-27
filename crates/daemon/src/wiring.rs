//! Wiring the contexts' adapters for the roles (SPEC-025).
//!
//! STUB for the red-first commit: the database is opened with no lock around its migrations.

use std::io;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Db, Environment, KernelError, Offload, Setting, SettingsError};

/// The directory systemd gives a unit for its state (`StateDirectory=`).
pub const STATE_DIRECTORY: &str = "STATE_DIRECTORY";
/// The database's file name in the state directory.
pub const DATABASE_FILE: &str = "deck_streak.db";
/// The lock file a role holds while it opens and migrates the database.
pub const OPEN_LOCK_FILE: &str = "deck_streak.db-open.lock";

/// The directory the database lives in.
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
    /// A missing or malformed setting.
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
    /// The open lock could not be taken or released.
    #[error("the database's open lock could not be taken or released")]
    OpenLock(#[source] io::Error),
    /// The kernel refused to open or migrate the database.
    #[error(transparent)]
    Kernel(#[from] KernelError),
}

/// Opens the database in `state`.
///
/// # Errors
///
/// [`WiringError::Kernel`] when the database cannot be opened or migrated.
pub async fn open_database(offload: &Offload, state: &StateDirectory) -> Result<Db, WiringError> {
    let _ = offload;
    Ok(Db::open(&state.database()).await?)
}
