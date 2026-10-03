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

use std::collections::{BTreeMap, BTreeSet};

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
    match source.find(':') {
        Some(at) => source.get(..=at).unwrap_or(source),
        None => source,
    }
}

/// One bucket while the rows are folded: its XP, its graduated cards and the days it paid on.
#[derive(Default)]
struct Tally {
    /// The XP of its rows.
    total: i64,
    /// The graduations of the distinct days it paid on.
    graduated: i64,
    /// The distinct days it paid on.
    days: BTreeSet<StudyDay>,
}

/// `numerator` over `denominator`, as the predecessor's float division reads them.
#[allow(
    clippy::cast_precision_loss,
    reason = "XP and graduation counts stay far below 2^53, where an i64 converts exactly"
)]
fn ratio(numerator: i64, denominator: i64) -> f64 {
    numerator as f64 / denominator as f64
}

/// The rate of each bucket of `rows`, in byte order, over the `graduations` of each study day.
#[must_use]
pub fn exchange_rates(rows: &[XpRow], graduations: &BTreeMap<StudyDay, i64>) -> Vec<SourceRate> {
    let mut buckets: BTreeMap<&str, Tally> = BTreeMap::new();
    for row in rows {
        let bucket = buckets.entry(bucket(&row.source)).or_default();
        bucket.total += row.amount;
        if bucket.days.insert(row.study_day) {
            bucket.graduated += graduations.get(&row.study_day).copied().unwrap_or(0);
        }
    }
    buckets
        .into_iter()
        .map(|(source, tally)| {
            let graduated_cards = tally.graduated;
            let rate_defined = graduated_cards > 0;
            SourceRate {
                source: source.to_owned(),
                total_xp: tally.total,
                graduated_cards,
                rate: rate_defined.then(|| ratio(tally.total, graduated_cards)),
                rate_defined,
            }
        })
        .collect()
}

/// Every XP row of both tables in `window` (its first and last day, both included), or of every
/// day when there is no window: the grants of `xp_ledger`, then the settled rows of
/// `xp_settlement`.
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn xp_rows(
    connection: &mut SqliteConnection,
    window: Option<(StudyDay, StudyDay)>,
) -> Result<Vec<XpRow>, KernelError> {
    let (first, last) = window.map_or((i64::MIN, i64::MAX), |(first, last)| {
        (first.epoch_day(), last.epoch_day())
    });
    let granted = sqlx::query!(
        r#"SELECT study_day AS "study_day!: i64", source, amount AS "amount!: i64"
           FROM xp_ledger WHERE study_day BETWEEN ?1 AND ?2"#,
        first,
        last
    )
    .fetch_all(&mut *connection)
    .await?;
    let settled = sqlx::query!(
        r#"SELECT study_day AS "study_day!: i64", source, amount AS "amount!: i64"
           FROM xp_settlement WHERE study_day BETWEEN ?1 AND ?2"#,
        first,
        last
    )
    .fetch_all(&mut *connection)
    .await?;
    let mut rows: Vec<XpRow> = granted
        .into_iter()
        .map(|row| XpRow {
            study_day: StudyDay::from_epoch_day(row.study_day),
            source: row.source,
            amount: row.amount,
        })
        .collect();
    let settled = settled.into_iter().map(|row| XpRow {
        study_day: StudyDay::from_epoch_day(row.study_day),
        source: row.source,
        amount: row.amount,
    });
    rows.extend(settled);
    Ok(rows)
}
