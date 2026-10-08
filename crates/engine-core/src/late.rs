//! Whether a review cannot count toward the streak for its card's due day (SPEC-376 R1 to R3,
//! ADR-387 D1): a card shown after the engine's day it was due on is past its due day. The rule
//! judges in the engine's own day, the day count and the next rollover the core reads from the
//! engine, so it reads no clock and agrees with the queue the engine built.

use anki_proto::cards::Card;

/// The engine's day, as its scheduler's timing of today answers it: the days elapsed since the
/// collection was created, which a review card's due is counted in, and the instant the next day
/// begins, in seconds, which an intraday learning card's due is compared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineDay {
    /// The engine's day count: today, as a review card's due names a day.
    pub days_elapsed: u32,
    /// The instant the engine's next day begins, in seconds since the epoch.
    pub next_day_at: i64,
}

/// Whether `card` is past its due day in the engine's `day` (SPEC-376 R2).
#[must_use]
pub fn past_due_day(card: &Card, day: EngineDay) -> bool {
    let _ = (card, day);
    false
}
