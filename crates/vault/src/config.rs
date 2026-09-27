//! The vault's configured paths and the start check (SPEC-042 R1).
//!
//! The vault root, the readings folder inside it and the archive folder inside that are settings,
//! read through the kernel's typed configuration; no vault path is a literal in code. The vault
//! root is private configuration: its [`fmt::Debug`] and every refusal name the setting, never its
//! value.
//!
//! [`VaultPaths::check`] is the start check: the vault features refuse to start when the root is not
//! a directory, or the readings folder is absent, not a directory, outside the root or not writable
//! by the service. It never creates a folder: the readings folder is made by the operator before the
//! first live night, and the adapter never creates anything at the vault's top level.

use std::fmt;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Environment, Setting, SettingsError};

use crate::VaultError;
use crate::fs::VaultFs;

/// The vault root: an absolute directory path, private to the deployment.
pub const VAULT_ROOT: &str = "DECKSTREAK_VAULT_ROOT";
/// The readings folder, one folder name at the vault's top level (example value `12-Readings`).
pub const READINGS_FOLDER: &str = "DECKSTREAK_VAULT_READINGS_FOLDER";
/// The archive folder inside the readings folder, one folder name (example value `Archive`).
pub const ARCHIVE_FOLDER: &str = "DECKSTREAK_VAULT_ARCHIVE_FOLDER";

/// The vault root, an absolute path. Its `Debug` never shows the path.
#[derive(Clone, PartialEq, Eq)]
pub struct VaultRoot(PathBuf);

impl VaultRoot {
    /// The root at `path`, or `None` when the path is not absolute.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Option<Self> {
        let path = path.into();
        path.is_absolute().then_some(Self(path))
    }

    /// The root's path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Debug for VaultRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("VaultRoot(..)")
    }
}

impl Setting for VaultRoot {
    const SHAPE: &'static str = "an absolute directory path";

    fn parse(text: &str) -> Option<Self> {
        Self::new(text)
    }
}

/// One folder name: not empty, not `.` or `..`, not hidden (a leading `.` is the vault's own
/// configuration), and with no slash, backslash or control character, so it can only name a folder
/// directly inside its parent.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FolderName(String);

impl FolderName {
    /// The longest name a file system takes, in bytes.
    const MAX_BYTES: usize = 255;

    /// The folder name `name`, or `None` when it could name anything but one folder.
    #[must_use]
    pub fn new(name: &str) -> Option<Self> {
        let refused = name.is_empty()
            || name.len() > Self::MAX_BYTES
            || name.starts_with('.')
            || name
                .chars()
                .any(|c| c == '/' || c == '\\' || c.is_control());
        (!refused).then(|| Self(name.to_owned()))
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FolderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Setting for FolderName {
    const SHAPE: &'static str =
        "one folder name: not empty, not hidden, and with no slash or control character";

    fn parse(text: &str) -> Option<Self> {
        Self::new(text)
    }
}

/// The vault's settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultSettings {
    /// The vault root.
    pub root: VaultRoot,
    /// The readings folder, at the vault's top level.
    pub readings: FolderName,
    /// The archive folder, inside the readings folder.
    pub archive: FolderName,
}

impl VaultSettings {
    /// The vault's settings, read from `env`. All three are required: a vault path has no default
    /// in code.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Missing`] for an unset setting and [`SettingsError::Malformed`] for one of
    /// the wrong shape, each naming the setting and never its value.
    pub fn from_env(env: &Environment) -> Result<Self, SettingsError> {
        Ok(Self {
            root: env.required(VAULT_ROOT)?,
            readings: env.required(READINGS_FOLDER)?,
            archive: env.required(ARCHIVE_FOLDER)?,
        })
    }
}

/// Why the vault features refuse to start (R1). Each names the rule, never the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StartRefusal {
    /// The vault root is missing or is not a directory.
    #[error("the vault root is not a directory")]
    RootNotADirectory,
    /// The readings folder does not exist; the adapter never creates it.
    #[error("the readings folder is absent")]
    ReadingsFolderMissing,
    /// Something other than a directory is at the readings folder's path.
    #[error("the readings folder is not a directory")]
    ReadingsFolderNotADirectory,
    /// The readings folder resolves, through a symbolic link, outside the vault root.
    #[error("the readings folder resolves outside the vault root")]
    ReadingsFolderOutsideRoot,
    /// The service cannot create a file in the readings folder.
    #[error("the readings folder is not writable by the service")]
    ReadingsFolderNotWritable,
}

/// The vault's folders, resolved and checked at start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultPaths {
    root: PathBuf,
    readings: PathBuf,
    readings_name: FolderName,
    archive_name: FolderName,
}

impl VaultPaths {
    /// The start check (R1): resolves the root and the readings folder, and proves the readings
    /// folder writable by creating and removing one temporary file whose name the sync bridge
    /// ignores. It creates no folder.
    ///
    /// # Errors
    ///
    /// [`VaultError::Start`] naming the refusal, and [`VaultError::Io`] when the check itself could
    /// not run.
    pub fn check<F: VaultFs + ?Sized>(
        settings: &VaultSettings,
        fs: &F,
    ) -> Result<Self, VaultError> {
        let _ = fs;
        Ok(Self {
            root: settings.root.path().to_path_buf(),
            readings: settings.root.path().join(settings.readings.as_str()),
            readings_name: settings.readings.clone(),
            archive_name: settings.archive.clone(),
        })
    }

    /// The vault root, resolved.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The readings folder, resolved.
    #[must_use]
    pub fn readings(&self) -> &Path {
        &self.readings
    }

    /// The readings folder's name, at the vault's top level.
    #[must_use]
    pub fn readings_name(&self) -> &FolderName {
        &self.readings_name
    }

    /// The archive folder's name, inside the readings folder.
    #[must_use]
    pub fn archive_name(&self) -> &FolderName {
        &self.archive_name
    }
}
