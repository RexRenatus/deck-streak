//! Whether a review cannot count toward the streak for its card's due day (SPEC-376 R1 to R3,
//! ADR-387 D1): a card shown after the engine's day it was due on is past its due day. The rule
//! judges in the engine's own day, the day count and the next rollover the core reads from the
//! engine, so it reads no clock and agrees with the queue the engine built.

use anki_proto::cards::Card;

/// The engine's intraday learning queue, as its `CardQueue` numbers it: the due is an instant.
const LEARN: i32 = 1;
/// The engine's review queue: the due is a day of the collection.
const REVIEW: i32 = 2;
/// The engine's day-learning queue: the due is a day of the collection.
const DAY_LEARN: i32 = 3;
/// One day, in seconds: the engine's day began this long before its next rollover.
const ONE_DAY: i64 = 86_400;

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

/// Whether `card` is past its due day in the engine's `day` (SPEC-376 R2). A card in the review
/// or the day-learning queue is when its due day is earlier than the engine's day; one in the
/// intraday learning queue is when its due instant is earlier than the instant the engine's day
/// began; no other card ever is. A card in a filtered deck is judged by the due it keeps for its
/// home deck when that due is set, as the engine reads its state, and never by its position there.
#[must_use]
pub fn past_due_day(card: &Card, day: EngineDay) -> bool {
    let due = if card.original_deck_id != 0 && card.original_due != 0 {
        card.original_due
    } else {
        card.due
    };
    match card.queue {
        REVIEW | DAY_LEARN => i64::from(due) < i64::from(day.days_elapsed),
        LEARN => i64::from(due) < day.next_day_at - ONE_DAY,
        _ => false,
    }
}
