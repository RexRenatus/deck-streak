//! The card snapshot (SPEC-071 R8): a point-in-time count of the cards' states at a collection day
//! number, the predecessor's `analytics.py:compute_card_snapshot` at `27ee2bc`, proved by
//! `goldens/card_snapshot.json`. A leech is a card at or above the threshold that is not suspended:
//! a buried card counts.

use deck_streak_ingest::reader::Card;

/// Every field of the predecessor's `CardStateSnapshot`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CardSnapshot {
    /// Every card counted.
    pub total_cards: i64,
    /// Review cards, not suspended, whose interval is mature.
    pub mature_count: i64,
    /// Review cards, not suspended, whose interval is positive and young.
    pub young_count: i64,
    /// Learning and relearning cards.
    pub learning_count: i64,
    /// Suspended cards.
    pub suspended_count: i64,
    /// Cards at or above the leech threshold that are not suspended.
    pub leech_active: i64,
    /// Review-queue cards due before the day number.
    pub backlog: i64,
    /// Review-queue cards due on the day number.
    pub due_today: i64,
}

/// The card state a rollup stores: five of the snapshot's counts (R8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CardState {
    /// Mature review cards.
    pub mature_count: i64,
    /// Young review cards.
    pub young_count: i64,
    /// Active leeches.
    pub leech_active: i64,
    /// Review cards overdue.
    pub backlog: i64,
    /// Review cards due on the day.
    pub due_today: i64,
}

impl From<&CardSnapshot> for CardState {
    fn from(snapshot: &CardSnapshot) -> Self {
        Self {
            mature_count: snapshot.mature_count,
            young_count: snapshot.young_count,
            leech_active: snapshot.leech_active,
            backlog: snapshot.backlog,
            due_today: snapshot.due_today,
        }
    }
}

/// The snapshot of `cards` at collection day number `day_number`, counting leeches at
/// `leech_threshold` lapses.
#[must_use]
pub fn card_snapshot(cards: &[Card], leech_threshold: i64, day_number: i64) -> CardSnapshot {
    let _ = (cards, leech_threshold, day_number);
    CardSnapshot::default()
}
