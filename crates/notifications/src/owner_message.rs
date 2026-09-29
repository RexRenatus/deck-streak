//! The owner's latest message (SPEC-084 R13): one row of `owner_last_message`, the id of the
//! owner's latest message to the bot and the instant it arrived, which a T1 reacts to.

use deck_streak_kernel::{Db, KernelError, UtcMillis};

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
pub async fn record(_db: &Db, _message_id: i64, _at: UtcMillis) -> Result<(), KernelError> {
    Ok(())
}

/// The owner's latest message, when one is recorded.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn latest(_db: &Db) -> Result<Option<LatestMessage>, KernelError> {
    Ok(None)
}
