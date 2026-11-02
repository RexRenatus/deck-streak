//! The reorder-only input of due-card scores (SPEC-414 A1 to A6; ADR-428 D1, D4; ADR-339): the
//! reorder presents the stock queue's own entries, unchanged, with the scored cards first, highest
//! first and stable, and the stock order whenever the input is off, absent, stale or empty.
//!
//! A queue entry here carries its card and five stock fields, each distinct from every other
//! card's, so a moved entry and a changed field are each visible. The enumerating tests run over a
//! generated population and refuse one that moved no card.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeMap;

use deck_streak_engine_core::reorder::{DueScores, reorder};

/// One entry of the stock queue: the card and the fields the stock scheduler wrote.
#[derive(Clone, Debug, PartialEq)]
struct Queued {
    card: i64,
    due: i64,
    interval: i64,
    ease: i64,
    difficulty: i64,
    stability: i64,
}

/// Today, as the engine counts days.
const TODAY: u32 = 3;

/// Today's schema stamp of the collection.
const SNAPSHOT: i64 = 7;

/// The cards the population arranges.
const CARDS: [i64; 4] = [11, 12, 13, 14];

/// The entry of `card`, with fields distinct from every other card's.
fn queued(card: i64) -> Queued {
    Queued {
        card,
        due: 100 + card,
        interval: card - 8,
        ease: 2000 + 10 * card,
        difficulty: card % 10,
        stability: 30 + card,
    }
}

/// The stock queue of `cards`, in that order.
fn stock(cards: &[i64]) -> Vec<Queued> {
    cards.iter().copied().map(queued).collect()
}

/// The cards of `queue`, in its order.
fn cards_of(queue: &[Queued]) -> Vec<i64> {
    queue.iter().map(|entry| entry.card).collect()
}

/// An input made for `day` and `snapshot`, scoring each listed card.
fn input(day: u32, snapshot: i64, scores: &[(i64, f64)]) -> DueScores {
    DueScores {
        day,
        snapshot,
        scores: scores.iter().copied().collect(),
    }
}

/// Today's reorder of `queue`, at today's day and stamp.
fn presented(on: bool, scores: Option<&DueScores>, queue: Vec<Queued>) -> Vec<Queued> {
    reorder(on, TODAY, SNAPSHOT, scores, queue, |entry: &Queued| {
        entry.card
    })
}

/// Today's reorder of the stock queue of `cards`, as cards.
fn order(on: bool, scores: Option<&DueScores>, cards: &[i64]) -> Vec<i64> {
    cards_of(&presented(on, scores, stock(cards)))
}

/// Every arrangement of `cards`.
fn arrangements(cards: &[i64]) -> Vec<Vec<i64>> {
    if cards.is_empty() {
        return vec![Vec::new()];
    }
    let mut found = Vec::new();
    for (at, &first) in cards.iter().enumerate() {
        let mut rest = cards.to_vec();
        rest.remove(at);
        for mut tail in arrangements(&rest) {
            tail.insert(0, first);
            found.push(tail);
        }
    }
    found
}

/// Every map that gives each of `CARDS` no score, 0, 0.5, 1, -0 or NaN.
fn maps() -> Vec<BTreeMap<i64, f64>> {
    let codes = [
        None,
        Some(0.0),
        Some(0.5),
        Some(1.0),
        Some(-0.0),
        Some(f64::NAN),
    ];
    let mut found = vec![BTreeMap::new()];
    for card in CARDS {
        let mut next = Vec::new();
        for map in &found {
            for code in codes {
                let mut map = map.clone();
                if let Some(score) = code {
                    map.insert(card, score);
                }
                next.push(map);
            }
        }
        found = next;
    }
    found
}

/// The population: every arrangement of the four cards, each with every map, keyed to today.
fn population() -> Vec<(Vec<i64>, DueScores)> {
    let maps = maps();
    arrangements(&CARDS)
        .into_iter()
        .flat_map(|queue| {
            maps.iter().map(move |scores| {
                (
                    queue.clone(),
                    DueScores {
                        day: TODAY,
                        snapshot: SNAPSHOT,
                        scores: scores.clone(),
                    },
                )
            })
        })
        .collect()
}

