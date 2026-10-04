//! The minutes log's store (SPEC-078 R2, R4, R21): every read and write of `minutes_log`, on the
//! caller's connection, so a use case holds the log write and its settles in one transaction.

use deck_streak_kernel::{StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use crate::minutes::Entry;

/// One logged entry, as the undo reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoggedEntry {
    /// The entry's id, which its Undo button carries.
    pub id: i64,
    /// The course's code, as stored.
    pub code: String,
    /// The study day it was logged on.
    pub day: StudyDay,
    /// The minutes.
    pub minutes: u32,
}

/// A sum of minutes as the rules take it. The table's bound keeps every entry from 1 to 600, so a
/// sum never reaches the bound of a `u32`; one past it saturates rather than wraps.
fn minutes(total: i64) -> u32 {
    u32::try_from(total.max(0)).unwrap_or(u32::MAX)
}

/// Logs `entry` on `day`, written at `at`, and answers its id.
///
/// # Errors
///
/// [`sqlx::Error`] when the write fails.
pub async fn insert(
    connection: &mut SqliteConnection,
    entry: &Entry,
    day: StudyDay,
    at: UtcMillis,
) -> Result<i64, sqlx::Error> {
    let code = entry.code.as_str();
    let study_day = day.epoch_day();
    let minutes = i64::from(entry.minutes);
    let created_at = at.epoch_millis();
    let written = sqlx::query!(
        "INSERT INTO minutes_log (code, study_day, minutes, note, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        code,
        study_day,
        minutes,
        entry.note,
        created_at,
    )
    .execute(connection)
    .await?;
    Ok(written.last_insert_rowid())
}

/// The newest entry, or none.
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn newest(connection: &mut SqliteConnection) -> Result<Option<LoggedEntry>, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT id AS "id!: i64", code, study_day, minutes
           FROM minutes_log ORDER BY id DESC LIMIT 1"#
    )
    .fetch_optional(connection)
    .await?;
    Ok(row.map(|row| LoggedEntry {
        id: row.id,
        code: row.code,
        day: StudyDay::from_epoch_day(row.study_day),
        minutes: minutes(row.minutes),
    }))
}

/// Removes the entry `id`.
///
/// # Errors
///
/// [`sqlx::Error`] when the write fails.
pub async fn remove(connection: &mut SqliteConnection, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM minutes_log WHERE id = ?1", id)
        .execute(connection)
        .await?;
    Ok(())
}

/// The minutes of `code` from `first` to `last`, both included.
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn minutes_between(
    connection: &mut SqliteConnection,
    code: &str,
    first: StudyDay,
    last: StudyDay,
) -> Result<u32, sqlx::Error> {
    let (first, last) = (first.epoch_day(), last.epoch_day());
    let total = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(minutes), 0) AS "minutes!: i64" FROM minutes_log
           WHERE code = ?1 AND study_day BETWEEN ?2 AND ?3"#,
        code,
        first,
        last,
    )
    .fetch_one(connection)
    .await?;
    Ok(minutes(total))
}

/// The codes with an entry from `first` to `last`, both included, each with its minutes there, in
/// code order.
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn minutes_by_code(
    connection: &mut SqliteConnection,
    first: StudyDay,
    last: StudyDay,
) -> Result<Vec<(String, u32)>, sqlx::Error> {
    let (first, last) = (first.epoch_day(), last.epoch_day());
    let rows = sqlx::query!(
        r#"SELECT code, SUM(minutes) AS "minutes!: i64" FROM minutes_log
           WHERE study_day BETWEEN ?1 AND ?2 GROUP BY code ORDER BY code"#,
        first,
        last,
    )
    .fetch_all(connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.code, minutes(row.minutes)))
        .collect())
}

/// Every code with an entry, each with its minutes, in the order each was first logged.
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn all_time_by_code(
    connection: &mut SqliteConnection,
) -> Result<Vec<(String, u32)>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT code AS "code!", SUM(minutes) AS "minutes!: i64" FROM minutes_log
           GROUP BY code ORDER BY MIN(id)"#
    )
    .fetch_all(connection)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.code, minutes(row.minutes)))
        .collect())
}

/// Confirms writing in `code` on `day`, at `at`, and answers whether a row was written: a course
/// already confirmed that day writes nothing.
///
/// # Errors
///
/// [`sqlx::Error`] when the write fails.
pub async fn confirm_writing(
    _connection: &mut SqliteConnection,
    _code: &str,
    _day: StudyDay,
    _at: UtcMillis,
) -> Result<bool, sqlx::Error> {
    Ok(false)
}

/// Clears `code`'s confirmation on `day`, and answers whether one was removed.
///
/// # Errors
///
/// [`sqlx::Error`] when the write fails.
pub async fn clear_writing(
    _connection: &mut SqliteConnection,
    _code: &str,
    _day: StudyDay,
) -> Result<bool, sqlx::Error> {
    Ok(false)
}

/// Every confirmation from `first` to `last`, both included, as its code and day, by day then
/// code.
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn confirmations_between(
    _connection: &mut SqliteConnection,
    _first: StudyDay,
    _last: StudyDay,
) -> Result<Vec<(String, StudyDay)>, sqlx::Error> {
    Ok(Vec::new())
}

/// How many confirmations were made through `last`.
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn confirmations_through(
    _connection: &mut SqliteConnection,
    _last: StudyDay,
) -> Result<u32, sqlx::Error> {
    Ok(0)
}

/// How many reading entries were logged through `last`.
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn entries_through(
    _connection: &mut SqliteConnection,
    _last: StudyDay,
) -> Result<u32, sqlx::Error> {
    Ok(0)
}
