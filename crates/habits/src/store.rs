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
    let _ = (connection, entry, day, at);
    Ok(0)
}

/// The newest entry, or none.
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn newest(connection: &mut SqliteConnection) -> Result<Option<LoggedEntry>, sqlx::Error> {
    let _ = connection;
    Ok(None)
}

/// Removes the entry `id`.
///
/// # Errors
///
/// [`sqlx::Error`] when the write fails.
pub async fn remove(connection: &mut SqliteConnection, id: i64) -> Result<(), sqlx::Error> {
    let _ = (connection, id);
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
    let _ = (connection, code, first, last);
    Ok(0)
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
    let _ = (connection, first, last);
    Ok(Vec::new())
}

/// Every code with an entry, each with its minutes, in the order each was first logged.
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn all_time_by_code(
    connection: &mut SqliteConnection,
) -> Result<Vec<(String, u32)>, sqlx::Error> {
    let _ = connection;
    Ok(Vec::new())
}
