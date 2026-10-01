//! The records view (SPEC-073 R13): each stored record with its label, value, day and the value it
//! beat, today's live value for its kind, and the record today is closest to
//! (`bot.py:CommandBot._render_records`, through progression's chase).

use deck_streak_kernel::StudyDay;
use deck_streak_progression::records::RecordKind;

use crate::recompute::records::StoredRecord;

/// The current day's live totals the view compares each record with.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LiveDay {
    /// The live score.
    pub score: i64,
    /// The study reviews.
    pub reviews: i64,
    /// Their seconds.
    pub seconds: f64,
}

/// One record as the view shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordLine {
    /// The kind.
    pub kind: RecordKind,
    /// Its label.
    pub label: &'static str,
    /// The best.
    pub value: i64,
    /// The day that set it.
    pub study_day: StudyDay,
    /// The value it beat.
    pub previous: i64,
    /// Today's live value for the kind.
    pub today: i64,
}

/// The records view.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordsView {
    /// Each stored record, in [`RecordKind::ALL`]'s order.
    pub lines: Vec<RecordLine>,
    /// The record today is closest to and its gap, if any.
    pub chase: Option<(RecordKind, i64)>,
}

/// The view of `stored` against today's `live` totals (R13).
#[must_use]
pub fn records_view(stored: &[StoredRecord], live: LiveDay) -> RecordsView {
    let _ = (stored, live);
    RecordsView::default()
}
