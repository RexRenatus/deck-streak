//! What the fold writes for the streaks context (SPEC-076 R16, R17): the two streak rows, the
//! freeze events, the habit strength and the governor's one row, on the connection the fold holds.
//!
//! The context owns these tables, so every statement that names them lives here. Instants are epoch
//! milliseconds and days are epoch days, as the tables store them.

use deck_streak_kernel::{KernelError, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use crate::freeze::FreezeEvent;
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
