//! The landmarks' two settings in `notification_settings` (SPEC-102 R5, R6; ADR-322): the mark
//! the predecessor stores once, and the cursor `landmarks_offered_through`, the epoch day through
//! which every landmark dated after the seed has been answered by the router.
//!
//! Each function opens its own write (BEGIN IMMEDIATE), as the router does, because the landmarks
//! are offered between the fold's writes and never inside one. The queries are runtime queries,
//! as the flush lease's are, so the offline query cache is unchanged.

use deck_streak_kernel::{Db, KernelError, StudyDay, UtcMillis};
use sqlx::SqliteConnection;

use crate::landmarks::LANDMARK_HIGH_WATER_KEY;

/// The cursor's key. Its value is an epoch day as decimal text.
const OFFERED_THROUGH_KEY: &str = "landmarks_offered_through";

/// Seeds both settings in ONE write: `mark` where no mark is stored, and the cursor at `through`
/// where no cursor is stored. It answers `true` exactly when that write read no mark, so the
/// recompute is the first run; racing first recomputes serialize on the write, and only one of
/// them reads the mark absent (SPEC-102 R6). A stored value is never overwritten.
///
/// # Errors
///
/// [`KernelError::Database`] when the settings cannot be read or written.
pub async fn seed(
    db: &Db,
    mark: &str,
    through: StudyDay,
    now: UtcMillis,
) -> Result<bool, KernelError> {
    let mut write = db.write().await?;
    let first = setting(&mut write, LANDMARK_HIGH_WATER_KEY)
        .await?
        .is_none();
    if first {
        insert(&mut write, LANDMARK_HIGH_WATER_KEY, mark, now).await?;
    }
    if setting(&mut write, OFFERED_THROUGH_KEY).await?.is_none() {
        insert(&mut write, OFFERED_THROUGH_KEY, &day_text(through), now).await?;
    }
    write.commit().await?;
    Ok(first)
}

/// The cursor, read on `read`: `None` before the first seed.
///
/// # Errors
///
/// [`KernelError::Database`] when the setting cannot be read, or holds a value that is not an
/// epoch day, which no write here stores: a cursor that cannot be read moves nothing.
pub async fn offered_through(read: &mut SqliteConnection) -> Result<Option<StudyDay>, KernelError> {
    let Some(value) = setting(read, OFFERED_THROUGH_KEY).await? else {
        return Ok(None);
    };
    value
        .parse::<i64>()
        .map(|day| Some(StudyDay::from_epoch_day(day)))
        .map_err(|_| {
            KernelError::Database(sqlx::Error::Protocol(format!(
                "the landmarks' cursor holds {value:?}, not an epoch day"
            )))
        })
}

/// Moves the cursor to `through` in a write of its own, and never backwards: a slower recompute
/// that read an older settle cursor leaves a later value in place (SPEC-102 R5).
///
/// # Errors
///
/// [`KernelError::Database`] when the setting cannot be written.
pub async fn advance(db: &Db, through: StudyDay, now: UtcMillis) -> Result<(), KernelError> {
    let mut write = db.write().await?;
    sqlx::query(
        "INSERT INTO notification_settings (key, value, created_at) VALUES (?, ?, ?) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value \
         WHERE CAST(notification_settings.value AS INTEGER) < CAST(excluded.value AS INTEGER)",
    )
    .bind(OFFERED_THROUGH_KEY)
    .bind(day_text(through))
    .bind(now.epoch_millis())
    .execute(&mut *write)
    .await?;
    write.commit().await?;
    Ok(())
}

/// The value stored under `key`, read on `connection`.
async fn setting(
    connection: &mut SqliteConnection,
    key: &str,
) -> Result<Option<String>, KernelError> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT value FROM notification_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(connection)
            .await?,
    )
}

/// Inserts `key` with `value`, which the same write has just read absent.
async fn insert(
    write: &mut SqliteConnection,
    key: &str,
    value: &str,
    now: UtcMillis,
) -> Result<(), KernelError> {
    sqlx::query("INSERT INTO notification_settings (key, value, created_at) VALUES (?, ?, ?)")
        .bind(key)
        .bind(value)
        .bind(now.epoch_millis())
        .execute(write)
        .await?;
    Ok(())
}

/// `day` as the cursor stores it.
fn day_text(day: StudyDay) -> String {
    day.epoch_day().to_string()
}
