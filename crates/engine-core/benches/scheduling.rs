//! The scheduler's benches (SPEC-618 R9, R13): the day queue, the event check
//! folded over one card's history, FSRS-7's fold followed by its whole-day interval, and the
//! integer schedule step behind it.
//!
//! Every input is built from a fixed seed before the measured function runs, so two runs measure
//! the same work, and no bench asserts a value the engine computes. Each measured body is a named
//! `#[inline(never)]` function `<bench>_measured` that the Divan closure calls with its input, so
//! the measured symbol holds the engine's work alone and setup never counts.

#![allow(
    clippy::expect_used,
    reason = "a bench whose fixed input cannot be built should stop, as a failed test does"
)]

use deck_streak_engine_core::late::EngineDay;
use deck_streak_engine_core::replay::{self, Fsrs7};
use deck_streak_engine_core::scheduler::{
    self, DayQueue, EventKey, FoldError, NO_NOTE, QueueCard, Recall, ReviewEvent, SchedulerPort,
};
use divan::Bencher;

/// The day queue's cards, the notes they share and the days their dues spread over.
const QUEUE_CARDS: u32 = 5_000;
const QUEUE_NOTES: u32 = 1_500;
const QUEUE_DAYS: u32 = 30;
/// The study day every queue and history is read on.
const TODAY: i32 = 20_000;
/// The reviews of the event check's one card, and of the fold's one long history.
const LONG_HISTORY: u32 = 1_000;
/// The fold's many cards and the reviews of each.
const MANY_CARDS: u32 = 1_000;
const SHORT_HISTORY: u32 = 10;
/// The schedule steps.
const SCHEDULE_STEPS: u32 = 10_000;
/// Milliseconds in a day, the review keys' unit.
const DAY_MS: i64 = 86_400_000;
/// Seconds in a day, the engine day's unit.
const DAY_SECONDS: i64 = 86_400;
/// The instant the schedule steps' engine days begin from, in seconds since the epoch.
const NEXT_DAY_AT: i64 = 1_800_000_000;
/// The grades a history draws from, Good most often.
const GRADES: [u8; 8] = [3, 3, 3, 2, 3, 4, 1, 3];

fn main() {
    divan::main();
}

/// A fixed-seed generator (`SplitMix64`): one seed gives the same inputs on every run.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^ (mixed >> 31)
    }

    /// A value from 0 to `bound` less one.
    fn below(&mut self, bound: u32) -> u32 {
        u32::try_from(self.next() % u64::from(bound)).unwrap_or_default()
    }

    fn grade(&mut self) -> u8 {
        let index = usize::try_from(self.below(8)).unwrap_or_default();
        GRADES.get(index).copied().unwrap_or(3)
    }
}

/// The day queue's input: cards over shared notes, 2% with no note, due over the 30 days around today.
fn queue_cards() -> Vec<QueueCard> {
    let mut seeded = Seeded(0x0618_0001);
    (1..=QUEUE_CARDS)
        .map(|card| {
            let note = if seeded.below(100) < 2 {
                NO_NOTE
            } else {
                i64::from(seeded.below(QUEUE_NOTES)) + 1
            };
            let offset = i32::try_from(seeded.below(QUEUE_DAYS)).unwrap_or_default();
            QueueCard {
                card: i64::from(card),
                note,
                due: TODAY - 15 + offset,
            }
        })
        .collect()
}

/// `reviews` reviews of `card` on days 1 to 9 apart: each key is its day's start, the card's own
/// second within the day and a few seeded milliseconds, so every key is strictly increasing and
/// no two cards' keys meet.
fn history(seeded: &mut Seeded, card: u32, reviews: u32) -> Vec<ReviewEvent> {
    let mut day = TODAY - 10_000;
    (0..reviews)
        .map(|_| {
            day += i32::try_from(seeded.below(9) + 1).unwrap_or(1);
            let key =
                i64::from(day) * DAY_MS + i64::from(card) * 1_000 + i64::from(seeded.below(1_000));
            ReviewEvent {
                card: i64::from(card),
                note: i64::from(card),
                deck: 1,
                preset: 1,
                grade: seeded.grade(),
                answer_ms: 1_000 + seeded.below(20_000),
                key: EventKey(key),
                study_day: day,
            }
        })
        .collect()
}

