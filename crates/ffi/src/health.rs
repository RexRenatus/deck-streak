//! The card health panel as a native client shows it (SPEC-507 part b, R12 and R13; ADR-525).
//!
//! [`crate::engine::Engine::card_health`] answers the core's record of one card, field for field, as
//! [`CardHealth`]: integers and closed enums only, with no stored FSRS memory state (ADR-525 D5).
//! [`card_health_lines`] words the record in the sections the app's sheet shows as they are, so the
//! app adds no word and no decision of its own (SPEC-347 R11): the recall rows show a number only
//! from the scheduler's Ready readout and every other state reads its own line (SPEC-507 R6), and no
//! line judges the card (R8).

use deck_streak_engine_core::health::{self, Provenance, Readout, SchedulerState};

/// Why the scheduler gives no readout of a card (SPEC-507 R6), one for one with the core's states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HealthState {
    /// This build has no readout on the scheduler's port.
    Absent,
    /// The scheduler is a fixture, whose figures are not the learner's.
    Fixture,
    /// The scheduler has not read this card's history yet.
    Unready,
    /// The scheduler's state is older than the card's last review.
    Stale,
}

/// The scheduler's readout of one card, or the state that says why there is none (ADR-525 D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HealthReadout {
    /// The scheduler's own estimate.
    Ready {
        /// The chance of recall now, in parts per million.
        recall_ppm: u32,
        /// The days until the scheduler's recall estimate falls to 90%.
        days_to_90: u32,
    },
    /// No estimate, for the reason the state names.
    NotAvailable {
        /// Why there is none.
        state: HealthState,
    },
}

/// Where the card came from (SPEC-507 R7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum HealthProvenance {
    /// Through the collection, created at the instant its id records.
    Collection {
        /// The card's created instant, in epoch milliseconds.
        created_ms: i64,
    },
}

/// What the card's review rows count (SPEC-507 R5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct HealthLogged {
    /// The rows that are answers: type 0 to 3 with a button of 1 or more.
    pub answers: u32,
    /// The answers of type 1 (review) with button 1 (Again).
    pub lapses: u32,
}

/// One review row as the panel shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct HealthReview {
    /// The row's instant, in epoch milliseconds (its id).
    pub at_ms: i64,
    /// The button pressed, 1 to 4, or 0 for an entry set by hand.
    pub grade: u8,
    /// The row's type: 0 learn, 1 review, 2 relearn, 3 filtered, 4 manual, 5 rescheduled.
    pub kind: u8,
    /// The interval after the row: days when positive, seconds when negative.
    pub interval: i32,
    /// The row's time, capped at 60,000 ms, a negative time read as 0.
    pub answer_ms_capped: u32,
}

/// One card's health as a native client reads it (SPEC-507 R2, R12): the core's record, field for
/// field.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CardHealth {
    /// The card's id.
    pub card: i64,
    /// The card's repetitions counter.
    pub reps: u32,
    /// The card's lapses counter.
    pub lapses: u32,
    /// The counts over the card's review rows.
    pub logged: HealthLogged,
    /// How many review rows the card has.
    pub rows: u32,
    /// The newest 200 review rows, newest first.
    pub reviews: Vec<HealthReview>,
    /// The sum of the answers' capped times, in milliseconds.
    pub answer_ms_total: u64,
    /// The answers' capped mean, rounded half up, or `None` with no answer.
    pub answer_ms_mean: Option<u32>,
    /// The answers' capped median, or `None` with no answer.
    pub answer_ms_median: Option<u32>,
    /// The scheduler's readout, or why there is none.
    pub scheduler: HealthReadout,
    /// Where the card came from.
    pub provenance: HealthProvenance,
}

