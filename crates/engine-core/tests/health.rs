//! The card health panel's record (SPEC-507 A1 to A3; ADR-525).
//!
//! A1 and A2 build a collection with the engine's own API, write or answer the review rows of one
//! of its cards, then read the record through a native dispatcher's two fixed reads, as the web
//! adapter does; A2 also holds the record's capped time equal to the analytics crate's over
//! generated rows. A3 judges the record's scheduler part in every state, beside a card that stores
//! an FSRS memory state.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;
use anki::prelude::{CardId, TimestampMillis};
use anki::scheduler::answering::{CardAnswer, Rating};
use deck_streak_analytics::metrics::daily_metrics;
use deck_streak_engine_core::dispatch::Dispatcher;
use deck_streak_engine_core::health::{
    self, ANSWER_CAP_MS, CardHealth, Logged, Provenance, ROWS_SHOWN, Readout, ReviewRow,
    SchedulerState,
};
use deck_streak_engine_core::table::Transport;
use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{StudyDay, StudyDayRule};
use serde_json::{Value, json};

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// The readout this build's web adapter gives: none, as no port carries one yet.
const ABSENT: Readout = Readout::NotAvailable(SchedulerState::Absent);
/// The fixture rows' first instant: 2023-11-14 22:13:20 UTC.
const BASE_MS: i64 = 1_700_000_000_000;
/// Writes an FSRS memory state into a card's data as the engine stores it: stability 12.34 days
/// and difficulty 5.6, which no part of the record may carry.
const STORE_STATE_SQL: &str =
    r#"update cards set data = '{"s":12.34,"d":5.6,"dr":0.9}' where id = ?"#;
/// Each state but Ready and its word in the record's JSON (SPEC-507 R6).
const STATE_WORDS: [(SchedulerState, &str); 4] = [
    (SchedulerState::Absent, "absent"),
    (SchedulerState::Fixture, "fixture"),
    (SchedulerState::Unready, "unready"),
    (SchedulerState::Stale, "stale"),
];

/// A native dispatcher with `synthetic` open.
fn opened(synthetic: &support::Synthetic) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &support::open_request(synthetic))
        .expect("the native dispatcher opens the collection");
    dispatcher
}

/// The record of `card`, read through the dispatcher's two fixed reads.
fn health_of(dispatcher: &Dispatcher, card: i64, readout: Readout) -> CardHealth {
    dispatcher
        .card_health(card, readout)
        .expect("the two fixed reads run")
        .expect("the card exists")
}

/// Runs each statement over the synthetic collection with the engine's own storage, each `?` in
/// it replaced by the next of its integers, then closes the collection.
fn write(synthetic: &support::Synthetic, statements: &[(&str, Vec<i64>)]) {
    let col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine opens the collection");
    for (sql, args) in statements {
        let mut args = args.iter();
        let statement: String = sql
            .split('?')
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    part.to_owned()
                } else {
                    format!(
                        "{}{part}",
                        args.next().expect("an integer for each placeholder")
                    )
                }
            })
            .collect();
        assert!(args.next().is_none(), "an integer left over for `{sql}`");
        col.storage
            .db()
            .execute_batch(&statement)
            .expect("the fixture statement runs");
    }
    col.close(None).expect("the engine closes the collection");
}

/// A review-log row for `card`: its instant, button, interval, time and type.
fn revlog(
    card: i64,
    at: i64,
    ease: i64,
    interval: i64,
    time: i64,
    kind: i64,
) -> (&'static str, Vec<i64>) {
    (
        "insert into revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
         values (?, ?, -1, ?, ?, 0, 2500, ?, ?)",
        vec![at, card, ease, interval, time, kind],
    )
}

/// The entries set by hand that come before the fixture's own rows, so the card has more rows
/// than the record carries.
const OLDER_ROWS: i64 = 200;

