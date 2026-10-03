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

/// An Epic's prize, as the owner chose it (R9): a double-XP token or a streak freeze. The stored
/// row's `choice` is '' until it is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// A double-XP token, held until the owner activates it (R12).
    Token,
    /// A streak freeze, granted through the streaks' freeze port with the reason `chest`.
    Freeze,
}

impl Choice {
    /// The name the stored row carries.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::Freeze => "freeze",
        }
    }

    /// The choice named `name`, none for the stored '' of a choice not made, and an error for any
    /// other name.
    ///
    /// # Errors
    ///
    /// [`ChestError::Unreadable`] for a name that is neither '' nor a choice.
    pub fn from_stored(name: &str) -> Result<Option<Self>, ChestError> {
        match name {
            "" => Ok(None),
            "token" => Ok(Some(Self::Token)),
            "freeze" => Ok(Some(Self::Freeze)),
            _ => Err(unreadable("chests.choice", name)),
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
            choice: None,
            announced: false,
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
    /// An Epic's choice once it is made; none until then, and for every other rarity.
    pub choice: Option<Choice>,
    /// Whether its announcement was handed to the router (T3): the caller's to set.
    pub announced: bool,
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
    let day = study_day.epoch_day();
    let rows = sqlx::query!(
        "SELECT id, study_day, origin, session_start, rarity, payout_xp, state, choice, \
         announced FROM chests WHERE study_day = ?1 ORDER BY id",
        day
    )
    .fetch_all(&mut *connection)
    .await?;
    rows.into_iter()
        .map(|row| {
            stored(&ChestRow {
                id: row.id,
                study_day: row.study_day,
                origin: &row.origin,
                session_start: row.session_start,
                rarity: &row.rarity,
                payout_xp: row.payout_xp,
                state: &row.state,
                choice: &row.choice,
                announced: row.announced,
            })
        })
        .collect()
}

/// The chest `id`, or none when no chest holds it.
///
/// # Errors
///
/// [`ChestError::Database`] when the read fails, and [`ChestError::Unreadable`] for a row whose
/// value is outside its column's rule.
pub async fn chest(
    connection: &mut SqliteConnection,
    id: i64,
) -> Result<Option<StoredChest>, ChestError> {
    let row = sqlx::query!(
        "SELECT id, study_day, origin, session_start, rarity, payout_xp, state, choice, \
         announced FROM chests WHERE id = ?1",
        id
    )
    .fetch_optional(&mut *connection)
    .await?;
    row.map(|row| {
        stored(&ChestRow {
            id: row.id,
            study_day: row.study_day,
            origin: &row.origin,
            session_start: row.session_start,
            rarity: &row.rarity,
            payout_xp: row.payout_xp,
            state: &row.state,
            choice: &row.choice,
            announced: row.announced,
        })
    })
    .transpose()
}

/// Every chest still sealed, vaulted or opened, of every study day, in the order they were
/// stored: the sweep's population (R10).
///
/// # Errors
///
/// [`ChestError::Database`] when the read fails, and [`ChestError::Unreadable`] for a row whose
/// value is outside its column's rule.
pub async fn unresolved_chests(
    connection: &mut SqliteConnection,
) -> Result<Vec<StoredChest>, ChestError> {
    let rows = sqlx::query!(
        "SELECT id, study_day, origin, session_start, rarity, payout_xp, state, choice, \
         announced FROM chests WHERE state IN ('sealed', 'vaulted', 'opened') ORDER BY id"
    )
    .fetch_all(&mut *connection)
    .await?;
    rows.into_iter()
        .map(|row| {
            stored(&ChestRow {
                id: row.id,
                study_day: row.study_day,
                origin: &row.origin,
                session_start: row.session_start,
                rarity: &row.rarity,
                payout_xp: row.payout_xp,
                state: &row.state,
                choice: &row.choice,
                announced: row.announced,
            })
        })
        .collect()
}

/// Opens chest `id`: sealed or vaulted becomes opened, and any other state is kept. Answers
/// whether this call opened it, so a second open is told apart from the first (R8).
///
/// # Errors
///
/// [`ChestError::Database`] when the write fails.
pub async fn mark_opened(connection: &mut SqliteConnection, id: i64) -> Result<bool, ChestError> {
    let written = sqlx::query!(
        "UPDATE chests SET state = 'opened' WHERE id = ?1 AND state IN ('sealed', 'vaulted')",
        id
    )
    .execute(&mut *connection)
    .await?;
    Ok(written.rows_affected() == 1)
}

/// Resolves opened chest `id` with no choice: the open's second step for a chest that pays XP
/// (R8). A chest in any other state is kept.
///
/// # Errors
///
/// [`ChestError::Database`] when the write fails.
pub async fn mark_resolved(connection: &mut SqliteConnection, id: i64) -> Result<(), ChestError> {
    sqlx::query!(
        "UPDATE chests SET state = 'resolved' WHERE id = ?1 AND state = 'opened'",
        id
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Settles opened Epic `id`'s choice and resolves it, unless its choice is made or it is not an
/// opened Epic. Answers whether this call settled it (R9).
///
/// # Errors
///
/// [`ChestError::Database`] when the write fails.
pub async fn settle_choice(
    connection: &mut SqliteConnection,
    id: i64,
    choice: Choice,
) -> Result<bool, ChestError> {
    let name = choice.name();
    let written = sqlx::query!(
        "UPDATE chests SET state = 'resolved', choice = ?2 \
         WHERE id = ?1 AND state = 'opened' AND rarity = 'epic' AND choice = ''",
        id,
        name
    )
    .execute(&mut *connection)
    .await?;
    Ok(written.rows_affected() == 1)
}

/// Resolves stale chest `id` from sealed, vaulted or opened. Answers whether this call resolved
/// it, so a chest another write resolved first is not paid again (R10).
///
/// # Errors
///
/// [`ChestError::Database`] when the write fails.
pub async fn sweep_resolve(connection: &mut SqliteConnection, id: i64) -> Result<bool, ChestError> {
    let written = sqlx::query!(
        "UPDATE chests SET state = 'resolved' \
         WHERE id = ?1 AND state IN ('sealed', 'vaulted', 'opened')",
        id
    )
    .execute(&mut *connection)
    .await?;
    Ok(written.rows_affected() == 1)
}

/// A stored row of `chests` as read, before its names are checked.
struct ChestRow<'a> {
    id: i64,
    study_day: i64,
    origin: &'a str,
    session_start: i64,
    rarity: &'a str,
    payout_xp: i64,
    state: &'a str,
    choice: &'a str,
    announced: i64,
}

/// The chest a stored row holds, refused when a value is outside its column's rule.
fn stored(row: &ChestRow<'_>) -> Result<StoredChest, ChestError> {
    Ok(StoredChest {
        id: row.id,
        study_day: StudyDay::from_epoch_day(row.study_day),
        origin: Origin::from_name(row.origin)
            .ok_or_else(|| unreadable("chests.origin", row.origin))?,
        session_start: row.session_start,
        rarity: Rarity::from_name(row.rarity)
            .ok_or_else(|| unreadable("chests.rarity", row.rarity))?,
        payout_xp: row.payout_xp,
        state: ChestState::from_name(row.state)
            .ok_or_else(|| unreadable("chests.state", row.state))?,
        choice: Choice::from_stored(row.choice)?,
        announced: match row.announced {
            0 => false,
            1 => true,
            other => return Err(unreadable("chests.announced", &other.to_string())),
        },
    })
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
    let day = chest.study_day.epoch_day();
    let origin = chest.origin.name();
    let rarity = chest.rarity.name();
    let state = chest.state.name();
    let at = at.epoch_millis();
    // The insert's own conflict with `chests_one_per_key` is the existence check, so the key lives
    // in the migration alone; a conflict writes nothing.
    let written = sqlx::query!(
        "INSERT INTO chests (study_day, origin, session_start, rarity, payout_xp, state, \
         created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT DO NOTHING",
        day,
        origin,
        chest.session_start,
        rarity,
        chest.payout_xp,
        state,
        at
    )
    .execute(&mut *connection)
    .await?;
    Ok((written.rows_affected() == 1).then(|| written.last_insert_rowid()))
}

/// The pity counters.
///
/// # Errors
///
/// [`ChestError::Database`] when the read fails.
pub async fn pity(connection: &mut SqliteConnection) -> Result<Pity, ChestError> {
    let row = sqlx::query!("SELECT since_epic, since_legendary FROM pity WHERE id = 1")
        .fetch_one(&mut *connection)
        .await?;
    Ok(Pity {
        since_epic: row.since_epic,
        since_legendary: row.since_legendary,
    })
}

/// Replaces the pity counters with `pity`, in place.
///
/// # Errors
///
/// [`ChestError::Database`] when the write fails.
pub async fn set_pity(connection: &mut SqliteConnection, pity: Pity) -> Result<(), ChestError> {
    sqlx::query!(
        "UPDATE pity SET since_epic = ?1, since_legendary = ?2 WHERE id = 1",
        pity.since_epic,
        pity.since_legendary
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// The chest settings.
///
/// # Errors
///
/// [`ChestError::Database`] when the read fails, and [`ChestError::Unreadable`] for a stored
/// vault hour outside a day.
pub async fn settings(connection: &mut SqliteConnection) -> Result<ChestSettings, ChestError> {
    let row = sqlx::query!("SELECT per_day_max, vault_hour FROM chest_settings WHERE id = 1")
        .fetch_one(&mut *connection)
        .await?;
    let vault_hour = u8::try_from(row.vault_hour)
        .ok()
        .and_then(Hour::new)
        .ok_or_else(|| unreadable("chest_settings.vault_hour", &row.vault_hour.to_string()))?;
    Ok(ChestSettings {
        per_day_max: row.per_day_max,
        vault_hour,
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
    let vault_hour = i64::from(settings.vault_hour.get());
    sqlx::query!(
        "UPDATE chest_settings SET per_day_max = ?1, vault_hour = ?2 WHERE id = 1",
        settings.per_day_max,
        vault_hour
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// The refusal of a stored value outside its column's rule.
fn unreadable(column: &'static str, value: &str) -> ChestError {
    ChestError::Unreadable {
        column,
        value: value.to_owned(),
    }
}
