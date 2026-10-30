//! The day buffs (SPEC-072 R20 to R22): the Ascendant buff a study day holds, armed by the day
//! before it and read by the day's bonus and by the chest roll.

use deck_streak_kernel::{StudyDay, UtcMillis};
use sqlx::SqliteConnection;

/// The Ascendant buff's kind, as `buffs` stores it.
pub const ASCENDANT: &str = "ascendant";

/// Arms the Ascendant buff for `study_day`, at the instant `at`, inside the caller's write. A day
/// that already holds it is left as it is, and the answer says which happened.
///
/// # Errors
///
/// [`sqlx::Error`] when the write fails.
pub async fn arm_ascendant(
    connection: &mut SqliteConnection,
    study_day: StudyDay,
    at: UtcMillis,
) -> Result<bool, sqlx::Error> {
    let day = study_day.epoch_day();
    let at = at.epoch_millis();
    let written = sqlx::query!(
        "INSERT OR IGNORE INTO buffs (study_day, kind, created_at) VALUES (?1, 'ascendant', ?2)",
        day,
        at
    )
    .execute(connection)
    .await?
    .rows_affected();
    Ok(written == 1)
}

/// Whether `study_day` is an Ascendant day (R22).
///
/// # Errors
///
/// [`sqlx::Error`] when the read fails.
pub async fn is_ascendant_day(
    connection: &mut SqliteConnection,
    study_day: StudyDay,
) -> Result<bool, sqlx::Error> {
    let day = study_day.epoch_day();
    let held = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "held!: i64" FROM buffs WHERE study_day = ?1 AND kind = 'ascendant'"#,
        day
    )
    .fetch_one(connection)
    .await?;
    Ok(held > 0)
}
