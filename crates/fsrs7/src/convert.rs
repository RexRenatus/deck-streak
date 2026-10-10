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

/// The kind of the engine's Forget row: a manual entry, told from a set-due-date entry by its
/// factor of 0 (SPEC-386 R2, R12).
pub const RESET_KIND: u8 = 4;

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
/// taken in id order, whatever order they arrive in; every row up to and including the card's
/// last reset is dropped (SPEC-386 R2), then manual and rescheduled entries and entries with no
/// rating (SPEC-342 R4). A card with no kept review is no item.
#[must_use]
pub fn histories(rows: &[RevlogRow]) -> Vec<CardHistory> {
    let mut cards: BTreeMap<i64, Vec<RevlogRow>> = BTreeMap::new();
    for row in rows {
        cards.entry(row.cid).or_default().push(*row);
    }
    cards
        .into_iter()
        .filter_map(|(cid, mut rows)| {
            rows.sort_by_key(|row| row.id);
            let since = rows.iter().rposition(reset).unwrap_or(0);
            let selected: Vec<&RevlogRow> = rows[since..].iter().filter(|row| kept(row)).collect();
            let last_id = selected.last()?.id;
            let mut previous = None;
            let reviews = selected
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
            Some(CardHistory {
                cid,
                last_id,
                item: FSRSItem { reviews },
            })
        })
        .collect()
}

/// Whether a row is the engine's Forget: a manual entry with the factor 0, which the engine's
/// set-due-date entry never carries (SPEC-386 R2, R12).
fn reset(row: &RevlogRow) -> bool {
    row.kind == RESET_KIND && row.factor == 0
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
