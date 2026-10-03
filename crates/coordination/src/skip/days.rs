//! The skip-set port (SPEC-083 R4; ADR-321 D12): the one function every reader of the skip days
//! calls.

use std::collections::BTreeSet;

use deck_streak_kernel::{KernelError, StudyDay};
use sqlx::SqliteConnection;

/// The study days that hold an applied skip not undone, read on the caller's connection.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
#[allow(
    clippy::unused_async,
    reason = "the red stub keeps the green signature, which reads the record"
)]
pub async fn skip_days(
    connection: &mut SqliteConnection,
) -> Result<BTreeSet<StudyDay>, KernelError> {
    let _ = connection;
    Ok(BTreeSet::new())
}

/// The same set as epoch day numbers, as the consistency run reads it.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn skip_epoch_days(
    connection: &mut SqliteConnection,
) -> Result<BTreeSet<i64>, KernelError> {
    Ok(skip_days(connection)
        .await?
        .into_iter()
        .map(StudyDay::epoch_day)
        .collect())
}
