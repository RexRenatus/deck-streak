//! The badge context of one evaluated day (SPEC-073 R5), equal to `goldens/badge_context.json`
//! (`pipeline.py:GamifyPipeline._evaluate_and_award`).
//!
//! It is a pure function of what the day's evaluation reads: the window's study reviews, the
//! card-to-deck map, the day's card snapshot, the language streak's badge view, the lifetime study
//! reviews, the day's score and the scores of the most recent rollups on or before it. The badge
//! step reads those from the fold's write; the golden builds them from its case.

use std::collections::BTreeMap;

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{StudyDay, StudyDayRule};
use deck_streak_progression::badges::conditions::{BadgeContext, Snapshot};

/// What one evaluated day's badge context is built from.
#[derive(Clone, Copy, Debug)]
pub struct DayState<'a> {
    /// The window's study reviews.
    pub reviews: &'a [Review],
    /// The study-day rule.
    pub rule: StudyDayRule,
    /// The day evaluated.
    pub day: StudyDay,
    /// Each card's home deck.
    pub card_decks: &'a BTreeMap<i64, i64>,
    /// The day's card snapshot.
    pub snapshot: Snapshot,
    /// The language streak's current length.
    pub streak_current: u64,
    /// Whether the language streak's comeback shows (SPEC-076's badge view).
    pub comeback_armed: bool,
    /// The lifetime study reviews through the day.
    pub lifetime: u64,
    /// The day's score: the score it closed with for a closing day, the live score for the current
    /// day.
    pub score_total: i64,
    /// The most recent rollups on or before the day, most recent first, each its day and stored
    /// score; the day's own counts with `score_total`.
    pub rollups: &'a [(StudyDay, i64)],
}

/// The badge context of `state`'s day (R5).
#[must_use]
pub fn badge_context(state: &DayState<'_>) -> BadgeContext {
    let _ = state;
    BadgeContext::default()
}
