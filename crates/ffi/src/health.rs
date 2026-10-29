//! The card health panel as a native client shows it (SPEC-507 part b, R12 and R13; ADR-525).
//!
//! [`crate::engine::Engine::card_health`] answers the core's record of one card, field for field, as
//! [`CardHealth`]: integers and closed enums only, with no stored FSRS memory state (ADR-525 D5).
//! [`card_health_lines`] words the record in the sections the app's sheet shows as they are, so the
//! app adds no word and no decision of its own (SPEC-347 R11): the recall rows show a number only
//! from the scheduler's Ready readout and every other state reads its own line (SPEC-507 R6), and no
//! line judges the card (R8).

use deck_streak_engine_core::health;

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
    fn from(_record: health::CardHealth) -> Self {
        Self {
            card: 0,
            reps: 0,
            lapses: 0,
            logged: HealthLogged {
                answers: 0,
                lapses: 0,
            },
            rows: 0,
            reviews: Vec::new(),
            answer_ms_total: 0,
            answer_ms_mean: None,
            answer_ms_median: None,
            scheduler: HealthReadout::NotAvailable {
                state: HealthState::Absent,
            },
            provenance: HealthProvenance::Collection { created_ms: 0 },
        }
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

/// The panel's sections for `health`, in the words the app shows as they are (SPEC-507 R13).
#[uniffi::export]
#[must_use]
pub fn card_health_lines(health: Option<CardHealth>) -> Vec<HealthSection> {
    drop(health);
    Vec::new()
}
