//! The card health panel's record (SPEC-507, ADR-525): one card's counters, its review rows with each
//! answer's time capped, the scheduler's readout or the state that says why there is none, and where
//! the card came from.
//!
//! The record holds integers and closed enums only. It holds no stored FSRS memory state, so no
//! figure the scheduler did not give can stand in for its estimate (ADR-525 D5). The dispatcher
//! fills it from its two fixed reads ([`crate::dispatch::Dispatcher::card_health`]); this module
//! reads nothing itself.

use serde_json::{Value, json};

/// The most milliseconds one answer counts: the analytics crate's 60-second cap (ADR-525 D2).
pub const ANSWER_CAP_MS: u32 = 60_000;
/// The most review rows the record carries, newest first.
pub const ROWS_SHOWN: usize = 200;

/// A card as the snapshot read selects it: `[id, queue, type, due, ivl, reps, lapses]`.
pub type SnapshotRow = [i64; 7];
/// A review row as the card reviews read selects it: `[id, ease, time, type, ivl]`.
pub type ReviewRow = [i64; 5];

/// Why the scheduler gives no readout of a card (SPEC-507 R6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerState {
    /// This build has no readout on the scheduler's port.
    Absent,
    /// The scheduler is a fixture, whose figures are not the learner's.
    Fixture,
    /// The scheduler has not read this card's history yet.
    Unready,
    /// The scheduler's state is older than the card's last review.
    Stale,
}

impl SchedulerState {
    /// Every state but Ready, in order.
    pub const ALL: [Self; 4] = [Self::Absent, Self::Fixture, Self::Unready, Self::Stale];

    /// The state's word in the record's JSON.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Fixture => "fixture",
            Self::Unready => "unready",
            Self::Stale => "stale",
        }
    }
}

/// The scheduler's readout of one card, or the state that says why there is none (ADR-525 D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readout {
    /// The scheduler's own estimate.
    Ready {
        /// The chance of recall now, in parts per million.
        recall_ppm: u32,
        /// The days until the scheduler's recall estimate falls to 90%.
        days_to_90: u32,
    },
    /// No estimate, for the reason the state names.
    NotAvailable(SchedulerState),
}

/// Where the card came from (SPEC-507 R7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Through the collection, created at the instant its id records.
    Collection {
        /// The card's created instant, in epoch milliseconds.
        created_ms: i64,
    },
}

/// One review row as the panel shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthReview {
    /// The row's instant, in epoch milliseconds (its id).
    pub at_ms: i64,
    /// The button pressed, 1 to 4, or 0 for an entry set by hand.
    pub grade: u8,
    /// The row's type: 0 learn, 1 review, 2 relearn, 3 filtered, 4 manual, 5 rescheduled.
    pub kind: u8,
    /// The interval after the row: days when positive, seconds when negative.
    pub interval: i32,
    /// The row's time, capped at [`ANSWER_CAP_MS`], a negative time read as 0.
    pub answer_ms_capped: u32,
}

/// What the card's review rows count (SPEC-507 R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Logged {
    /// The rows that are answers: type 0 to 3 with a button of 1 or more.
    pub answers: u32,
    /// The answers of type 1 (review) with button 1 (Again).
    pub lapses: u32,
}

/// One card's health, as the panel shows it (SPEC-507 R2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardHealth {
    /// The card's id.
    pub card: i64,
    /// The card's repetitions counter.
    pub reps: u32,
    /// The card's lapses counter.
    pub lapses: u32,
    /// The counts over the card's review rows.
    pub logged: Logged,
    /// How many review rows the card has.
    pub rows: u32,
    /// The newest [`ROWS_SHOWN`] review rows, newest first.
    pub reviews: Vec<HealthReview>,
    /// The sum of the answers' capped times, in milliseconds.
    pub answer_ms_total: u64,
    /// The answers' capped mean, rounded half up, or `None` with no answer.
    pub answer_ms_mean: Option<u32>,
    /// The answers' capped median, or `None` with no answer.
    pub answer_ms_median: Option<u32>,
    /// The scheduler's readout, or why there is none.
    pub scheduler: Readout,
    /// Where the card came from.
    pub provenance: Provenance,
}

/// Whether a row of `kind` with button `ease` is an answer, as the analytics crate's study event is.
#[must_use]
pub const fn is_answer(kind: i64, ease: i64) -> bool {
    matches!(kind, 0..=3) && ease >= 1
}