/// The fixture's statements: `OLDER_ROWS` entries set by hand, then seven rows whose answers'
/// capped times are 8,000, 0, 60,000, 12,000, 20,001 and 29,998 ms, then counters that differ
/// from the log one count at a time: `card`'s reps (8 against 6 answers, its lapses 1 against 1)
/// and `quiet`'s lapses (2 against none, its reps 0 against none).
fn fixture_rows(card: i64, quiet: i64) -> Vec<(&'static str, Vec<i64>)> {
    let mut statements: Vec<(&'static str, Vec<i64>)> = (0..OLDER_ROWS)
        .map(|index| revlog(card, BASE_MS - (OLDER_ROWS - index) * 1_000, 0, 0, 0, 4))
        .collect();
    statements.extend([
        revlog(card, BASE_MS + 1_000, 3, -600, 8_000, 0),
        // a learning Again is no lapse, and its negative time counts as 0
        revlog(card, BASE_MS + 2_000, 1, -60, -5_000, 0),
        // an hour's answer counts as the cap, 60,000 ms
        revlog(card, BASE_MS + 3_000, 3, 1, 3_600_000, 0),
        // a review Again: the one lapse
        revlog(card, BASE_MS + 4_000, 1, -600, 12_000, 1),
        // an entry set by hand is a row, not an answer
        revlog(card, BASE_MS + 5_000, 0, 0, 0, 4),
        revlog(card, BASE_MS + 6_000, 3, 1, 20_001, 2),
        // a review Good is no lapse
        revlog(card, BASE_MS + 7_000, 3, 4, 29_998, 1),
        (
            "update cards set reps = 8, lapses = 1 where id = ?",
            vec![card],
        ),
        (
            "update cards set reps = 0, lapses = 2 where id = ?",
            vec![quiet],
        ),
    ]);
    statements
}

/// The newest 200 of the fixture's rows as `(instant less BASE_MS, button, type, capped time)`:
/// its seven rows newest first, then the entries set by hand down to the eighth oldest row.
fn newest_rows() -> Vec<(i64, u8, u8, u32)> {
    [
        (7_000, 3, 1, 29_998),
        (6_000, 3, 2, 20_001),
        (5_000, 0, 4, 0),
        (4_000, 1, 1, 12_000),
        (3_000, 3, 0, ANSWER_CAP_MS),
        (2_000, 1, 0, 0),
        (1_000, 3, 0, 8_000),
    ]
    .into_iter()
    .chain((1..=OLDER_ROWS - 7).map(|age| (-age * 1_000, 0, 4, 0)))
    .collect()
}

/// Holds the record's JSON provenance to the card's created instant in epoch milliseconds: the
/// card's id, which the engine took from its clock in milliseconds when this test made the card.
fn created_in_milliseconds(card: i64, record: &Value) {
    let now_ms = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the clock reads after the epoch")
            .as_millis(),
    )
    .expect("a millisecond instant fits an i64");
    assert!(
        (now_ms - 86_400_000..=now_ms).contains(&card),
        "the card's id {card} is a millisecond instant of this run (now {now_ms} ms)"
    );
    assert_eq!(
        record.pointer("/provenance"),
        Some(&json!({ "source": "collection", "created_ms": card })),
        "the record's JSON carries the created instant in milliseconds"
    );
}

