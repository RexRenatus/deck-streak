//! Personal records (SPEC-073 R10, R13): the detection over a window of rollups, and the record to
//! chase.
//!
//! The detection equals `goldens/detect_records.json`: each kind's best over the rollups, and only
//! a value strictly above the stored best is a record. The record to chase equals
//! `goldens/records_chase.json`: the smallest positive gap from today, the first on a tie.

use std::collections::BTreeMap;

/// A record's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecordKind {
    /// The highest daily score.
    BestScore,
    /// The most study reviews in a day.
    MostReviews,
    /// The whole minutes of the most study seconds in a day.
    MostMinutes,
}

impl RecordKind {
    /// Every kind, in the predecessor's order.
    pub const ALL: [Self; 3] = [Self::BestScore, Self::MostReviews, Self::MostMinutes];

    /// The kind as `records.kind` stores it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BestScore => "best_score",
            Self::MostReviews => "most_reviews",
            Self::MostMinutes => "most_minutes",
        }
    }

    /// The kind `text` names, if any.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == text)
    }

    /// The label a screen shows.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::BestScore => "Best daily score",
            Self::MostReviews => "Most reviews in a day",
            Self::MostMinutes => "Most minutes in a day",
        }
    }
}

/// One rollup as the detection reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DayTotals {
    /// The day's score.
    pub score: i64,
    /// The day's study reviews.
    pub reviews: i64,
    /// The day's study seconds.
    pub seconds: f64,
}

/// A detected record: a kind and its new value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Detected {
    /// The kind.
    pub kind: RecordKind,
    /// The new value.
    pub value: i64,
}

/// Each kind whose best over `rollups` is strictly above its stored best in `previous` (a missing
/// best is 0), in [`RecordKind::ALL`]'s order (R10).
#[must_use]
pub fn detect_records(
    rollups: &[DayTotals],
    previous: &BTreeMap<RecordKind, i64>,
) -> Vec<Detected> {
    // An empty window sets no record, whatever is stored: the predecessor returns before its bests.
    let Some(best_score) = rollups.iter().map(|day| day.score).max() else {
        return Vec::new();
    };
    let most_reviews = rollups.iter().map(|day| day.reviews).max().unwrap_or(0);
    let most_minutes = rollups
        .iter()
        .map(|day| whole_minutes(day.seconds))
        .max()
        .unwrap_or(0);
    let bests = [best_score, most_reviews, most_minutes];
    RecordKind::ALL
        .into_iter()
        .zip(bests)
        .filter(|(kind, value)| *value > previous.get(kind).copied().unwrap_or(0))
        .map(|(kind, value)| Detected { kind, value })
        .collect()
}

/// The whole minutes of `seconds`, truncated toward zero as the predecessor's `int()` takes them.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a day's study seconds are far below i64::MAX minutes, and the predecessor truncates"
)]
fn whole_minutes(seconds: f64) -> i64 {
    (seconds / 60.0) as i64
}

/// One line of the records board as the record to chase reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardLine {
    /// The record's value.
    pub value: i64,
    /// Today's live value for its kind.
    pub today: i64,
}

/// The line today is closest to: the smallest positive gap from today to the record, the first on a
/// tie, as `(the line's index, the gap)`; `None` when no gap is positive (R13).
#[must_use]
pub fn chase(board: &[BoardLine]) -> Option<(usize, i64)> {
    let mut found: Option<(usize, i64)> = None;
    for (at, line) in board.iter().enumerate() {
        let gap = line.value - line.today;
        if gap > 0 && found.is_none_or(|(_, best)| gap < best) {
            found = Some((at, gap));
        }
    }
    found
}
