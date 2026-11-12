//! What the fold writes for the streaks context (SPEC-076 R16, R17, R27): the two streak rows, the
//! freeze events, the habit strength, the governor's one row and the relight's due list, on the
//! connection the fold holds.
//!
//! The context owns these tables, so every statement that names them lives here. Instants are epoch
//! milliseconds and days are epoch days, as the tables store them.

use deck_streak_kernel::{KernelError, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use crate::freeze::{FreezeEvent, FreezeReason};
use crate::streak::StreakState;

/// The governor's one row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GovernorRow {
    /// The open lapse's anchor, when one is open.
    pub lapse_since: Option<StudyDay>,
    /// Whether the last settled day stood by.
    pub standby: bool,
    /// The study day of the last standby notice.
    pub notified_day: Option<StudyDay>,
}

/// Writes `events`; a day's fold-owned event that is already there is left as it is, so a recompute
/// of a settled day writes none again (R17).
///
/// # Errors
///
/// [`KernelError`] when the write fails.
pub async fn insert_events(
    connection: &mut SqliteConnection,
    events: &[FreezeEvent],
    at: UtcMillis,
) -> Result<(), KernelError> {
    let created = at.epoch_millis();
    for event in events {
        let day = event.day.epoch_day();
        let delta = i64::from(event.delta);
        let reason = event.reason.as_str();
        sqlx::query!(
            "INSERT OR IGNORE INTO freeze_events (study_day, delta, reason, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            day,
            delta,
            reason,
            created
        )
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

/// Stores `state` as the row of `track` (`language` or `law`).
///
/// # Errors
///
/// [`KernelError`] when the write fails.
pub async fn upsert_state(
    connection: &mut SqliteConnection,
    track: &str,
    state: &StreakState,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let current = i64::from(state.current);
    let longest = i64::from(state.longest);
    let freezes = i64::from(state.freezes);
    let last = state.last_study_day.map(StudyDay::epoch_day);
    let armed = i64::from(state.comeback_armed);
    let created = at.epoch_millis();
    sqlx::query!(
        "INSERT INTO streak_state
             (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT (track) DO UPDATE SET
             current_days = excluded.current_days,
             longest_days = excluded.longest_days,
             freezes = excluded.freezes,
             last_study_day = excluded.last_study_day,
             comeback_armed = excluded.comeback_armed",
        track,
        current,
        longest,
        freezes,
        last,
        armed,
        created
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Stores the habit strength of `day`, replacing an earlier value for it.
///
/// # Errors
///
/// [`KernelError`] when the write fails.
pub async fn put_strength(
    connection: &mut SqliteConnection,
    day: StudyDay,
    strength: f64,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let day = day.epoch_day();
    let created = at.epoch_millis();
    sqlx::query!(
        "INSERT INTO habit_strength (study_day, strength, created_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (study_day) DO UPDATE SET strength = excluded.strength",
        day,
        strength,
        created
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// The governor's row, as the last settled day left it.
///
/// # Errors
///
/// [`KernelError`] when the read fails.
pub async fn governor(connection: &mut SqliteConnection) -> Result<GovernorRow, KernelError> {
    let row =
        sqlx::query!("SELECT lapse_since, standby, notified_day FROM governor_state WHERE id = 1")
            .fetch_optional(&mut *connection)
            .await?;
    Ok(row.map_or(
        GovernorRow {
            lapse_since: None,
            standby: false,
            notified_day: None,
        },
        |row| GovernorRow {
            lapse_since: row.lapse_since.map(StudyDay::from_epoch_day),
            standby: row.standby != 0,
            notified_day: row.notified_day.map(StudyDay::from_epoch_day),
        },
    ))
}

/// Stores the governor's anchor and standby for a settled day, keeping the notice day.
///
/// # Errors
///
/// [`KernelError`] when the write fails.
pub async fn put_governor(
    connection: &mut SqliteConnection,
    lapse_since: Option<StudyDay>,
    standby: bool,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let anchor = lapse_since.map(StudyDay::epoch_day);
    let standby = i64::from(standby);
    let created = at.epoch_millis();
    sqlx::query!(
        "INSERT INTO governor_state (id, lapse_since, standby, notified_day, created_at)
         VALUES (1, ?1, ?2, NULL, ?3)
         ON CONFLICT (id) DO UPDATE SET
             lapse_since = excluded.lapse_since,
             standby = excluded.standby",
        anchor,
        standby,
        created
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// The stored streak of `track` (`language` or `law`), when the fold has written one.
///
/// # Errors
///
/// [`KernelError`] when the read fails.
pub async fn state(
    connection: &mut SqliteConnection,
    track: &str,
) -> Result<Option<StreakState>, KernelError> {
    let row = sqlx::query!(
        "SELECT current_days, longest_days, freezes, last_study_day, comeback_armed
         FROM streak_state WHERE track = ?1",
        track
    )
    .fetch_optional(&mut *connection)
    .await?;
    Ok(row.map(|row| StreakState {
        current: u32::try_from(row.current_days).unwrap_or(0),
        longest: u32::try_from(row.longest_days).unwrap_or(0),
        freezes: u32::try_from(row.freezes).unwrap_or(0),
        last_study_day: row.last_study_day.map(StudyDay::from_epoch_day),
        comeback_armed: row.comeback_armed != 0,
    }))
}

/// Every freeze event, oldest first.
///
/// # Errors
///
/// [`KernelError`] when the read fails.
pub async fn events(connection: &mut SqliteConnection) -> Result<Vec<FreezeEvent>, KernelError> {
    let rows = sqlx::query!("SELECT study_day, delta, reason FROM freeze_events ORDER BY id")
        .fetch_all(&mut *connection)
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            Some(FreezeEvent {
                day: StudyDay::from_epoch_day(row.study_day),
                delta: i32::try_from(row.delta).ok()?,
                reason: FreezeReason::parse(&row.reason)?,
            })
        })
        .collect())
}

/// The net freezes the outside sources paid (a shop purchase, a chest, the weekly quest, a season
/// node): what the fold's own replay cannot know, and adds back to the language row.
///
/// # Errors
///
/// [`KernelError`] when the read fails.
pub async fn external_freezes(connection: &mut SqliteConnection) -> Result<i64, KernelError> {
    let row = sqlx::query!(
        "SELECT COALESCE(SUM(delta), 0) AS \"net!: i64\" FROM freeze_events
         WHERE reason IN ('chest', 'weekly_quest', 'season', 'shop')"
    )
    .fetch_one(&mut *connection)
    .await?;
    Ok(row.net)
}

/// The strength of the latest day the fold stored, when it has stored one.
///
/// # Errors
///
/// [`KernelError`] when the read fails.
pub async fn latest_strength(
    connection: &mut SqliteConnection,
) -> Result<Option<f64>, KernelError> {
    let row = sqlx::query!("SELECT strength FROM habit_strength ORDER BY study_day DESC LIMIT 1")
        .fetch_optional(&mut *connection)
        .await?;
    Ok(row.map(|row| row.strength))
}

/// Adds one freeze to the language row and its event, in the caller's write (R7).
///
/// # Errors
///
/// [`KernelError`] when a statement fails.
pub async fn add_freeze(
    connection: &mut SqliteConnection,
    day: StudyDay,
    reason: FreezeReason,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let held = state(connection, "language")
        .await?
        .unwrap_or_else(StreakState::start);
    let raised = StreakState {
        freezes: held.freezes + 1,
        ..held
    };
    upsert_state(connection, "language", &raised, at).await?;
    let event = FreezeEvent {
        day,
        delta: 1,
        reason,
    };
    insert_events(connection, &[event], at).await
}

/// Records `day` as due its relight celebration, in the write that grants its relight, so the day is
/// due exactly when the grant commits (R27). A day already due is left as it is.
///
/// # Errors
///
/// [`KernelError`] when the write fails.
pub async fn put_relight_due(
    connection: &mut SqliteConnection,
    day: StudyDay,
    at: UtcMillis,
) -> Result<(), KernelError> {
    let day = day.epoch_day();
    let created = at.epoch_millis();
    sqlx::query!(
        "INSERT INTO relight_due (study_day, created_at) VALUES (?1, ?2)
         ON CONFLICT (study_day) DO NOTHING",
        day,
        created
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

/// Every study day still due its relight celebration, oldest first (R27).
///
/// # Errors
///
/// [`KernelError`] when the read fails.
pub async fn relight_due(connection: &mut SqliteConnection) -> Result<Vec<StudyDay>, KernelError> {
    let rows = sqlx::query!("SELECT study_day FROM relight_due ORDER BY study_day")
        .fetch_all(&mut *connection)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| StudyDay::from_epoch_day(row.study_day))
        .collect())
}

/// Removes `day` from the due list, once the router has decided its celebration (R27).
///
/// # Errors
///
/// [`KernelError`] when the write fails.
pub async fn clear_relight_due(
    connection: &mut SqliteConnection,
    day: StudyDay,
) -> Result<(), KernelError> {
    let day = day.epoch_day();
    sqlx::query!("DELETE FROM relight_due WHERE study_day = ?1", day)
        .execute(&mut *connection)
        .await?;
    Ok(())
}
