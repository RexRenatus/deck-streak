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

use deck_streak_kernel::{Db, KernelError, PortFuture, StudyDay, UtcMillis};
use deck_streak_progression::records::{Detected, RecordKind};
use sqlx::SqliteConnection;

use super::{Celebrate, DayEvaluation, DayStep, Phase};

/// The records step's name, as the fold's report and log name it.
pub const RECORDS_STEP: &str = "progression.records";

/// The ladder's event a record celebration names.
pub const RECORD_EVENT: &str = "record";

/// How many of the most recent rollups the detection reads (`pipeline.py`'s
/// `get_recent_rollups(370)`).
pub const RECORDS_WINDOW: i64 = 0;

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
    let _ = (kind, day);
    String::new()
}

/// The line a record's celebration carries.
#[must_use]
pub fn record_line(kind: RecordKind, previous: i64, value: i64) -> String {
    let _ = (kind, previous, value);
    String::new()
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
    let _ = (stored, empty, new, day);
    RecordPlan::default()
}

/// Every stored record, in [`RecordKind::ALL`]'s order.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn stored_records(
    connection: &mut SqliteConnection,
) -> Result<Vec<StoredRecord>, KernelError> {
    let _ = connection;
    Ok(Vec::new())
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
            let _ = (day, write);
            Ok(())
        })
    }
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
    let _ = (celebrate, db, now, today);
    Ok(())
}
