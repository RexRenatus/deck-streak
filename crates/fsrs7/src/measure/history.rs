//! The synthetic history a replay is timed over (SPEC-342 R5): deterministic, with no randomness,
//! and read from no collection.

use crate::convert::RevlogRow;

/// The first card's id; each later card's is one more.
const FIRST_CARD: i64 = 1_000;
/// The rating cycle, by the review's position across the history: Good, Good, Hard, Good, Easy,
/// Again.
const EASES: [u8; 6] = [3, 3, 2, 3, 4, 1];
/// The hours between a card's consecutive reviews, cycled by the review's position in its card.
const GAP_HOURS: [i64; 5] = [24, 36, 72, 12, 168];
/// Milliseconds in an hour.
const HOUR_MS: i64 = 3_600_000;
/// A review-log row's kind for a review.
const REVIEW: u8 = 1;

/// The cards' lengths for `reviews` rows at a mean of `mean` reviews per card: the lengths cycle
/// through 1 to `2 * mean - 1`, whose mean is `mean`, and the last card is cut so the lengths sum
/// to exactly `reviews`.
#[must_use]
pub fn card_lengths(reviews: usize, mean: usize) -> Vec<usize> {
    let cycle = 2 * mean - 1;
    let mut lengths = Vec::new();
    let mut total = 0;
    for card in 0..reviews {
        if total == reviews {
            break;
        }
        let length = (card % cycle + 1).min(reviews - total);
        lengths.push(length);
        total += length;
    }
    lengths
}

/// The history's review-log rows: one card per length of [`card_lengths`], in turn. Each row's id
/// is the one before's plus the gap of [`GAP_HOURS`] at the earlier row's position in its card,
/// so every card's reviews are in id order, and the ratings follow [`EASES`] across the whole
/// history.
#[must_use]
pub fn rows(reviews: usize, mean: usize) -> Vec<RevlogRow> {
    let mut rows = Vec::with_capacity(reviews);
    let mut at = 0;
    for (cid, length) in (FIRST_CARD..).zip(card_lengths(reviews, mean)) {
        for review in 0..length {
            rows.push(RevlogRow {
                cid,
                id: at,
                ease: EASES[rows.len() % EASES.len()],
                kind: REVIEW,
                factor: 0,
            });
            at += GAP_HOURS[review % GAP_HOURS.len()] * HOUR_MS;
        }
    }
    rows
}