impl From<health::CardHealth> for CardHealth {
    fn from(record: health::CardHealth) -> Self {
        let Provenance::Collection { created_ms } = record.provenance;
        Self {
            card: record.card,
            reps: record.reps,
            lapses: record.lapses,
            logged: HealthLogged {
                answers: record.logged.answers,
                lapses: record.logged.lapses,
            },
            rows: record.rows,
            reviews: record
                .reviews
                .iter()
                .map(|review| HealthReview {
                    at_ms: review.at_ms,
                    grade: review.grade,
                    kind: review.kind,
                    interval: review.interval,
                    answer_ms_capped: review.answer_ms_capped,
                })
                .collect(),
            answer_ms_total: record.answer_ms_total,
            answer_ms_mean: record.answer_ms_mean,
            answer_ms_median: record.answer_ms_median,
            scheduler: readout(record.scheduler),
            provenance: HealthProvenance::Collection { created_ms },
        }
    }
}

/// The core's readout as a native client reads it, each state by its own variant.
const fn readout(readout: Readout) -> HealthReadout {
    match readout {
        Readout::Ready {
            recall_ppm,
            days_to_90,
        } => HealthReadout::Ready {
            recall_ppm,
            days_to_90,
        },
        Readout::NotAvailable(state) => HealthReadout::NotAvailable {
            state: match state {
                SchedulerState::Absent => HealthState::Absent,
                SchedulerState::Fixture => HealthState::Fixture,
                SchedulerState::Unready => HealthState::Unready,
                SchedulerState::Stale => HealthState::Stale,
            },
        },
    }
}

/// One section of the panel: its heading, and its lines, each shown and read by VoiceOver as it
/// is (SPEC-507 R13, R16).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct HealthSection {
    /// The section's heading.
    pub title: String,
    /// The section's lines, in order.
    pub lines: Vec<String>,
}

/// The panel's sections for `health`, in the words the app shows as they are (SPEC-507 R13):
/// the counts and times, the recall rows, where the card came from, and its newest rows when it
/// has any. With no record, one section says the record could not be read.
#[uniffi::export]
#[must_use]
pub fn card_health_lines(health: Option<CardHealth>) -> Vec<HealthSection> {
    let Some(health) = health else {
        return vec![section(UNREADABLE_TITLE, vec![UNREADABLE.to_owned()])];
    };
    let HealthProvenance::Collection { created_ms } = health.provenance;
    let mut sections = vec![
        section(COUNTS, counts(&health)),
        section(RECALL, recall(health.scheduler)),
        section(
            ORIGIN,
            vec![format!(
                "Came through your collection, created {}.",
                date(created_ms)
            )],
        ),
    ];
    if !health.reviews.is_empty() {
        sections.push(section(HISTORY, history(&health)));
    }
    sections
}

/// The heading of a panel whose record could not be read, and its one line.
const UNREADABLE_TITLE: &str = "Card info";
const UNREADABLE: &str = "This card's record could not be read.";
/// The sections' headings, in the panel's order.
const COUNTS: &str = "Counts and times";
const RECALL: &str = "Recall";
const ORIGIN: &str = "Where it came from";
const HISTORY: &str = "Newest reviews";
/// The labels of the counts and times, the web panel's own.
const REVIEWS: &str = "Reviews";
const LAPSES: &str = "Lapses";
const TIME_TOTAL: &str = "Time answering, each answer at most 60 s";
const TIME_MEAN: &str = "Mean answer time";
const TIME_MEDIAN: &str = "Median answer time";
const NO_ANSWERS: &str = "No answers yet";
/// The label over the scheduler's own estimate (SPEC-507 R6).
const ESTIMATES: &str = "The scheduler estimates";
/// The name of the button a review row records, by its number: 0 is an entry set by hand, and 1 to
/// 4 are the review's own answer names, which record what was pressed (SPEC-507 R8).
const ANSWER_NAMES: [&str; 5] = ["Set by hand", "Again", "Hard", "Good", "Easy"];
/// The milliseconds in a day.
const DAY_MS: i64 = 86_400_000;

/// A section of `title` holding `lines`.
fn section(title: &str, lines: Vec<String>) -> HealthSection {
    HealthSection {
        title: title.to_owned(),
        lines,
    }
}

