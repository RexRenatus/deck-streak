//! The records view (SPEC-073 R13): each stored record with its label, value, day and the value it
//! beat, today's live value for its kind, and the record today is closest to
//! (`bot.py:CommandBot._render_records`, through progression's chase).

use deck_streak_kernel::StudyDay;
use deck_streak_progression::records::{BoardLine, RecordKind, chase};

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
    let lines: Vec<RecordLine> = stored
        .iter()
        .map(|record| RecordLine {
            kind: record.kind,
            label: record.kind.label(),
            value: record.value,
            study_day: record.study_day,
            previous: record.previous,
            today: today(record.kind, live),
        })
        .collect();
    let board: Vec<BoardLine> = lines
        .iter()
        .map(|line| BoardLine {
            value: line.value,
            today: line.today,
        })
        .collect();
    let chase = chase(&board).and_then(|(at, gap)| lines.get(at).map(|line| (line.kind, gap)));
    RecordsView { lines, chase }
}

/// Today's live value for `kind`: the score, the study reviews, or the whole minutes of their
/// seconds.
fn today(kind: RecordKind, live: LiveDay) -> i64 {
    match kind {
        RecordKind::BestScore => live.score,
        RecordKind::MostReviews => live.reviews,
        RecordKind::MostMinutes => whole_minutes(live.seconds),
    }
}

/// The whole minutes of `seconds`, truncated toward zero as the records' detection takes them.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a day's study seconds are far below i64::MAX minutes, and the predecessor truncates"
)]
fn whole_minutes(seconds: f64) -> i64 {
    (seconds / 60.0) as i64
}
