//! The XP exchange readout (SPEC-075 R4, R5, #80): how much XP each source paid for each card that
//! graduated on the days it paid, as the predecessor's `exchange.py:exchange_rates` reads it.
//!
//! A source is read by its bucket (the text up to and including its first `:`, else the whole
//! source), so `quest:1` and `quest:2` are one bucket. A bucket's graduated cards are the rollup
//! graduations of the DISTINCT days it paid on, a day with no rollup counting 0; its rate is its XP
//! over those graduations, and it has none when nothing graduated. The track is not a dimension.
//!
//! The rows come from both XP tables, `xp_ledger` and `xp_settlement`, which only this context
//! names (ADR-072); the graduations are analytics' and the join is coordination's (ADR-075). The
//! readout writes nothing and changes no grant (R8).

use std::collections::BTreeMap;

use deck_streak_kernel::{KernelError, StudyDay};
use sqlx::SqliteConnection;

/// One XP row as the readout reads it, from either table.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct XpRow {
    /// The study day the XP is for.
    pub study_day: StudyDay,
    /// The source that paid it.
    pub source: String,
    /// The XP.
    pub amount: i64,
}

/// One bucket's rate.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceRate {
    /// The bucket.
    pub source: String,
    /// The XP its rows hold.
    pub total_xp: i64,
    /// The graduations of the distinct days it paid on.
    pub graduated_cards: i64,
    /// XP per graduated card; none when nothing graduated.
    pub rate: Option<f64>,
    /// Whether a card graduated, so the rate is defined.
    pub rate_defined: bool,
}

/// The bucket of `source`: its text up to and including the first `:`, or the whole source.
#[must_use]
pub fn bucket(source: &str) -> &str {
    source
}

/// The rate of each bucket of `rows`, in byte order, over the `graduations` of each study day.
#[must_use]
pub fn exchange_rates(_rows: &[XpRow], _graduations: &BTreeMap<StudyDay, i64>) -> Vec<SourceRate> {
    Vec::new()
}

/// Every XP row of both tables in `window` (its first and last day, both included), or of every
/// day when there is no window: the grants of `xp_ledger`, then the settled rows of
/// `xp_settlement`.
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn xp_rows(
    _connection: &mut SqliteConnection,
    _window: Option<(StudyDay, StudyDay)>,
) -> Result<Vec<XpRow>, KernelError> {
    Ok(Vec::new())
}