/// The counts and times: the card's counters, its answers' capped times, and the log's counts when
/// they differ from the counters, as a fact (SPEC-507 R3, R5).
fn counts(health: &CardHealth) -> Vec<String> {
    let mut lines = vec![
        format!("{REVIEWS}: {}", health.reps),
        format!("{LAPSES}: {}", health.lapses),
        format!("{TIME_TOTAL}: {}", seconds(health.answer_ms_total)),
        format!("{TIME_MEAN}: {}", answer_time(health.answer_ms_mean)),
        format!("{TIME_MEDIAN}: {}", answer_time(health.answer_ms_median)),
    ];
    let HealthLogged { answers, lapses } = health.logged;
    if answers != health.reps || lapses != health.lapses {
        lines.push(format!(
            "The review log holds {answers} answers and {lapses} lapses."
        ));
    }
    lines
}

/// The recall rows: under the scheduler's label, its two figures when it is Ready; the state's
/// line alone, with no label and no number, in every other state (SPEC-507 R6).
fn recall(readout: HealthReadout) -> Vec<String> {
    match readout {
        HealthReadout::Ready {
            recall_ppm,
            days_to_90,
        } => vec![
            ESTIMATES.to_owned(),
            format!(
                "Chance of recall now: {}%",
                tenths(u64::from(recall_ppm), 1_000)
            ),
            format!("Days until recall falls to 90%: {days_to_90}"),
        ],
        HealthReadout::NotAvailable { state } => vec![state_line(state).to_owned()],
    }
}

/// The line the recall rows read in `state`, the web panel's own.
const fn state_line(state: HealthState) -> &'static str {
    match state {
        HealthState::Absent => "Not available in this build.",
        HealthState::Fixture => "Not available: the scheduler is a test fixture.",
        HealthState::Unready => "Not available: the scheduler has not read this card yet.",
        HealthState::Stale => {
            "Not available: the scheduler's state is older than this card's last review."
        }
    }
}

/// The newest rows, newest first, each as its date, the name of its button and its capped time,
/// after how many of how many when the record holds fewer rows than the card has (SPEC-507 R2).
fn history(health: &CardHealth) -> Vec<String> {
    let shown = u32::try_from(health.reviews.len()).unwrap_or(u32::MAX);
    let mut lines = Vec::new();
    if health.rows > shown {
        lines.push(format!("The newest {shown} of {} reviews.", health.rows));
    }
    lines.extend(health.reviews.iter().map(|review| {
        format!(
            "{}, {}, {}",
            date(review.at_ms),
            answer_name(review.grade),
            seconds(u64::from(review.answer_ms_capped))
        )
    }));
    lines
}

/// The name of the button `grade`, or its number when it has none.
fn answer_name(grade: u8) -> String {
    ANSWER_NAMES
        .get(usize::from(grade))
        .map_or_else(|| grade.to_string(), |name| (*name).to_owned())
}

/// An answer time in seconds, or that there is no answer yet.
fn answer_time(ms: Option<u32>) -> String {
    ms.map_or_else(|| NO_ANSWERS.to_owned(), |ms| seconds(u64::from(ms)))
}

/// `ms` as seconds to one decimal, rounded half up: 21,667 ms is `21.7 s`.
fn seconds(ms: u64) -> String {
    format!("{} s", tenths(ms, 100))
}

/// `value` counted in tenths of `per_tenth`, rounded half up, written with one decimal:
/// `tenths(912_345, 1_000)` is `91.2`.
fn tenths(value: u64, per_tenth: u64) -> String {
    let tenths = value.saturating_add(per_tenth / 2) / per_tenth;
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// The UTC date of the epoch millisecond `ms`, as `yyyy-mm-dd`, by the civil-from-days algorithm
/// over 400-year eras of 146,097 days, each year counted from 1 March.
fn date(ms: i64) -> String {
    let shifted = ms.div_euclid(DAY_MS) + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1_460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let from_march = (5 * of_year + 2) / 153;
    let day = of_year - (153 * from_march + 2) / 5 + 1;
    let month = if from_march < 10 {
        from_march + 3
    } else {
        from_march - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
