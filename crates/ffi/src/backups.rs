//! The device's backups (SPEC-516 R5; ADR-534 D3, D4): back up now, the list and one backup's
//! bytes, in the collection's folder the app names.
//!
//! A backup and a server copy are the files the full-sync choice makes, `backup-{stamp}-{process}`
//! and `server-copy-{stamp}-{process}` with `.anki2`, the stamp the clock's nanoseconds when the
//! adapter named the file; back up now names its file the same way. The list reads only those
//! names, so no file time is read (a required-reason API on iOS), and it applies the core's
//! retention, the rule the web keeps (SPEC-377 R17): the newest three of each kind stay, and the
//! newest of a kind is never removed. While a full-sync choice's stage is held, back up now makes
//! nothing and the list removes nothing, so the counted server copy is never taken from under the
//! confirm. An export answers the bytes of a file the list names by an id, and only such a file:
//! an id is never a path.

use std::fmt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_engine_core::backup_now;
use deck_streak_engine_core::dispatch::Refusal;
use deck_streak_engine_core::one_way::Reason;
use deck_streak_engine_core::retention::{self, Held};

use crate::engine::{Engine, choice_file};

/// The kind of one device backup, as its file's name says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum DeviceBackupKind {
    /// A backup of this device's collection.
    Backup,
    /// A copy of the server's collection.
    ServerCopy,
}

impl DeviceBackupKind {
    /// The word a file of this kind is named by.
    const fn word(self) -> &'static str {
        match self {
            Self::Backup => "backup",
            Self::ServerCopy => "server-copy",
        }
    }
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

/// One file the list names: its id, its kind and the stamp its name carries.
struct Named {
    id: String,
    kind: DeviceBackupKind,
    stamp: u128,
}

/// A second, in the nanoseconds the names' stamps count.
const SECOND_NANOS: u128 = 1_000_000_000;

#[uniffi::export]
impl Engine {
    /// Back up now: the core writes the open collection into a new backup in `folder`, named as a
    /// full sync names its backup, and reads it back; answers the new backup's id.
    ///
    /// # Errors
    ///
    /// [`BackupRefusal::StageHeld`] while a full-sync choice is open; [`BackupRefusal::Unfillable`]
    /// and [`BackupRefusal::Unheld`] for the core's refusals of the file; and
    /// [`BackupRefusal::Engine`] when the engine refuses, no collection being open among them.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's values cross the boundary owned, as the bindings pass them"
    )]
    pub fn back_up_now(&self, folder: String) -> Result<String, BackupRefusal> {
        if self.stage_held() {
            return Err(BackupRefusal::StageHeld);
        }
        let backup = choice_file(&folder, DeviceBackupKind::Backup.word());
        backup_now::back_up(self.dispatcher(), &backup).map_err(unbacked)?;
        let named = backup
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(named);
        named.map(|named| named.id).ok_or(BackupRefusal::Unfillable)
    }

    /// The backups and server copies in `folder`, newest first, each by its id, its kind and its
    /// age in whole seconds, once the core's retention removed what its rule removes; while a
    /// full-sync choice is open it removes nothing.
    ///
    /// # Errors
    ///
    /// [`BackupRefusal::Folder`] when the folder cannot be read or a removal fails.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's values cross the boundary owned, as the bindings pass them"
    )]
    pub fn device_backups(&self, folder: String) -> Result<Vec<DeviceBackup>, BackupRefusal> {
        let folder = Path::new(&folder);
        let mut listed = listed(folder)?;
        if !self.stage_held() {
            let held: Vec<Held<DeviceBackupKind, u128>> = listed
                .iter()
                .map(|named| Held {
                    kind: named.kind,
                    made: named.stamp,
                })
                .collect();
            let removed = retention::removals(&held);
            for index in removed.iter().rev() {
                let named = listed.remove(*index);
                std::fs::remove_file(folder.join(file_of(&named.id)))
                    .map_err(|error| folder_refusal(&error))?;
            }
        }
        listed.sort_by(|a, b| (b.stamp, &b.id).cmp(&(a.stamp, &a.id)));
        let now = now_nanos();
        Ok(listed
            .into_iter()
            .map(|named| DeviceBackup {
                id: named.id,
                kind: named.kind,
                age_seconds: age(named.stamp, now),
            })
            .collect())
    }

    /// The bytes of the backup or server copy `id` names in `folder`, when the folder lists it as
    /// one: an id is a listed file's stem, never a path.
    ///
    /// # Errors
    ///
    /// [`BackupRefusal::NotListed`] for any other id, and [`BackupRefusal::Folder`] when the folder
    /// or the file cannot be read.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's values cross the boundary owned, as the bindings pass them"
    )]
    pub fn backup_bytes(&self, folder: String, id: String) -> Result<Vec<u8>, BackupRefusal> {
        let folder = Path::new(&folder);
        let listed = listed(folder)?;
        if !listed.iter().any(|named| named.id == id) {
            return Err(BackupRefusal::NotListed);
        }
        std::fs::read(folder.join(file_of(&id))).map_err(|error| folder_refusal(&error))
    }
}

/// Every backup and server copy `folder` holds, in the folder's order.
fn listed(folder: &Path) -> Result<Vec<Named>, BackupRefusal> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(folder).map_err(|error| folder_refusal(&error))? {
        let entry = entry.map_err(|error| folder_refusal(&error))?;
        if let Some(named) = entry.file_name().to_str().and_then(named) {
            found.push(named);
        }
    }
    Ok(found)
}

/// The backup or server copy a file name names: `{kind}-{stamp}-{process}.anki2`, the stamp and
/// the process each a number spelt as the number it is. Any other name is none.
fn named(file: &str) -> Option<Named> {
    let id = file.strip_suffix(".anki2")?;
    let (kind, rest) = [DeviceBackupKind::ServerCopy, DeviceBackupKind::Backup]
        .into_iter()
        .find_map(|kind| Some((kind, id.strip_prefix(kind.word())?.strip_prefix('-')?)))?;
    let (stamp, process) = rest.split_once('-')?;
    let stamp = number(stamp)?;
    number(process)?;
    Some(Named {
        id: id.to_owned(),
        kind,
        stamp,
    })
}

/// `text` as the number it spells, when it spells one with no sign and no leading zero.
fn number(text: &str) -> Option<u128> {
    text.parse::<u128>()
        .ok()
        .filter(|value| value.to_string() == text)
}

/// The file name of the backup `id` names.
fn file_of(id: &str) -> String {
    format!("{id}.anki2")
}

/// The clock now, in nanoseconds since the epoch, as the names' stamps count it.
fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos())
}

/// How long ago, in whole seconds, a file stamped `stamp` was named; never negative.
fn age(stamp: u128, now: u128) -> u64 {
    u64::try_from(now.saturating_sub(stamp) / SECOND_NANOS).unwrap_or(u64::MAX)
}

/// A folder's refusal, by the operating system's reason alone.
fn folder_refusal(error: &std::io::Error) -> BackupRefusal {
    BackupRefusal::Folder {
        reason: error.kind().to_string(),
    }
}

/// Back up now's refusal, from the core's reason.
fn unbacked(reason: Reason) -> BackupRefusal {
    match reason {
        Reason::Engine(Refusal::Engine { error }) => BackupRefusal::Engine { error },
        Reason::Engine(_) => BackupRefusal::Engine { error: Vec::new() },
        Reason::Unheld => BackupRefusal::Unheld,
        Reason::OpenCollection | Reason::HoldsRows | Reason::Gesture(_) => {
            BackupRefusal::Unfillable
        }
    }
}
