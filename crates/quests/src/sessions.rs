//! Study sessions, their effort and the effort floor that earns a chest (SPEC-081 R1).
//!
//! A session is a run of study reviews with no gap of ten minutes or more between consecutive
//! ones. The rules here are pure: the reviews and the per-answer time cap are inputs, because
//! this context reads no analytics and computes no XP (docs/CONTEXT-MAP.md).

#![allow(unused_variables)]

use deck_streak_ingest::reader::Review;
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

/// The `(start, end)` bounds the study reviews split into, without measuring effort.
#[must_use]
pub fn session_bounds(reviews: &[Review]) -> Vec<(UtcMillis, UtcMillis)> {
    Vec::new()
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
    RealEffort {
        reviews: 0,
        distinct_cards: 0,
        minutes: 0.0,
    }
}

/// The sessions the study reviews split into, each with its measured effort.
#[must_use]
pub fn sessions_from_reviews(reviews: &[Review], answer_time_cap_seconds: f64) -> Vec<Session> {
    Vec::new()
}

/// The sessions that meet the effort floor, in their order.
#[must_use]
pub fn eligible_sessions(sessions: &[Session]) -> Vec<Session> {
    Vec::new()
}

/// Whether a session's effort earns a chest.
#[must_use]
pub const fn meets_chest_floor(effort: &RealEffort) -> bool {
    false
}