/// Why `shown` is not the entries of `stock`, each unchanged, or `None` when it is.
fn refusal(stock: &[Queued], shown: &[Queued]) -> Option<String> {
    if stock.len() != shown.len() {
        return Some(format!(
            "{} stock entries are shown as {}",
            stock.len(),
            shown.len()
        ));
    }
    let mut held = stock.to_vec();
    held.sort_by_key(|entry| entry.card);
    let mut kept = shown.to_vec();
    kept.sort_by_key(|entry| entry.card);
    held.iter()
        .zip(&kept)
        .find(|(held, kept)| held != kept)
        .map(|(held, kept)| format!("card {}: {held:?} is shown as {kept:?}", held.card))
}

/// A planted model that reorders and then moves the head card's due date to today: ADR-339's
/// refused "model that writes a due date".
fn planted(scores: &DueScores, queue: Vec<Queued>) -> Vec<Queued> {
    let mut shown = presented(true, Some(scores), queue);
    if let Some(head) = shown.first_mut() {
        head.due = i64::from(TODAY);
    }
    shown
}

#[test]
fn the_reorder_presents_the_same_entries_and_writes_no_field() {
    let cases = support::examined("queue and input case(s)", population());
    let mut moved = 0_usize;
    for (cards, scores) in &cases {
        let shown = presented(true, Some(scores), stock(cards));
        assert_eq!(
            refusal(&stock(cards), &shown),
            None,
            "the stock queue {cards:?} with {scores:?} is shown as its own entries, unchanged"
        );
        if cards_of(&shown) != *cards {
            moved += 1;
        }
    }
    println!("moved {moved} of {} case(s)", cases.len());
    assert!(
        moved > 0,
        "the population moved no card, so it judged no reorder"
    );

    let scores = input(TODAY, SNAPSHOT, &[(12, 1.0)]);
    assert_eq!(
        refusal(&stock(&[11, 12]), &planted(&scores, stock(&[11, 12]))),
        Some(format!(
            "card 12: {:?} is shown as {:?}",
            queued(12),
            Queued {
                due: 3,
                ..queued(12)
            }
        )),
        "a planted model that writes the head card's due date is refused by the card it changed"
    );
}

#[test]
fn scored_cards_lead_highest_first_and_the_stock_order_breaks_ties() {
    let scores = input(TODAY, SNAPSHOT, &[(12, 0.5), (13, 1.0), (14, 0.5)]);
    assert_eq!(
        order(true, Some(&scores), &[11, 12, 13, 14]),
        vec![13, 12, 14, 11],
        "13 leads at 1, then 12 and 14 at 0.5 in stock order, then unscored 11"
    );
    assert_eq!(
        order(true, Some(&scores), &[14, 13, 12, 11]),
        vec![13, 14, 12, 11],
        "the tie between 14 and 12 follows the stock order, whichever it is"
    );

    let zeros = input(TODAY, SNAPSHOT, &[(11, 0.0), (12, -0.0), (13, 0.25)]);
    assert_eq!(
        order(true, Some(&zeros), &[11, 12, 13]),
        vec![13, 11, 12],
        "0 and -0 tie, so 11 stays before 12 as the stock order has them"
    );
    assert_eq!(
        order(true, Some(&zeros), &[12, 11, 13]),
        vec![13, 12, 11],
        "0 and -0 tie, so 12 stays before 11 as the stock order has them"
    );
}

#[test]
fn an_invalid_score_counts_as_no_score() {
    let invalid = support::examined(
        "invalid score(s)",
        vec![
            ("NaN", f64::NAN),
            ("inf", f64::INFINITY),
            ("-inf", f64::NEG_INFINITY),
            ("1.25", 1.25),
            ("-0.25", -0.25),
            ("the largest finite", f64::MAX),
        ],
    );
    for (name, score) in invalid {
        let scores = input(TODAY, SNAPSHOT, &[(11, score), (13, 0.5)]);
        assert_eq!(
            order(true, Some(&scores), &[12, 11, 13]),
            vec![13, 12, 11],
            "a score of {name} counts as no score: 11 stays after the unscored 12, as the stock \
             order has it, and behind the scored 13"
        );
    }

    let bounds = input(TODAY, SNAPSHOT, &[(11, 1.0), (12, 0.0)]);
    assert_eq!(
        order(true, Some(&bounds), &[13, 12, 11]),
        vec![11, 12, 13],
        "1 and 0 are scores: both lead the unscored 13"
    );
}

