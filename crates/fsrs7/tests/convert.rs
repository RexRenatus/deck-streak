//! SPEC-342 A5 and A6 (R4): review-log rows become one FSRS-7 item per card. Every expected value
//! is typed by hand from R4, never printed by the crate.

use deck_streak_fsrs7::convert::{self, CardHistory, RevlogRow};
use fsrs7::{FSRSItem, FSRSReview};

/// An arbitrary answer time, in milliseconds, that the fixtures count from.
const T: i64 = 5_000_000_000;
/// Milliseconds in an hour.
const HOUR: i64 = 3_600_000;
/// A review-log row's kind for a learning step and for a review.
const LEARNING: u8 = 0;
const REVIEW: u8 = 1;
/// A manual entry's kind and a reschedule's.
const MANUAL: u8 = 4;
const RESCHEDULED: u8 = 5;

fn row(cid: i64, at: i64, ease: u8, kind: u8) -> RevlogRow {
    RevlogRow {
        cid,
        id: at,
        ease,
        kind,
    }
}

fn card(cid: i64, reviews: &[(u32, f32)]) -> CardHistory {
    CardHistory {
        cid,
        item: FSRSItem {
            reviews: reviews
                .iter()
                .map(|&(rating, delta_t)| FSRSReview { rating, delta_t })
                .collect(),
        },
    }
}

#[test]
fn a_cards_reviews_become_fractional_day_intervals_from_zero() {
    // Card 7 answered at T, T+36h and T+48h; card 3, between them by id, at T+12h and T+60h. The
    // rows arrive out of id order.
    let rows = [
        row(7, T + 48 * HOUR, 3, REVIEW),
        row(3, T + 12 * HOUR, 4, REVIEW),
        row(7, T, 3, LEARNING),
        row(7, T + 36 * HOUR, 2, REVIEW),
        row(3, T + 60 * HOUR, 1, REVIEW),
    ];

    assert_eq!(
        convert::histories(&rows),
        vec![
            card(3, &[(4, 0.0), (1, 2.0)]),
            card(7, &[(3, 0.0), (2, 1.5), (3, 0.5)]),
        ]
    );
}

#[test]
fn manual_rescheduled_and_unrated_entries_are_dropped() {
    let rows = [
        row(5, T, 3, REVIEW),
        row(5, T + 6 * HOUR, 3, MANUAL),
        row(5, T + 12 * HOUR, 0, REVIEW),
        row(5, T + 18 * HOUR, 3, RESCHEDULED),
        row(5, T + 24 * HOUR, 2, REVIEW),
        row(9, T, 0, MANUAL),
    ];

    assert_eq!(
        convert::histories(&rows),
        vec![card(5, &[(3, 0.0), (2, 1.0)])],
        "card 5 keeps its two rated reviews a day apart, and card 9, with no kept row, is no item"
    );
}
