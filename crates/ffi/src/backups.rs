//! The device's backups (SPEC-516 R5; ADR-534 D3, D4): back up now, the list and one backup's
//! bytes, in the collection's folder.

use std::fmt;

use crate::engine::Engine;

/// The kind of one device backup, as its file's name says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum DeviceBackupKind {
    /// A backup of this device's collection.
    Backup,
    /// A copy of the server's collection.
    ServerCopy,
}

/// One backup or server copy the collection's folder holds.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct DeviceBackup {
    /// Its file's stem: the one id the app names it by.
    pub id: String,
    /// Its kind.
    pub kind: DeviceBackupKind,
    /// How long ago it was made, in whole seconds.
    pub age_seconds: u64,
}

/// Why a backup call answered nothing.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum BackupRefusal {
    /// A full-sync choice's stage is held, so no backup is made until it ends.
    StageHeld,
    /// The id names no backup the folder lists.
    NotListed,
    /// The core refused the new file: it names the open collection, or a file that holds a row.
    Unfillable,
    /// The backup, read back from its file, lacks a card, note or review of the device.
    Unheld,
    /// The engine refused: its encoded `BackendError` message.
    Engine {
        /// The engine's error, as protobuf bytes.
        error: Vec<u8>,
    },
    /// The folder could not be read or written: the operating system's reason, never a path.
    Folder {
        /// The operating system's reason.
        reason: String,
    },
}

impl fmt::Display for BackupRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StageHeld => f.write_str("a full sync's choice is open, so no backup was made"),
            Self::NotListed => f.write_str("no listed backup has that id"),
            Self::Unfillable => f.write_str("the backup's file could not be used"),
            Self::Unheld => f.write_str("the backup lacks a row of this device"),
            Self::Engine { error } => {
                write!(f, "the engine refused the backup ({} bytes)", error.len())
            }
            Self::Folder { reason } => write!(f, "the backups' folder refused: {reason}"),
        }
    }
}

impl std::error::Error for BackupRefusal {}

#[uniffi::export]
impl Engine {
    /// Back up now.
    ///
    /// # Errors
    ///
    /// Every [`BackupRefusal`].
    pub fn back_up_now(&self, folder: String) -> Result<String, BackupRefusal> {
        let _ = folder;
        Err(BackupRefusal::NotListed)
    }

    /// The folder's backups.
    ///
    /// # Errors
    ///
    /// Every [`BackupRefusal`].
    pub fn device_backups(&self, folder: String) -> Result<Vec<DeviceBackup>, BackupRefusal> {
        let _ = folder;
        Ok(Vec::new())
    }

    /// One listed backup's bytes.
    ///
    /// # Errors
    ///
    /// Every [`BackupRefusal`].
    pub fn backup_bytes(&self, folder: String, id: String) -> Result<Vec<u8>, BackupRefusal> {
        let _ = (folder, id);
        Err(BackupRefusal::NotListed)
    }
}
