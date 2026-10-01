//! The records step (SPEC-073 R10 to R12; ADR-303): phase 7 of the fold keeps each kind's best,
//! and the offers raise each new record's celebration until the router answers.
//!
//! A closing day at its settle and the current day are evaluated: the detection runs over the
//! [`RECORDS_WINDOW`] most recent rollups on or before the day, the day itself counting with the
//! score the badges read, against the stored bests. The first detection, with no record stored,
//! seeds each kind's best silently. A record row holds one kind, so the day's write re-reads the
//! row inside its own `BEGIN IMMEDIATE`: the fold's offers ran before it and offered every unmarked
//! record, and a row of another day whose mark is still unset is named in a log line (its kind, its
//! day, and that its celebration was not sent) before it is replaced.

use std::collections::BTreeMap;

use deck_streak_analytics::rollup::{self, recent_totals};
use deck_streak_kernel::{Db, KernelError, PortFuture, StudyDay, UtcMillis};
use deck_streak_progression::records::{DayTotals, Detected, RecordKind, detect_records};
use sqlx::SqliteConnection;

use super::badges::judged_score;
use super::{Celebrate, Celebration, DayEvaluation, DayStep, Phase};

/// The records step's name, as the fold's report and log name it.
pub const RECORDS_STEP: &str = "progression.records";

/// The ladder's event a record celebration names.
pub const RECORD_EVENT: &str = "record";

/// How many of the most recent rollups the detection reads (`pipeline.py`'s
/// `get_recent_rollups(370)`).
pub const RECORDS_WINDOW: i64 = 370;

/// One row a records evaluation writes: the kind, its new best, the day that set it and the value
/// it beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedRecord {
    /// The kind.
    pub kind: RecordKind,
    /// The new best.
    pub value: i64,
    /// The day that set it.
    pub study_day: StudyDay,
    /// The value it beat; equal to the value for a seeded best.
    pub previous: i64,
}

/// What one records evaluation does (`goldens/records_window.json`): the rows it writes, the
/// dedupe keys it owes a celebration under, and whether it seeded the bests silently.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordPlan {
    /// The rows written, in [`RecordKind::ALL`]'s order.
    pub written: Vec<PlannedRecord>,
    /// The dedupe keys owed a celebration, in the rows' order.
    pub celebrated: Vec<String>,
    /// Whether the bests were seeded silently.
    pub seeded: bool,
}

/// A stored record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredRecord {
    /// The kind.
    pub kind: RecordKind,
    /// The best.
    pub value: i64,
    /// The day that set it.
    pub study_day: StudyDay,
    /// The value it beat.
    pub previous: i64,
    /// When its celebration was marked, once the router answered.
    pub celebrated_at: Option<UtcMillis>,
}

/// The dedupe key of a record's celebration: `pr:<kind>:<epoch day>` (R11).
#[must_use]
pub fn record_key(kind: RecordKind, day: StudyDay) -> String {
    format!("pr:{}:{}", kind.as_str(), day.epoch_day())
}

/// The line a record's celebration carries.
#[must_use]
pub fn record_line(kind: RecordKind, previous: i64, value: i64) -> String {
    format!(
        "\u{1f4c8} <b>NEW RECORD</b> \u{2014} {}: {previous} \u{2192} {value}",
        kind.label()
    )
}

/// The plan of one records evaluation on `day` (`pipeline.py:_compute_and_store_coaching`): with
/// `empty` (no record row) and `stored` bests, every best is seeded with `previous` equal to its
/// value and none is celebrated; otherwise each record in `new` is written with the stored best it
/// beat (0 when none is stored) and owes its celebration.
#[must_use]
pub fn plan(
    stored: &BTreeMap<RecordKind, i64>,
    empty: bool,
    new: &[Detected],
    day: StudyDay,
) -> RecordPlan {
    if empty && !stored.is_empty() {
        // The bests the owner has already seen are seeded with `previous` equal to their value,
        // and none is celebrated as new.
        let mut merged = stored.clone();
        for record in new {
            merged.insert(record.kind, record.value);
        }
        let written = RecordKind::ALL
            .into_iter()
            .filter_map(|kind| {
                merged.get(&kind).map(|value| PlannedRecord {
                    kind,
                    value: *value,
                    study_day: day,
                    previous: *value,
                })
            })
            .collect();
        return RecordPlan {
            written,
            celebrated: Vec::new(),
            seeded: true,
        };
    }
    let written: Vec<PlannedRecord> = new
        .iter()
        .map(|record| PlannedRecord {
            kind: record.kind,
            value: record.value,
            study_day: day,
            previous: stored.get(&record.kind).copied().unwrap_or(0),
        })
        .collect();
    let celebrated = written
        .iter()
        .map(|record| record_key(record.kind, record.study_day))
        .collect();
    RecordPlan {
        written,
        celebrated,
        seeded: false,
    }
}

