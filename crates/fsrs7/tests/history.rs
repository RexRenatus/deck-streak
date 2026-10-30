//! SPEC-342 A7 (R5): the synthetic history holds exactly a cell's review rows, at its mean card
//! length. The lengths and rows below are typed by hand from R5's rule, never printed by the crate.

use deck_streak_fsrs7::convert::RevlogRow;
use deck_streak_fsrs7::measure::history::{card_lengths, rows};
use deck_streak_fsrs7::measure::{MEANS, REVIEWS};

/// Milliseconds in an hour.
const HOUR: i64 = 3_600_000;
/// A review-log row's kind for a review.
const REVIEW: u8 = 1;
/// The ease factor every generated row logs, in thousandths: never 0, so no row is a reset.
const FACTOR: u32 = 2500;

#[test]
fn a_cell_holds_exactly_its_review_rows_at_its_mean_length() {
    // Ten rows at a mean of 2: the lengths cycle through 1 to 3, and the fourth cycle is cut at 1.
    assert_eq!(card_lengths(10, 2), vec![1, 2, 3, 1, 2, 1]);

    // The same cell's rows as (card, hours from the first answer, ease): the gaps cycle through
    // 24, 36, 72, 12 and 168 hours within a card, and the eases through 3, 3, 2, 3, 4 and 1
    // across the history.
    let golden: Vec<RevlogRow> = [
        (1000, 0, 3),
        (1001, 24, 3),
        (1001, 48, 2),
        (1002, 84, 3),
        (1002, 108, 4),
        (1002, 144, 1),
        (1003, 216, 3),
        (1004, 240, 3),
        (1004, 264, 2),
        (1005, 300, 3),
    ]
    .into_iter()
    .map(|(cid, hours, ease)| RevlogRow {
        cid,
        id: hours * HOUR,
        ease,
        kind: REVIEW,
        factor: FACTOR,
    })
    .collect();
    assert_eq!(rows(10, 2), golden);

    // Every cell of the grid sums to exactly its rows, with its card count within one cut card of
    // rows / mean, and every length in 1 to 2m - 1.
    let mut cells = 0;
    for reviews in REVIEWS {
        for mean in MEANS {
            let lengths = card_lengths(reviews, mean);
            let cards = lengths.len();
            assert_eq!(
                lengths.iter().sum::<usize>(),
                reviews,
                "{reviews} rows at a mean of {mean}"
            );
            assert!(
                reviews.abs_diff(mean * cards) <= mean * mean,
                "{reviews} rows at a mean of {mean} made {cards} cards, further from rows / mean \
                 than one cut card"
            );
            assert!(
                lengths.iter().all(|length| (1..2 * mean).contains(length)),
                "{reviews} rows at a mean of {mean}: a length outside 1 to {}",
                2 * mean - 1
            );
            cells += 1;
        }
    }
    println!("examined {cells} grid cell(s)");
    assert_eq!(cells, 6, "the grid is three row counts by two means");
}
