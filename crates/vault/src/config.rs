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
use std::io;
use std::path::{Path, PathBuf};

use deck_streak_kernel::{Environment, Setting, SettingsError};

use crate::VaultError;
use crate::fs::{EntryKind, VaultFs};

/// The vault root: an absolute directory path, private to the deployment.
pub const VAULT_ROOT: &str = "DECKSTREAK_VAULT_ROOT";
/// The readings folder, one folder name at the vault's top level (example value `12-Readings`).
pub const READINGS_FOLDER: &str = "DECKSTREAK_VAULT_READINGS_FOLDER";
/// The archive folder inside the readings folder, one folder name (example value `Archive`).
pub const ARCHIVE_FOLDER: &str = "DECKSTREAK_VAULT_ARCHIVE_FOLDER";
/// The owner's vault layout file, an absolute path (SPEC-118 R4). Unset, the vendored layout
/// (`crates/vault/data/layout.json`) is in force. The owner's layout is private and never enters the
/// repository.
pub const VAULT_LAYOUT: &str = "DECKSTREAK_VAULT_LAYOUT";

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
    /// The layout file `DECKSTREAK_VAULT_LAYOUT` names cannot be read, or the setting names no
    /// absolute path (SPEC-118 R4).
    #[error("the vault layout file cannot be read")]
    LayoutUnreadable,
    /// The layout file is not a layout: not JSON, no inbox folder, or a folder that is not a plain
    /// relative path inside the vault (SPEC-118 R4).
    #[error("the vault layout file is not a layout")]
    LayoutMalformed,
}

/// The layout in force (SPEC-118 R4): the inbox folder captures land in, and the journal folders no
/// write may reach (R5). Each is a path relative to the vault root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutInForce {
    /// The inbox folder, relative to the vault root.
    pub inbox: String,
    /// The journal folders, relative to the vault root.
    pub journal: Vec<String>,
}

impl LayoutInForce {
    /// The layout `DECKSTREAK_VAULT_LAYOUT` names, or the vendored layout when it is unset.
    ///
    /// # Errors
    ///
    /// [`StartRefusal::LayoutUnreadable`] when the setting names no absolute path or a file that
    /// cannot be read, and [`StartRefusal::LayoutMalformed`] when the file is not a layout.
    pub fn from_env(env: &Environment) -> Result<Self, VaultError> {
        let Some(file) = env
            .optional::<LayoutFile>(VAULT_LAYOUT)
            .map_err(|_malformed| StartRefusal::LayoutUnreadable)?
        else {
            return Self::vendored();
        };
        let text =
            std::fs::read_to_string(&file.0).map_err(|_unread| StartRefusal::LayoutUnreadable)?;
        Self::parse(&text)
    }

    /// The vendored layout, `crates/vault/data/layout.json`.
    ///
    /// # Errors
    ///
    /// [`StartRefusal::LayoutMalformed`] when the vendored file is not a layout, which a test of
    /// this crate rules out.
    pub fn vendored() -> Result<Self, VaultError> {
        Self::parse(crate::staged::VENDORED_LAYOUT)
    }

    /// The layout a layout file's `text` holds: its `inbox` and its `journal` folders. Each is
    /// trimmed of `/` and must be a plain relative path inside the vault; `journal`, when present,
    /// is a list. Every other key is the duties' and is not read here.
    ///
    /// # Errors
    ///
    /// [`StartRefusal::LayoutMalformed`] when `text` is not a layout.
    pub fn parse(text: &str) -> Result<Self, VaultError> {
        let malformed = || VaultError::from(StartRefusal::LayoutMalformed);
        let layout: serde_json::Value = serde_json::from_str(text).map_err(|_json| malformed())?;
        let inbox = layout
            .get("inbox")
            .and_then(serde_json::Value::as_str)
            .and_then(layout_folder)
            .ok_or_else(malformed)?;
        let journal = match layout.get("journal") {
            None => Vec::new(),
            Some(serde_json::Value::Array(folders)) => folders
                .iter()
                .map(|folder| folder.as_str().and_then(layout_folder))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(malformed)?,
            Some(_) => return Err(malformed()),
        };
        Ok(Self { inbox, journal })
    }

    /// The journal folders inside `root`.
    #[must_use]
    pub fn journal_paths(&self, root: &Path) -> Vec<PathBuf> {
        self.journal
            .iter()
            .map(|folder| root.join(folder))
            .collect()
    }
}

/// A layout folder trimmed of `/`, or `None` unless it is a plain relative path inside the vault:
/// not empty, and with no empty, `.` or `..` component, no backslash and no control character.
fn layout_folder(text: &str) -> Option<String> {
    let folder = text.trim_matches('/');
    let plain = !folder.is_empty()
        && folder.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.chars().any(|c| c == '\\' || c.is_control())
        });
    plain.then(|| folder.to_owned())
}

/// The owner's layout file: an absolute path, private to the deployment.
struct LayoutFile(PathBuf);

impl Setting for LayoutFile {
    const SHAPE: &'static str = "an absolute file path";

    fn parse(text: &str) -> Option<Self> {
        let path = PathBuf::from(text);
        path.is_absolute().then_some(Self(path))
    }
}

/// The vault's folders, resolved and checked at start. Its `Debug` names the folders and never
/// the host's paths.
#[derive(Clone, PartialEq, Eq)]
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
        let root = match fs.canonicalize(settings.root.path()) {
            Ok(root) => root,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(StartRefusal::RootNotADirectory.into());
            }
            Err(error) => return Err(VaultError::io("resolve the vault root")(error)),
        };
        if fs
            .kind(&root)
            .map_err(VaultError::io("read the vault root"))?
            != Some(EntryKind::Dir)
        {
            return Err(StartRefusal::RootNotADirectory.into());
        }
        let configured = root.join(settings.readings.as_str());
        match fs
            .kind(&configured)
            .map_err(VaultError::io("read the readings folder"))?
        {
            None => return Err(StartRefusal::ReadingsFolderMissing.into()),
            Some(EntryKind::Dir | EntryKind::Symlink) => {}
            Some(_) => return Err(StartRefusal::ReadingsFolderNotADirectory.into()),
        }
        let readings = match fs.canonicalize(&configured) {
            Ok(readings) => readings,
            // A link to nothing: the folder it names is absent.
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(StartRefusal::ReadingsFolderMissing.into());
            }
            Err(error) => return Err(VaultError::io("resolve the readings folder")(error)),
        };
        if fs
            .kind(&readings)
            .map_err(VaultError::io("read the readings folder"))?
            != Some(EntryKind::Dir)
        {
            return Err(StartRefusal::ReadingsFolderNotADirectory.into());
        }
        if readings == root || !readings.starts_with(&root) {
            return Err(StartRefusal::ReadingsFolderOutsideRoot.into());
        }
        if !crate::atomic::probe_writable(fs, &readings)? {
            return Err(StartRefusal::ReadingsFolderNotWritable.into());
        }
        Ok(Self {
            root,
            readings,
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

impl fmt::Debug for VaultPaths {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VaultPaths")
            .field("readings", &self.readings_name)
            .field("archive", &self.archive_name)
            .finish_non_exhaustive()
    }
}
