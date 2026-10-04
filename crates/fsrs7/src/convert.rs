//! Review-log rows into FSRS-7 items (SPEC-342 R4).

use std::collections::BTreeMap;

use fsrs7::{FSRSItem, FSRSReview};

/// Milliseconds in a day. A review's interval is the difference of two review-log ids, which are
/// the answers' times in milliseconds, divided by this: a fractional number of days, as FSRS-7
/// reads it.
pub const MS_PER_DAY: f64 = 86_400_000.0;

/// The kinds of review-log entry that are no review of the card's memory: a manual entry (4) and
/// a reschedule (5), as the engine's review-log table writes them.
pub const DROPPED_KINDS: [u8; 2] = [4, 5];

/// One review-log row, shaped as the engine's table holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RevlogRow {
    /// The card the row belongs to.
    pub cid: i64,
    /// The row's id: the answer's time, in milliseconds.
    pub id: i64,
    /// The button pressed, 1 to 4; 0 for an entry with no rating.
    pub ease: u8,
    /// The entry's kind; 4 is a manual entry and 5 a reschedule.
    pub kind: u8,
}

/// One card's kept reviews, as one FSRS-7 item.
#[derive(Clone, Debug, PartialEq)]
pub struct CardHistory {
    /// The card.
    pub cid: i64,
    /// Its reviews in id order: the first at an interval of 0, each later one at its distance
    /// from the one before, in days.
    pub item: FSRSItem,
}

/// Each card's kept reviews as one FSRS-7 item, the cards in id order. Within a card the rows are
/// taken in id order, whatever order they arrive in; manual and rescheduled entries, and entries
/// with no rating, are dropped (SPEC-342 R4).
#[must_use]
pub fn histories(rows: &[RevlogRow]) -> Vec<CardHistory> {
    let mut cards: BTreeMap<i64, Vec<FSRSReview>> = BTreeMap::new();
    for row in rows {
        cards.entry(row.cid).or_default().push(FSRSReview {
            rating: u32::from(row.ease),
            delta_t: 0.0,
        });
    }
    cards
        .into_iter()
        .map(|(cid, reviews)| CardHistory {
            cid,
            item: FSRSItem { reviews },
        })
        .collect()
}
