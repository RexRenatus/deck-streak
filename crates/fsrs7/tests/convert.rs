//! SPEC-342 A5 and A6 (R4): review-log rows become one FSRS-7 item per card; SPEC-386 A2 to A4
//! (R2 to R4): a reset cuts a card's history, a history names its last kept review, and the
//! selection is a function of the row set. Every expected value is typed by hand from those
//! requirements, never printed by the crate.

#![allow(
    clippy::print_stdout,
    reason = "the enumerating test's helper prints what it examined, outside a #[test] function"
)]

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
        factor: FACTOR,
    }
}

fn card(cid: i64, reviews: &[(u32, f32)]) -> CardHistory {
    CardHistory {
        cid,
        last_id: 0,
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
            ended(3, T + 60 * HOUR, &[(4, 0.0), (1, 2.0)]),
            ended(7, T + 48 * HOUR, &[(3, 0.0), (2, 1.5), (3, 0.5)]),
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
        vec![ended(5, T + 24 * HOUR, &[(3, 0.0), (2, 1.0)])],
        "card 5 keeps its two rated reviews a day apart, and card 9, with no kept row, is no item"
    );
}

/// A review's ease factor as the engine logs it, in thousandths: never 0 once a card is
/// reviewed, so a row carrying it is no reset.
const FACTOR: u32 = 2500;

/// The engine's Forget row: a manual entry with the factor 0 (SPEC-386 R2, R12).
fn reset(cid: i64, at: i64) -> RevlogRow {
    RevlogRow {
        cid,
        id: at,
        ease: 0,
        kind: MANUAL,
        factor: 0,
    }
}

/// The engine's set-due-date row: a manual entry that keeps the card's factor.
fn set_due(cid: i64, at: i64) -> RevlogRow {
    RevlogRow {
        cid,
        id: at,
        ease: 0,
        kind: MANUAL,
        factor: FACTOR,
    }
}

/// A learning step answered before the card's first factor: kind 0 with the factor 0.
fn first_step(cid: i64, at: i64, ease: u8) -> RevlogRow {
    RevlogRow {
        cid,
        id: at,
        ease,
        kind: LEARNING,
        factor: 0,
    }
}

/// A history with its last kept review's id.
fn ended(cid: i64, last_id: i64, reviews: &[(u32, f32)]) -> CardHistory {
    CardHistory {
        last_id,
        ..card(cid, reviews)
    }
}

/// Prints how many `what` a test examined and refuses none.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

#[test]
fn a_reset_cuts_every_earlier_review_of_its_card() {
    // Card 5 is reset twice; only the reviews after the second reset count, and its set-due-date
    // row and its first learning step (both factor or kind short of a reset) cut nothing. Card 2
    // has no reset and keeps every review. Card 9 ends on a reset, so it has no kept review.
    let rows = [
        first_step(5, T, 3),
        row(5, T + 24 * HOUR, 3, REVIEW),
        reset(5, T + 30 * HOUR),
        row(5, T + 36 * HOUR, 4, REVIEW),
        reset(5, T + 40 * HOUR),
        first_step(5, T + 48 * HOUR, 3),
        set_due(5, T + 60 * HOUR),
        row(5, T + 72 * HOUR, 2, REVIEW),
        first_step(2, T, 1),
        row(2, T + 12 * HOUR, 3, REVIEW),
        row(9, T, 3, REVIEW),
        reset(9, T + 6 * HOUR),
    ];

    assert_eq!(
        convert::histories(&rows),
        vec![
            ended(2, T + 12 * HOUR, &[(1, 0.0), (3, 0.5)]),
            ended(5, T + 72 * HOUR, &[(3, 0.0), (2, 1.0)]),
        ],
        "card 5 keeps the two reviews after its last reset, card 2 keeps both, card 9 none"
    );
}

#[test]
fn the_history_names_its_last_kept_review() {
    // Card 4's last rows are dropped (a manual entry, an unrated entry, a reschedule), so its
    // last kept review is the second; card 6 has one review.
    let rows = [
        row(4, T, 3, LEARNING),
        row(4, T + 24 * HOUR, 3, REVIEW),
        set_due(4, T + 30 * HOUR),
        row(4, T + 36 * HOUR, 0, REVIEW),
        row(4, T + 40 * HOUR, 3, RESCHEDULED),
        row(6, T + 6 * HOUR, 4, REVIEW),
    ];

    let named: Vec<(i64, i64)> = convert::histories(&rows)
        .iter()
        .map(|history| (history.cid, history.last_id))
        .collect();
    assert_eq!(named, vec![(4, T + 24 * HOUR), (6, T + 6 * HOUR)]);
}

/// Every order of `rows`, by Heap's algorithm.
fn permutations(rows: &[RevlogRow]) -> Vec<Vec<RevlogRow>> {
    fn heap(k: usize, rows: &mut Vec<RevlogRow>, found: &mut Vec<Vec<RevlogRow>>) {
        if k <= 1 {
            found.push(rows.clone());
            return;
        }
        for i in 0..k - 1 {
            heap(k - 1, rows, found);
            if k.is_multiple_of(2) {
                rows.swap(i, k - 1);
            } else {
                rows.swap(0, k - 1);
            }
        }
        heap(k - 1, rows, found);
    }
    let mut rows = rows.to_vec();
    let mut found = Vec::new();
    heap(rows.len(), &mut rows, &mut found);
    found
}

#[test]
fn any_arrival_order_gives_the_same_histories() {
    let rows = [
        row(4, T, 3, REVIEW),
        reset(4, T + 12 * HOUR),
        row(4, T + 24 * HOUR, 4, REVIEW),
        row(4, T + 48 * HOUR, 3, REVIEW),
        first_step(8, T + 6 * HOUR, 3),
        row(8, T + 30 * HOUR, 1, REVIEW),
        set_due(8, T + 36 * HOUR),
    ];
    let expected = vec![
        ended(4, T + 48 * HOUR, &[(4, 0.0), (3, 1.0)]),
        ended(8, T + 30 * HOUR, &[(3, 0.0), (1, 1.0)]),
    ];

    let orders = permutations(&rows);
    for order in &orders {
        assert_eq!(
            convert::histories(order),
            expected,
            "the rows arrived as {order:?}"
        );
    }
    examined("arrival order(s)", orders);
}