/// The fold's two inputs: every card's reviews in key order, and the cards.
fn histories(cards: u32, reviews: u32) -> (Vec<ReviewEvent>, Vec<i64>) {
    let mut seeded = Seeded(0x0618_0003);
    let mut events: Vec<ReviewEvent> = (1..=cards)
        .flat_map(|card| history(&mut seeded, card, reviews))
        .collect();
    events.sort_by_key(|event| event.key);
    (events, (1..=cards).map(i64::from).collect())
}

/// The schedule step's input: whole-day intervals, last reviews in the 2,000 days before the day, the engine's
/// day and the preset's maximum.
fn schedule_steps() -> Vec<(u32, i64, EngineDay, u32)> {
    let mut seeded = Seeded(0x0618_0004);
    (0..SCHEDULE_STEPS)
        .map(|_| {
            let day = EngineDay {
                days_elapsed: 2_000 + seeded.below(1_000),
                next_day_at: NEXT_DAY_AT,
            };
            let began = NEXT_DAY_AT - DAY_SECONDS;
            let last_review = (began - i64::from(seeded.below(2_000 * 86_400))) * 1_000;
            let max_ivl =
                [365, 3_650, 36_500][usize::try_from(seeded.below(3)).unwrap_or_default()];
            (seeded.below(40_000), last_review, day, max_ivl)
        })
        .collect()
}

#[divan::bench]
fn day_queue(bencher: Bencher) {
    let cards = queue_cards();
    bencher.bench_local(|| day_queue_measured(&cards, TODAY));
}

/// The day's queue: the due filter, the sort by due then card, and the sibling rule.
#[inline(never)]
fn day_queue_measured(cards: &[QueueCard], day: i32) -> DayQueue {
    scheduler::day_queue(cards, day)
}

#[divan::bench]
fn check_event(bencher: Bencher) {
    let events = history(&mut Seeded(0x0618_0002), 1, LONG_HISTORY);
    bencher.bench_local(|| check_event_measured(&events));
}

/// The check every fold makes first, over one card's ordered history.
#[inline(never)]
fn check_event_measured(events: &[ReviewEvent]) -> Result<(), FoldError> {
    let mut last = None;
    for event in events {
        scheduler::check_event(last, event)?;
        last = Some(event.key);
    }
    Ok(())
}

#[divan::bench(args = ["one card", "many cards"])]
fn fold(bencher: Bencher, shape: &str) {
    let model = Fsrs7::new(&[]).expect("the pinned revision's defaults make a model");
    let target = Recall::new(0.9).expect("0.9 is a recall");
    let (events, cards) = if shape == "one card" {
        histories(1, LONG_HISTORY)
    } else {
        histories(MANY_CARDS, SHORT_HISTORY)
    };
    bencher.bench_local(|| fold_measured(&model, &events, &cards, target));
}

/// FSRS-7's fold of every event into a fresh state, then each card's whole days at the
/// target, the first of which replays the folded history.
#[inline(never)]
fn fold_measured(
    model: &Fsrs7,
    events: &[ReviewEvent],
    cards: &[i64],
    target: Recall,
) -> Result<Vec<u32>, FoldError> {
    let mut state = model.new_state();
    for event in events {
        model.fold(&mut state, event)?;
    }
    cards
        .iter()
        .map(|card| model.interval_days(&state, *card, target))
        .collect()
}

#[divan::bench]
fn schedule_of(bencher: Bencher) {
    let steps = schedule_steps();
    bencher.bench_local(|| schedule_of_measured(&steps));
}

/// The integer schedule step of each whole-day interval: how many the replay schedules, and
/// the sum of their due days.
#[inline(never)]
fn schedule_of_measured(steps: &[(u32, i64, EngineDay, u32)]) -> (usize, i64) {
    steps
        .iter()
        .filter_map(|&(rounded, last_review, day, max_ivl)| {
            replay::schedule_of(rounded, last_review, day, max_ivl)
        })
        .fold((0, 0), |(count, sum), schedule| {
            (count + 1, sum + i64::from(schedule.due))
        })
}
