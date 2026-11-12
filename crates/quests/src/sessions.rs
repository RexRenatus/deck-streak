//! Study sessions, their effort and the effort floor that earns a chest (SPEC-081 R1).
//!
//! A session is a run of study reviews with no gap of ten minutes or more between consecutive
//! ones. The rules here are pure: the reviews and the per-answer time cap are inputs, because
//! this context reads no analytics and computes no XP (docs/CONTEXT-MAP.md).

use std::collections::BTreeSet;

use deck_streak_ingest::reader::{Review, is_study_event};
use deck_streak_kernel::UtcMillis;

/// A gap this long between two study reviews ends a session, in milliseconds.
pub const SESSION_GAP_MS: i64 = 600_000;
/// The distinct cards a session needs before it earns a chest.
pub const REAL_EFFORT_CHEST_MIN_DISTINCT: i64 = 15;

/// What a run of study reviews measures: their count, their distinct cards and their minutes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RealEffort {
    /// Study reviews counted.
    pub reviews: i64,
    /// Distinct cards among them.
    pub distinct_cards: i64,
    /// Summed answer time in minutes, each answer capped.
    pub minutes: f64,
}

/// One detected study session: its first and last review's instants and its effort.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Session {
    /// The first review's instant.
    pub start: UtcMillis,
    /// The last review's instant.
    pub end: UtcMillis,
    /// What the session measured.
    pub effort: RealEffort,
}

/// An integer as the predecessor's float: every value here is a count or a millisecond span far
/// below 2^53, so the conversion is exact.
#[allow(
    clippy::cast_precision_loss,
    reason = "counts and millisecond spans stay far below 2^53"
)]
const fn float(value: i64) -> f64 {
    value as f64
}

/// The `(start, end)` bounds the study reviews split into, without measuring effort. Both ends
/// are inclusive: the first and last answer's instants.
#[must_use]
pub fn session_bounds(reviews: &[Review]) -> Vec<(UtcMillis, UtcMillis)> {
    let mut times: Vec<i64> = reviews
        .iter()
        .filter(|review| is_study_event(review.kind, review.ease))
        .map(|review| review.id)
        .collect();
    times.sort_unstable();
    let Some(&first) = times.first() else {
        return Vec::new();
    };
    let mut bounds = Vec::new();
    let (mut start, mut previous) = (first, first);
    for &time in &times[1..] {
        if time - previous >= SESSION_GAP_MS {
            bounds.push((
                UtcMillis::from_epoch_millis(start),
                UtcMillis::from_epoch_millis(previous),
            ));
            start = time;
        }
        previous = time;
    }
    bounds.push((
        UtcMillis::from_epoch_millis(start),
        UtcMillis::from_epoch_millis(previous),
    ));
    bounds
}

/// The effort of the study reviews answered in `[start, end]`, each answer's seconds capped at
/// `answer_time_cap_seconds`.
#[must_use]
pub fn measure(
    reviews: &[Review],
    start: UtcMillis,
    end: UtcMillis,
    answer_time_cap_seconds: f64,
) -> RealEffort {
    let (start, end) = (start.epoch_millis(), end.epoch_millis());
    let mut count = 0_i64;
    let mut cards = BTreeSet::new();
    let mut seconds = 0.0_f64;
    for review in reviews {
        if !is_study_event(review.kind, review.ease) || review.id < start || review.id > end {
            continue;
        }
        count += 1;
        cards.insert(review.card_id);
        seconds += (float(review.taken_ms) / 1000.0).min(answer_time_cap_seconds);
    }
    RealEffort {
        reviews: count,
        distinct_cards: i64::try_from(cards.len()).unwrap_or(i64::MAX),
        minutes: seconds / 60.0,
    }
}

/// The sessions the study reviews split into, each with its measured effort.
#[must_use]
pub fn sessions_from_reviews(reviews: &[Review], answer_time_cap_seconds: f64) -> Vec<Session> {
    session_bounds(reviews)
        .into_iter()
        .map(|(start, end)| Session {
            start,
            end,
            effort: measure(reviews, start, end, answer_time_cap_seconds),
        })
        .collect()
}

/// The sessions that meet the effort floor, in their order.
#[must_use]
pub fn eligible_sessions(sessions: &[Session]) -> Vec<Session> {
    sessions
        .iter()
        .filter(|session| meets_chest_floor(&session.effort))
        .copied()
        .collect()
}

/// Whether a session's effort earns a chest.
#[must_use]
pub const fn meets_chest_floor(effort: &RealEffort) -> bool {
    effort.distinct_cards >= REAL_EFFORT_CHEST_MIN_DISTINCT
}