#[test]
fn scores_made_for_another_day_or_snapshot_leave_the_stock_order() {
    let stale = [
        (TODAY - 1, SNAPSHOT, "yesterday's scores"),
        (TODAY + 1, SNAPSHOT, "scores made for tomorrow"),
        (TODAY, SNAPSHOT - 1, "scores made for a replaced snapshot"),
        (
            TODAY - 1,
            SNAPSHOT - 1,
            "scores of another day and snapshot",
        ),
    ];
    for (day, snapshot, what) in support::examined("stale key(s)", stale.to_vec()) {
        let scores = input(day, snapshot, &[(12, 1.0)]);
        assert_eq!(
            presented(true, Some(&scores), stock(&[11, 12])),
            stock(&[11, 12]),
            "{what} leave the stock order"
        );
    }
    let own = input(TODAY, SNAPSHOT, &[(12, 1.0)]);
    assert_eq!(
        order(true, Some(&own), &[11, 12]),
        vec![12, 11],
        "the same scores on their own day and snapshot reorder the queue"
    );
}

#[test]
fn an_unused_input_presents_the_stock_queue_and_on_again_reorders() {
    let mixed = input(TODAY, SNAPSHOT, &[(11, 0.25), (12, 1.0), (13, 0.5)]);
    let unused = [
        (false, Some(mixed.clone()), "the switch off"),
        (true, None, "no input"),
        (true, Some(input(TODAY, SNAPSHOT, &[])), "an empty map"),
        (
            true,
            Some(input(TODAY, SNAPSHOT, &[(99, 1.0)])),
            "a map that scores no queued card",
        ),
        (
            true,
            Some(input(
                TODAY,
                SNAPSHOT,
                &[(11, f64::NAN), (12, f64::INFINITY), (13, 1.25)],
            )),
            "a map whose every queued score is invalid",
        ),
    ];
    for (on, scores, what) in support::examined("unused input(s)", unused.to_vec()) {
        assert_eq!(
            presented(on, scores.as_ref(), stock(&[11, 12, 13])),
            stock(&[11, 12, 13]),
            "{what} presents the stock queue exactly"
        );
    }
    assert_eq!(
        presented(true, Some(&mixed), Vec::new()),
        Vec::<Queued>::new(),
        "an empty queue stays empty"
    );
    assert_eq!(
        order(true, Some(&mixed), &[11, 12, 13]),
        vec![12, 13, 11],
        "the switch on again reorders the queue by the scores"
    );
}

#[test]
fn reordering_twice_is_once_and_the_order_reads_only_the_queued_scores() {
    let cases = support::examined("queue and input case(s)", population());
    let mut moved = 0_usize;
    for (cards, scores) in &cases {
        let once = presented(true, Some(scores), stock(cards));
        assert_eq!(
            presented(true, Some(scores), once.clone()),
            once,
            "the reorder of {cards:?} by {scores:?} is its own reorder"
        );

        let mut other = DueScores {
            scores: scores
                .scores
                .iter()
                .filter(|(_, score)| (0.0..=1.0).contains(*score))
                .map(|(card, score)| (*card, *score))
                .collect(),
            ..scores.clone()
        };
        other.scores.insert(99, 0.75);
        other.scores.insert(98, f64::NAN);
        assert_eq!(
            presented(true, Some(&other), stock(cards)),
            once,
            "an input with the same valid scores for the queued cards of {cards:?} gives the same order"
        );
        if cards_of(&once) != *cards {
            moved += 1;
        }
    }
    println!("moved {moved} of {} case(s)", cases.len());
    assert!(
        moved > 0,
        "the population moved no card, so it judged no reorder"
    );
}
