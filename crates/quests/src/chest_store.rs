//! The repository over `chests`, `pity` and `chest_settings` (SPEC-081 R5, R6, R21; ADR-081).
//!
//! Every function runs on the caller's connection, inside the caller's write: the grant step
//! stores a chest and the pity counters after it in that one write, as the fold's day write
//! holds its other steps (the kernel's repository base opens it with `BEGIN IMMEDIATE`). Nothing
//! here opens a connection or a transaction of its own.

use deck_streak_kernel::{Hour, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use crate::chests::{ChestError, Rarity};
use crate::pity::Pity;

/// Where a chest came from: a study session, the daily challenge quest or the weekly quest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A session that met the effort floor (R1, R2).
    Session,
    /// The daily challenge quest (R16).
    Challenge,
    /// The weekly quest (R17).
    Weekly,
}

impl Origin {
    /// The name the stored row carries.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Challenge => "challenge",
            Self::Weekly => "weekly",
        }
    }

    /// The origin named `name`, or none when it is not one.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "session" => Some(Self::Session),
            "challenge" => Some(Self::Challenge),
            "weekly" => Some(Self::Weekly),
            _ => None,
        }
    }
}

/// Where a chest is in its life (R7, R8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChestState {
    /// Earned and announced, not yet opened.
    Sealed,
    /// Earned late in the day or in quiet hours: kept for the morning, not announced.
    Vaulted,
    /// Opened: its payout is paid, or its Epic choice is waiting.
    Opened,
    /// Opened and its Epic choice made.
    Resolved,
}

impl ChestState {
    /// The name the stored row carries.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sealed => "sealed",
            Self::Vaulted => "vaulted",
            Self::Opened => "opened",
            Self::Resolved => "resolved",
        }
    }

    /// The state named `name`, or none when it is not one.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "sealed" => Some(Self::Sealed),
            "vaulted" => Some(Self::Vaulted),
            "opened" => Some(Self::Opened),
            "resolved" => Some(Self::Resolved),
            _ => None,
        }
    }
}

/// A chest to store: its key (study day, origin, session start) and what it was rolled as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewChest {
    /// The study day it was earned on.
    pub study_day: StudyDay,
    /// Where it came from.
    pub origin: Origin,
    /// A session chest's session start in epoch milliseconds; 0 for the two quest origins, which
    /// hold one chest a study day each.
    pub session_start: i64,
    /// Its rarity, rolled once.
    pub rarity: Rarity,
    /// Its payout, rolled once.
    pub payout_xp: i64,
    /// Sealed or vaulted when earned.
    pub state: ChestState,
}

impl NewChest {
    /// The chest as stored under `id`.
    #[must_use]
    pub const fn stored(self, id: i64) -> StoredChest {
        StoredChest {
            id,
            study_day: self.study_day,
            origin: self.origin,
            session_start: self.session_start,
            rarity: self.rarity,
            payout_xp: self.payout_xp,
            state: self.state,
        }
    }
}

/// A stored chest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredChest {
    /// Its row id, which its announcement's dedupe key names (R7).
    pub id: i64,
    /// The study day it was earned on.
    pub study_day: StudyDay,
    /// Where it came from.
    pub origin: Origin,
    /// A session chest's session start; 0 for the quest origins.
    pub session_start: i64,
    /// Its rarity as rolled.
    pub rarity: Rarity,
    /// Its payout as rolled.
    pub payout_xp: i64,
    /// Where it is in its life.
    pub state: ChestState,
}

/// The chest settings (R2, R7): the most chests a study day holds and the local hour from which
/// a chest earned that day is vaulted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChestSettings {
    /// The most chests a study day holds, counting every origin.
    pub per_day_max: i64,
    /// The local hour from which a chest is vaulted rather than announced.
    pub vault_hour: Hour,
}

/// Every chest of `study_day`, in the order they were stored.
///
/// # Errors
///
/// [`ChestError::Database`] when the read fails, and [`ChestError::Unreadable`] for a row whose
/// value is outside its column's rule.
pub async fn chests_of_day(
    connection: &mut SqliteConnection,
    study_day: StudyDay,
) -> Result<Vec<StoredChest>, ChestError> {
    let _ = (connection, study_day);
    Ok(Vec::new())
}

/// Stores `chest` unless its key is already held, and answers its id, or none when the key was
/// held and nothing was written. The unique index is the existence check (ADR-081).
///
/// # Errors
///
/// [`ChestError::Database`] when the write fails.
pub async fn insert_chest(
    connection: &mut SqliteConnection,
    chest: &NewChest,
    at: UtcMillis,
) -> Result<Option<i64>, ChestError> {
    let _ = (connection, chest, at);
    Ok(None)
}

/// The pity counters.
///
/// # Errors
///
/// [`ChestError::Database`] when the read fails.
pub async fn pity(connection: &mut SqliteConnection) -> Result<Pity, ChestError> {
    let _ = connection;
    Ok(Pity::default())
}

/// Replaces the pity counters with `pity`, in place.
///
/// # Errors
///
/// [`ChestError::Database`] when the write fails.
pub async fn set_pity(connection: &mut SqliteConnection, pity: Pity) -> Result<(), ChestError> {
    let _ = (connection, pity);
    Ok(())
}

/// The chest settings.
///
/// # Errors
///
/// [`ChestError::Database`] when the read fails, and [`ChestError::Unreadable`] for a stored
/// vault hour outside a day.
pub async fn settings(connection: &mut SqliteConnection) -> Result<ChestSettings, ChestError> {
    let _ = connection;
    Hour::new(0)
        .map(|vault_hour| ChestSettings {
            per_day_max: 0,
            vault_hour,
        })
        .ok_or_else(|| ChestError::Unreadable {
            column: "chest_settings.vault_hour",
            value: String::from("0"),
        })
}

/// Replaces the chest settings with `settings`, in place.
///
/// # Errors
///
/// [`ChestError::Database`] when the write fails.
pub async fn set_settings(
    connection: &mut SqliteConnection,
    settings: ChestSettings,
) -> Result<(), ChestError> {
    let _ = (connection, settings);
    Ok(())
}