#[test]
fn the_panel_reads_a_fixture_cards_figures_through_the_two_fixed_reads() {
    let synthetic = support::synthetic("health-fixture");
    let [card, quiet] = synthetic.cards;
    write(&synthetic, &fixture_rows(card, quiet));
    let dispatcher = opened(&synthetic);
    let health = health_of(&dispatcher, card, ABSENT);
    support::examined("review row(s)", health.reviews.clone());
    // six answers capped: 0, 8,000, 12,000, 20,001, 29,998 and 60,000. Their sum, 129,999, is odd,
    // and its mean, 21,666.5, rounds half up; the two middle ones' mean, 16,000.5, is floored.
    assert_eq!(
        (
            health.reps,
            health.lapses,
            health.logged,
            health.rows,
            health.answer_ms_total,
            health.answer_ms_mean,
            health.answer_ms_median,
        ),
        (
            8,
            1,
            Logged {
                answers: 6,
                lapses: 1
            },
            207,
            129_999,
            Some(21_667),
            Some(16_000),
        ),
        "the card's counters, the rows' counts and the capped figures"
    );
    assert_eq!(
        health.reviews.len(),
        ROWS_SHOWN,
        "the record carries the newest 200 of the card's 207 rows"
    );
    assert_eq!(
        health
            .reviews
            .iter()
            .map(|review| (
                review.at_ms - BASE_MS,
                review.grade,
                review.kind,
                review.answer_ms_capped
            ))
            .collect::<Vec<_>>(),
        newest_rows(),
        "the newest 200 rows by time, newest first, each with its capped time"
    );
    assert_eq!(
        (health.card, health.provenance, health.scheduler),
        (card, Provenance::Collection { created_ms: card }, ABSENT),
        "the card came through the collection at the instant its id records"
    );
    created_in_milliseconds(card, &health.to_json());
    // a card with no row has no mean and no median, and shows its own counters
    let empty = health_of(&dispatcher, quiet, ABSENT);
    assert_eq!(
        (
            empty.reps,
            empty.lapses,
            empty.logged,
            empty.rows,
            empty.answer_ms_total,
            empty.answer_ms_mean,
            empty.answer_ms_median
        ),
        (
            0,
            2,
            Logged {
                answers: 0,
                lapses: 0
            },
            0,
            0,
            None,
            None
        ),
        "a card with no review row"
    );
    assert_eq!(
        dispatcher
            .card_health(card.max(quiet) + 1, ABSENT)
            .expect("the reads run"),
        None,
        "an id no card has"
    );
}

/// A seeded generator of the differential's rows (`SplitMix64`), so a failure repeats.
struct Rows(u64);

impl Rows {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> i64 {
        i64::try_from(self.next() % bound).expect("a bound below i64::MAX")
    }

    /// A time near the cap, far past it, or anywhere below it.
    fn time(&mut self) -> i64 {
        match self.below(6) {
            0 => i64::from(ANSWER_CAP_MS) + self.below(3) - 1,
            1 => self.below(4_000_000),
            2 => 0,
            _ => self.below(70_000),
        }
    }
}

#[test]
fn the_panels_capped_time_equals_the_analytics_functions_over_generated_rows() {
    // 20,000 days after the epoch, at 05:00 UTC: inside one study day under the default rule
    let day = StudyDay::from_epoch_day(20_000);
    let start = 20_000 * 86_400_000 + 5 * 3_600_000;
    let card = start - 1;
    let mut generator = Rows(507);
    let mut mismatches = Vec::new();
    let mut over_cap = 0;
    let cases: Vec<u32> = (0..300).collect();
    for case in support::examined("generated card(s)", cases) {
        let count = generator.below(41);
        let mut rows: Vec<ReviewRow> = Vec::new();
        let mut reviews: Vec<Review> = Vec::new();
        for index in 0..count {
            let (kind, ease, time) = (generator.below(6), generator.below(5), generator.time());
            if health::is_answer(kind, ease) && time > i64::from(ANSWER_CAP_MS) {
                over_cap += 1;
            }
            rows.push([start + index, ease, time, kind, 1]);
            reviews.push(Review {
                id: start + index,
                card_id: card,
                ease,
                interval: 1,
                last_interval: 0,
                factor: 2500,
                taken_ms: time,
                kind,
            });
        }
        let health = health::card_health([card, 0, 0, 0, 0, 0, 0], &rows, ABSENT);
        let metrics = daily_metrics(&reviews, StudyDayRule::default(), day, &BTreeMap::new());
        let analytics_ms = (metrics.seconds * 1000.0).round();
        #[allow(clippy::cast_precision_loss, reason = "a total under 2^53 ms is exact")]
        let panel_ms = health.answer_ms_total as f64;
        if (i64::from(health.logged.answers), panel_ms) != (metrics.reviews, analytics_ms) {
            mismatches.push(format!(
                "case {case}: panel {} answers {panel_ms} ms, analytics {} reviews {analytics_ms} ms",
                health.logged.answers, metrics.reviews
            ));
        }
    }
    assert!(
        over_cap > 0,
        "the generator drew no answer past the cap, so the cap was not judged"
    );
    assert_eq!(
        mismatches,
        Vec::<String>::new(),
        "the panel's capped time and the analytics crate's"
    );
}

