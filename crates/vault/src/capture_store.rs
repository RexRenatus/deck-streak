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

/// Records `row` unless its stem, or a Mini App capture's key, is already recorded (ADR-118 and its
/// capture-key amendment). The insert names no conflict target, so the stem's primary key and the
/// Mini App key's partial unique index each refuse it; when nothing is inserted, the recorded
/// capture's name answers: the row of the stem, or, for a Mini App capture, the Mini App row of its
/// key. A Telegram capture is claimed by its stem alone.
///
/// # Errors
///
/// [`KernelError::Database`] when the insert or the read fails, or when nothing was inserted and no
/// recorded row explains it.
pub async fn claim(
    connection: &mut SqliteConnection,
    row: &CaptureRow<'_>,
) -> Result<Claim, KernelError> {
    let kind = row.kind.as_str();
    let source = row.source.as_str();
    let at = row.captured_at.epoch_millis();
    let inserted = sqlx::query!(
        "INSERT INTO inbox_captures
             (stem, capture_key, kind, source, attachment, captured_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) ON CONFLICT DO NOTHING",
        row.stem,
        row.capture_key,
        kind,
        source,
        row.attachment,
        at
    )
    .execute(&mut *connection)
    .await?;
    if inserted.rows_affected() == 1 {
        return Ok(Claim::Claimed);
    }
    let by_stem = sqlx::query_scalar!(
        r#"SELECT COALESCE(attachment, stem || '.md') AS "name!: String"
           FROM inbox_captures WHERE stem = ?1"#,
        row.stem
    )
    .fetch_optional(&mut *connection)
    .await?;
    if let Some(name) = by_stem {
        return Ok(Claim::Recorded { name });
    }
    if row.source == Source::MiniApp {
        let by_key = sqlx::query_scalar!(
            r#"SELECT COALESCE(attachment, stem || '.md') AS "name!: String"
               FROM inbox_captures WHERE capture_key = ?1 AND source = 'miniapp'"#,
            row.capture_key
        )
        .fetch_optional(&mut *connection)
        .await?;
        if let Some(name) = by_key {
            return Ok(Claim::Recorded { name });
        }
    }
    Err(sqlx::Error::RowNotFound.into())
}