/// Every stored record, in [`RecordKind::ALL`]'s order.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn stored_records(
    connection: &mut SqliteConnection,
) -> Result<Vec<StoredRecord>, KernelError> {
    let rows = sqlx::query!(
        "SELECT kind, value, study_day, previous, celebrated_at FROM records ORDER BY kind"
    )
    .fetch_all(&mut *connection)
    .await?;
    let mut records: Vec<StoredRecord> = rows
        .into_iter()
        .filter_map(|row| {
            // The column's check admits only the three kinds.
            RecordKind::parse(&row.kind).map(|kind| StoredRecord {
                kind,
                value: row.value,
                study_day: StudyDay::from_epoch_day(row.study_day),
                previous: row.previous,
                celebrated_at: row.celebrated_at.map(UtcMillis::from_epoch_millis),
            })
        })
        .collect();
    records.sort_by_key(|record| record.kind);
    Ok(records)
}

/// Phase 7's records step.
#[derive(Clone, Copy, Debug, Default)]
pub struct RecordsStep;

impl DayStep for RecordsStep {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        RECORDS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            if !day.evaluation.runs_today_only_rules() {
                return Ok(());
            }
            let score = rollup::stored(write, day.day)
                .await?
                .map(|stored| judged_score(&stored, day.evaluation));
            let window: Vec<DayTotals> = recent_totals(write, day.day, RECORDS_WINDOW)
                .await?
                .into_iter()
                .map(|totals| DayTotals {
                    score: score
                        .filter(|_| totals.day == day.day)
                        .unwrap_or(totals.score),
                    reviews: totals.reviews,
                    seconds: totals.seconds,
                })
                .collect();
            let held = stored_records(write).await?;
            let empty = held.is_empty();
            // With no record stored, the window's own bests are what the owner has already seen:
            // the first detection seeds them silently (R12).
            let stored: BTreeMap<RecordKind, i64> = if empty {
                detect_records(&window, &BTreeMap::new())
                    .into_iter()
                    .map(|record| (record.kind, record.value))
                    .collect()
            } else {
                held.iter()
                    .map(|record| (record.kind, record.value))
                    .collect()
            };
            let new = detect_records(&window, &stored);
            let plan = plan(&stored, empty, &new, day.day);
            let marked = plan.seeded.then_some(day.facts.now);
            for record in &plan.written {
                upsert(write, record, marked, day.facts.now).await?;
            }
            Ok(())
        })
    }
}

/// Writes `record` over its kind's row inside the day's own write. The row is re-read first: one
/// of another day whose mark is still unset was never offered, and is named before it is replaced
/// (ADR-303). A row of the same day keeps its mark, so a best that climbs all day is celebrated
/// once.
async fn upsert(
    write: &mut SqliteConnection,
    record: &PlannedRecord,
    marked: Option<UtcMillis>,
    now: UtcMillis,
) -> Result<(), KernelError> {
    let kind = record.kind.as_str();
    let day = record.study_day.epoch_day();
    let held = sqlx::query!(
        "SELECT study_day, celebrated_at FROM records WHERE kind = ?1",
        kind
    )
    .fetch_optional(&mut *write)
    .await?;
    if let Some(held) = held
        && held.study_day != day
        && held.celebrated_at.is_none()
    {
        tracing::warn!(
            kind,
            day = %StudyDay::from_epoch_day(held.study_day),
            "celebration not sent: the record was replaced before it was offered"
        );
    }
    let marked = marked.map(UtcMillis::epoch_millis);
    let at = now.epoch_millis();
    sqlx::query!(
        "INSERT INTO records (kind, value, study_day, previous, celebrated_at, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT (kind) DO UPDATE SET value = excluded.value, previous = excluded.previous, \
         celebrated_at = CASE WHEN records.study_day = excluded.study_day \
         THEN records.celebrated_at ELSE excluded.celebrated_at END, \
         study_day = excluded.study_day",
        kind,
        record.value,
        day,
        record.previous,
        marked,
        at
    )
    .execute(&mut *write)
    .await?;
    Ok(())
}

/// Offers every record whose mark is unset to `celebrate`, and marks each one it answered at
/// `now`, in a write of its own that marks only while the row still holds that day's record
/// (ADR-303). An offer the router did not answer leaves the record owed for the next offers.
///
/// # Errors
///
/// [`KernelError`] when the owed records cannot be read, or a mark cannot be written.
pub async fn offer_records(
    celebrate: &dyn Celebrate,
    db: &Db,
    now: UtcMillis,
    today: StudyDay,
) -> Result<(), KernelError> {
    let owed: Vec<StoredRecord> = {
        let mut read = db.reader().acquire().await?;
        stored_records(&mut read).await?
    }
    .into_iter()
    .filter(|record| record.celebrated_at.is_none())
    .collect();
    for record in owed {
        let celebration = Celebration {
            event: RECORD_EVENT,
            key: record_key(record.kind, record.study_day),
            text: record_line(record.kind, record.previous, record.value),
            study_day: today,
        };
        // The mark is set only once the router has answered (ADR-303).
        if let Err(error) = celebrate.celebrate(&celebration).await {
            tracing::warn!(key = %celebration.key, %error, "the router did not answer; the record stays owed");
            continue;
        }
        let at = now.epoch_millis();
        let kind = record.kind.as_str();
        let day = record.study_day.epoch_day();
        let mut write = db.write().await?;
        sqlx::query!(
            "UPDATE records SET celebrated_at = ?1 \
             WHERE kind = ?2 AND study_day = ?3 AND celebrated_at IS NULL",
            at,
            kind,
            day
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
    }
    Ok(())
}
