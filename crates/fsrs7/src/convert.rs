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
    /// The ease factor the engine logged, in thousandths.
    pub factor: u32,
}

/// One card's kept reviews, as one FSRS-7 item.
#[derive(Clone, Debug, PartialEq)]
pub struct CardHistory {
    /// The card.
    pub cid: i64,
    /// The id of the card's last kept review.
    pub last_id: i64,
    /// Its reviews in id order: the first at an interval of 0, each later one at its distance
    /// from the one before, in days.
    pub item: FSRSItem,
}

/// Each card's kept reviews as one FSRS-7 item, the cards in id order. Within a card the rows are
/// taken in id order, whatever order they arrive in; manual and rescheduled entries, and entries
/// with no rating, are dropped (SPEC-342 R4).
#[must_use]
pub fn histories(rows: &[RevlogRow]) -> Vec<CardHistory> {
    let mut cards: BTreeMap<i64, Vec<RevlogRow>> = BTreeMap::new();
    for row in rows.iter().filter(|row| kept(row)) {
        cards.entry(row.cid).or_default().push(*row);
    }
    cards
        .into_iter()
        .map(|(cid, mut reviews)| {
            reviews.sort_by_key(|row| row.id);
            let mut previous = None;
            let reviews = reviews
                .iter()
                .map(|row| {
                    let delta_t = previous.map_or(0.0, |at| days(row.id - at));
                    previous = Some(row.id);
                    FSRSReview {
                        rating: u32::from(row.ease),
                        delta_t,
                    }
                })
                .collect();
            CardHistory {
                cid,
                last_id: 0,
                item: FSRSItem { reviews },
            }
        })
        .collect()
}

/// Whether a row is a rated review of the card's memory.
fn kept(row: &RevlogRow) -> bool {
    row.ease != 0 && !DROPPED_KINDS.contains(&row.kind)
}

/// An interval in milliseconds, as days.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "FSRS-7 reads an interval as an f32 number of days; an interval of a collection's \
              lifetime is exact in an f64 and a few days' rounding in an f32 is below its own"
)]
fn days(elapsed_ms: i64) -> f32 {
    (elapsed_ms as f64 / MS_PER_DAY) as f32
}
