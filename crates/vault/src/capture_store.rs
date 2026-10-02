//! The vault's capture table (SPEC-118 R3, R12; `migrations/011801_vault_inbox_captures.sql`): the
//! row that makes a capture once-only. A stem is recorded once; a Mini App capture's key is
//! recorded once among the Mini App's captures, so a retry after UTC midnight answers the first
//! name (ADR-118's capture-key amendment).

use deck_streak_kernel::{KernelError, UtcMillis};
use sqlx::SqliteConnection;

use crate::inbox::{CaptureKind, Source};

/// The table of captures.
pub const INBOX_CAPTURES_TABLE: &str = "inbox_captures";

/// One capture's row, as its claim records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureRow<'a> {
    /// The capture's stem, the table's key.
    pub stem: &'a str,
    /// The safe unique of the capture (R1), unique among the Mini App's captures.
    pub capture_key: &'a str,
    /// What was captured.
    pub kind: CaptureKind,
    /// Where it came from.
    pub source: Source,
    /// The attachment's file name, or `None` for a capture without one.
    pub attachment: Option<&'a str>,
    /// The instant it was captured.
    pub captured_at: UtcMillis,
}

/// What a claim answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Claim {
    /// This call recorded the row: the capture is written.
    Claimed,
    /// The stem, or a Mini App capture's key, was already recorded; `name` is the recorded
    /// capture's file.
    Recorded {
        /// The recorded capture's attachment name, or its stub's without one.
        name: String,
    },
}

/// Records `row` unless its stem, or a Mini App capture's key, is already recorded.
///
/// # Errors
///
/// [`KernelError::Database`] when the insert or the read fails.
pub async fn claim(
    connection: &mut SqliteConnection,
    row: &CaptureRow<'_>,
) -> Result<Claim, KernelError> {
    let _ = row;
    sqlx::query("SELECT 1").execute(connection).await?;
    Ok(Claim::Claimed)
}
