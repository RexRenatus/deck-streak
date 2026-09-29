//! The settlement of a study day's derived XP (SPEC-072 R6 to R10; ADR-072).
//!
//! What a day's reviews and its daily bonuses come to is derived, so it is settled into
//! `xp_settlement` rather than granted: one row per (study day, source, track), written by the
//! fold's recompute through [`settle`] and by nothing else in the running service. An open day's
//! row is replaced as the day grows; a closed day's is only ever raised by a recompute, so a
//! recompute over the same data changes nothing and one over more data adds. Only the owner's
//! correction lowers a row. The XP total is the sum of this table and `xp_ledger` (R10).

use deck_streak_kernel::{StudyDay, Track, UtcMillis};
use sqlx::SqliteConnection;

/// The table the settlement lives in (`migrations/007201_progression_xp_settlement.sql`).
pub const XP_SETTLEMENT_TABLE: &str = "xp_settlement";

/// The derived sources: the ones a day's XP is settled for, and the only ones [`settle`] accepts.
/// Every other XP is a grant through the port.
pub const DERIVED_SOURCES: [&str; 9] = [
    "reviews",
    "reviews_law",
    "studied",
    "backlog_zero",
    "streak",
    "score90",
    "graduations",
    "consistency",
    "ascendant",
];

/// Why a settlement was asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettleCause {
    /// The fold's recompute: an open day's amount is replaced, a closed day's only raised.
    Recompute,
    /// The owner's correction: the amount is replaced, whatever it was.
    OwnersCorrection,
}

/// One settlement, as the fold asks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettleRequest<'a> {
    /// The study day settled.
    pub study_day: StudyDay,
    /// The derived source, one of [`DERIVED_SOURCES`].
    pub source: &'a str,
    /// The track the XP is on.
    pub track: Track,
    /// The XP the day's data comes to.
    pub amount: u32,
    /// Whether the day is over.
    pub closed: bool,
}

/// Why a settlement was refused.
#[derive(Debug, thiserror::Error)]
pub enum SettleError {
    /// The source is not a derived one. The refusal names the rule and never the source.
    #[error("a settlement's source is one of the derived sources")]
    NotDerived,
    /// The database refused the operation.
    #[error("the database refused the settlement")]
    Database(#[from] sqlx::Error),
}

/// Settles `request` on `connection`, inside the caller's write transaction, and answers the
/// amount the row holds after it.
///
/// # Errors
///
/// [`SettleError::NotDerived`] before any write when the source is not derived;
/// [`SettleError::Database`] when the database refuses.
pub async fn settle(
    connection: &mut SqliteConnection,
    request: &SettleRequest<'_>,
    cause: SettleCause,
    at: UtcMillis,
) -> Result<u32, SettleError> {
    if !DERIVED_SOURCES.contains(&request.source) {
        return Err(SettleError::NotDerived);
    }
    let day = request.study_day.epoch_day();
    let track = request.track.as_str();
    let source = request.source;
    let held = sqlx::query!(
        r#"SELECT amount AS "amount!: u32", closed AS "closed!: bool" FROM xp_settlement
           WHERE study_day = ?1 AND source = ?2 AND track = ?3"#,
        day,
        source,
        track
    )
    .fetch_optional(&mut *connection)
    .await?;
    let (amount, closed) = match (held, cause) {
        (Some(held), SettleCause::Recompute) if held.closed || request.closed => {
            (held.amount.max(request.amount), true)
        }
        _ => (request.amount, request.closed),
    };
    let at = at.epoch_millis();
    sqlx::query!(
        "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT (study_day, source, track) \
         DO UPDATE SET amount = excluded.amount, closed = excluded.closed",
        day,
        source,
        track,
        amount,
        closed,
        at
    )
    .execute(connection)
    .await?;
    Ok(amount)
}