#[test]
fn the_cards_reps_and_lapses_equal_the_counts_of_its_review_rows() {
    let synthetic = support::synthetic("health-answers");
    let [card, _] = synthetic.cards;
    // two Goods graduate the new card, each Again then lapses it from review
    let presses = [false, false, true, false, true, false];
    let mut col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine opens the collection");
    for again in presses {
        let states = col
            .get_scheduling_states(CardId(card))
            .expect("the engine gives the card's states");
        let (rating, new_state) = if again {
            (Rating::Again, states.again)
        } else {
            (Rating::Good, states.good)
        };
        let mut answer = CardAnswer {
            card_id: CardId(card),
            current_state: states.current,
            new_state,
            rating,
            answered_at: TimestampMillis::now(),
            milliseconds_taken: 4_000,
            custom_data: None,
            from_queue: true,
        };
        col.answer_card(&mut answer)
            .expect("the engine records the answer");
    }
    col.close(None).expect("the engine closes the collection");
    let dispatcher = opened(&synthetic);
    let health = health_of(&dispatcher, card, ABSENT);
    support::examined("review row(s)", health.reviews.clone());
    assert_eq!(
        (health.reps, health.lapses, health.logged),
        (
            6,
            2,
            Logged {
                answers: 6,
                lapses: 2
            }
        ),
        "the engine's counters and the counts over the rows its answers wrote"
    );
}

#[test]
fn no_scheduler_figure_reaches_the_recall_rows_outside_ready() {
    let synthetic = support::synthetic("health-states");
    let [card, _] = synthetic.cards;
    write(
        &synthetic,
        &[
            revlog(card, BASE_MS, 3, 12, 9_000, 1),
            (
                "update cards set type = 2, queue = 2, ivl = 12 where id = ?",
                vec![card],
            ),
        ],
    );
    // the card stores an FSRS memory state, as a card the previous scheduler kept does
    write(&synthetic, &[(STORE_STATE_SQL, vec![card])]);
    let dispatcher = opened(&synthetic);
    // each state's word as the record's JSON spells it, written out here rather than read back
    // from the type, so a wrong word fails
    let states = support::examined("state(s) but Ready", STATE_WORDS.to_vec());
    assert_eq!(
        states.iter().map(|(state, _)| *state).collect::<Vec<_>>(),
        SchedulerState::ALL.to_vec(),
        "every state but Ready, each with its word"
    );
    for (state, word) in states {
        let record = health_of(&dispatcher, card, Readout::NotAvailable(state)).to_json();
        assert_eq!(
            record.pointer("/scheduler"),
            Some(&json!({ "state": word })),
            "the recall rows' part names its state and no number"
        );
        let text = record.to_string();
        assert!(
            !text.contains('.') && !text.contains("12.34") && !text.contains("5.6"),
            "no stored stability or difficulty, and no fraction, in {text}"
        );
        assert_eq!(
            record.pointer("/reps").and_then(Value::as_u64),
            Some(0),
            "the rest of the record is read as ever"
        );
    }
    let ready = health_of(
        &dispatcher,
        card,
        Readout::Ready {
            recall_ppm: 912_345,
            days_to_90: 12,
        },
    )
    .to_json();
    assert_eq!(
        ready.pointer("/scheduler"),
        Some(&json!({ "state": "ready", "recall_ppm": 912_345, "days_to_90": 12 })),
        "a Ready readout carries the scheduler's two figures and nothing else"
    );
}
