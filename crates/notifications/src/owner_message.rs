//! The owner's latest message (SPEC-084 R13): one row of `owner_last_message`, the id of the
//! owner's latest message to the bot and the instant it arrived, which a T1 reacts to.

use deck_streak_kernel::{Db, KernelError, UtcMillis};
use sqlx::SqliteConnection;

/// The owner's latest message to the bot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LatestMessage {
    /// The message's id in the owner's chat.
    pub message_id: i64,
    /// When it arrived.
    pub arrived_at: UtcMillis,
}

/// Records `message_id`, arrived at `at`, as the owner's latest message.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn record(db: &Db, message_id: i64, at: UtcMillis) -> Result<(), KernelError> {
    let arrived_at = at.epoch_millis();
    let mut write = db.write().await?;
    sqlx::query!(
        "UPDATE owner_last_message SET message_id = ?, arrived_at = ? WHERE id = 1",
        message_id,
        arrived_at,
    )
    .execute(&mut *write)
    .await?;
    write.commit().await?;
    Ok(())
}

/// The owner's latest message, when one is recorded.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn latest(db: &Db) -> Result<Option<LatestMessage>, KernelError> {
    let mut connection = db.reader().acquire().await?;
    read(&mut connection).await
}

/// The owner's latest message, read on `connection`: the router reads it inside its own write.
pub(crate) async fn read(
    connection: &mut SqliteConnection,
) -> Result<Option<LatestMessage>, KernelError> {
    let row = sqlx::query!("SELECT message_id, arrived_at FROM owner_last_message WHERE id = 1")
        .fetch_optional(connection)
        .await?;
    Ok(row.and_then(|row| {
        Some(LatestMessage {
            message_id: row.message_id?,
            arrived_at: UtcMillis::from_epoch_millis(row.arrived_at?),
        })
    }))
}