/// The review row's columns, as the card reviews read selects them.
const ID: usize = 0;
const EASE: usize = 1;
const TIME: usize = 2;
const KIND: usize = 3;

/// `time_ms` capped at [`ANSWER_CAP_MS`], a negative time read as 0.
#[must_use]
pub fn capped(time_ms: i64) -> u32 {
    u32::try_from(time_ms.clamp(0, i64::from(ANSWER_CAP_MS))).unwrap_or(ANSWER_CAP_MS)
}

/// The health of the card `snapshot` reads, over its review `rows` oldest first, with the
/// scheduler's `readout` (SPEC-507 R2 to R7).
#[must_use]
pub fn card_health(snapshot: SnapshotRow, rows: &[ReviewRow], scheduler: Readout) -> CardHealth {
    let [card, _queue, _kind, _due, _interval, reps, lapses] = snapshot;
    let answers: Vec<&ReviewRow> = rows
        .iter()
        .filter(|row| is_answer(row[KIND], row[EASE]))
        .collect();
    let logged_lapses = answers
        .iter()
        .filter(|row| row[KIND] == 1 && row[EASE] == 1)
        .count();
    let mut times: Vec<u32> = answers.iter().map(|row| capped(row[TIME])).collect();
    let total: u64 = times.iter().copied().map(u64::from).sum();
    times.sort_unstable();
    let count = u64::try_from(times.len()).unwrap_or(u64::MAX);
    let mean =
        (count > 0).then(|| u32::try_from((total + count / 2) / count).unwrap_or(ANSWER_CAP_MS));
    CardHealth {
        card,
        reps: counter(reps),
        lapses: counter(lapses),
        logged: Logged {
            answers: tally(answers.len()),
            lapses: tally(logged_lapses),
        },
        rows: tally(rows.len()),
        reviews: rows.iter().rev().take(ROWS_SHOWN).map(review).collect(),
        answer_ms_total: total,
        answer_ms_mean: mean,
        answer_ms_median: median(&times),
        scheduler,
        provenance: Provenance::Collection { created_ms: card },
    }
}

/// A counter the card stores, a negative one read as 0.
fn counter(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}

/// A count of rows.
fn tally(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// One review row as the panel shows it.
fn review(row: &ReviewRow) -> HealthReview {
    let [_, ease, time, kind, interval] = *row;
    HealthReview {
        at_ms: row[ID],
        grade: u8::try_from(ease.clamp(0, i64::from(u8::MAX))).unwrap_or(u8::MAX),
        kind: u8::try_from(kind.clamp(0, i64::from(u8::MAX))).unwrap_or(u8::MAX),
        interval: i32::try_from(interval.clamp(i64::from(i32::MIN), i64::from(i32::MAX)))
            .unwrap_or_default(),
        answer_ms_capped: capped(time),
    }
}

/// The middle of `sorted`, or the floor of its two middle values' mean; `None` when it is empty.
fn median(sorted: &[u32]) -> Option<u32> {
    let middle = sorted.len() / 2;
    let high = *sorted.get(middle)?;
    if sorted.len() % 2 == 1 {
        return Some(high);
    }
    let low = *sorted.get(middle.checked_sub(1)?)?;
    Some(low.midpoint(high))
}

impl CardHealth {
    /// The record as the web adapter answers it: integers, closed words and `null`s only.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let scheduler = match self.scheduler {
            Readout::Ready {
                recall_ppm,
                days_to_90,
            } => json!({ "state": "ready", "recall_ppm": recall_ppm, "days_to_90": days_to_90 }),
            Readout::NotAvailable(state) => json!({ "state": state.word() }),
        };
        let Provenance::Collection { created_ms } = self.provenance;
        let reviews: Vec<Value> = self
            .reviews
            .iter()
            .map(|review| {
                json!({
                    "at": review.at_ms,
                    "grade": review.grade,
                    "kind": review.kind,
                    "interval": review.interval,
                    "ms": review.answer_ms_capped,
                })
            })
            .collect();
        json!({
            "card": self.card,
            "reps": self.reps,
            "lapses": self.lapses,
            "logged": { "answers": self.logged.answers, "lapses": self.logged.lapses },
            "rows": self.rows,
            "reviews": reviews,
            "total_ms": self.answer_ms_total,
            "mean_ms": self.answer_ms_mean,
            "median_ms": self.answer_ms_median,
            "scheduler": scheduler,
            "provenance": { "source": "collection", "created_ms": created_ms },
        })
    }
}
