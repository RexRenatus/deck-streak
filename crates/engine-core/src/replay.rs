//! The replay of a deck set's review history into stock-field values (SPEC-386 R7 to R13,
//! ADR-400 D2 and D3).
//!
//! The caller names a deck set, the preset's terms and the engine day it read; the core reads the
//! review rows of the cards whose home deck is in the set by one fixed statement, the isolated
//! FSRS-7 crate selects and replays them at its pinned revision, and the result is each card's
//! stock values. Nothing is written: the write is the scheduler switch, the owner's tap on one
//! preset (#611), and [`CardReplay::stock_fields`] is the mapping that write will use.

use std::collections::BTreeMap;

use anki_proto::cards::{Card, FsrsMemoryState};
use deck_streak_fsrs7::convert::{self, RevlogRow};
use deck_streak_fsrs7::replay::{self as scheduler, Method};
use deck_streak_fsrs7::stock::{self, FSRS};

use crate::dispatch::{Dispatcher, Refusal, failed};
use crate::late::EngineDay;

/// The length of a preset's fitted FSRS-7 parameter vector. An empty vector names the pinned
/// revision's defaults, and the scheduler would read most other lengths as an older model's, so
/// no other length is replayed (SPEC-386 R8).
const PARAMETER_COUNT: usize = 34;

/// A review card's type, as the engine's `CardType` numbers it: the one type given a schedule
/// (SPEC-386 R9).
const REVIEW_TYPE: i64 = 2;

/// Milliseconds in a second: a review-log id is an instant in milliseconds, and the engine's next
/// rollover one in seconds.
const MS_PER_SECOND: i64 = 1_000;

/// Seconds in one of the engine's days.
const SECONDS_PER_DAY: i64 = 86_400;

/// One replayed card: its stock values, and a review card's schedule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardReplay {
    /// The card.
    pub card: i64,
    /// The id of the card's last kept review: its answer's time, in milliseconds.
    pub last_review: i64,
    /// The stock stability: the interval, in days, at which the replayed state's forgetting curve
    /// reads 0.9.
    pub stability: f32,
    /// The replayed difficulty, held to the stock range of 1 to 10.
    pub difficulty: f32,
    /// A review card's schedule; `None` for a card of any other type, which keeps its own.
    pub schedule: Option<Schedule>,
}

/// A review card's schedule, with no fuzz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    /// The interval in days at the preset's desired retention, rounded, at least 1 and at most the
    /// preset's maximum.
    pub ivl: u32,
    /// The engine day of the card's last kept review plus [`Self::ivl`].
    pub due: i32,
}

impl CardReplay {
    /// Writes this replay's stock fields into `card`, a card of the engine's own message: the
    /// memory state's stability and difficulty, and a review card's interval and due. Every other
    /// field is left as it is. The due is written as the card's own: where it belongs for a card
    /// that sits in a filtered deck, whose own due is its place there and whose home due is kept
    /// apart, is the scheduler switch's to decide (#611), which this mapping leaves open.
    pub fn stock_fields(&self, card: &mut Card) {
        card.memory_state = Some(FsrsMemoryState {
            stability: self.stability,
            difficulty: self.difficulty,
        });
        if let Some(schedule) = self.schedule {
            card.interval = schedule.ivl;
            card.due = schedule.due;
        }
    }
}

impl Dispatcher {
    /// The replay of every card whose home deck is in `decks`, from its review history, under
    /// `parameters` (empty for the pinned revision's defaults, or 34 values), with the interval
    /// at the desired `retention` and at most `max_ivl` days, and a review card's due counted in
    /// `day`, the engine day its caller read. A card with no kept review has no entry. It writes
    /// nothing, and it makes no engine call but its one read: the engine's own day read unburies
    /// cards on a new day, so the day is the caller's (SPEC-386 R8, R10).
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the parameter vector is neither empty nor 34 values long, when
    /// the model refuses it, or when the engine cannot run the read.
    pub fn replay(
        &self,
        decks: &[i64],
        parameters: &[f32],
        retention: f32,
        max_ivl: u32,
        day: EngineDay,
    ) -> Result<Vec<CardReplay>, Refusal> {
        if !(parameters.is_empty() || parameters.len() == PARAMETER_COUNT) {
            return Err(failed(
                "the replay's parameter vector is neither empty nor 34 values long",
            ));
        }
        let model = FSRS::new(parameters)
            .map_err(|_| failed("the scheduler refuses the replay's parameter vector"))?;
        let mut rows = Vec::new();
        let mut types = BTreeMap::new();
        for row in self.history(decks)? {
            let (revlog, card_type) = decoded(row)
                .ok_or_else(|| failed("a review row's ease, kind or factor is out of its range"))?;
            types.insert(revlog.cid, card_type);
            rows.push(revlog);
        }
        let histories = convert::histories(&rows);
        let items = histories
            .iter()
            .map(|history| history.item.clone())
            .collect();
        let states = scheduler::replay(Method::Batch, &model, items)
            .map_err(|_| failed("the scheduler cannot replay a card's review history"))?;
        histories
            .iter()
            .zip(states)
            .map(|(history, state)| {
                let projected = stock::project(&model, state, retention);
                let schedule = if types.get(&history.cid) == Some(&REVIEW_TYPE) {
                    Some(scheduled(
                        projected.interval,
                        history.last_id,
                        day,
                        max_ivl,
                    )?)
                } else {
                    None
                };
                Ok(CardReplay {
                    card: history.cid,
                    last_review: history.last_id,
                    stability: projected.stability,
                    difficulty: projected.difficulty,
                    schedule,
                })
            })
            .collect()
    }
}

/// A row of the replay's read as a review-log row and its card's type, or `None` when a cell is
/// out of the range the engine's table holds it in.
fn decoded([cid, id, ease, kind, factor, card_type]: [i64; 6]) -> Option<(RevlogRow, i64)> {
    let row = RevlogRow {
        cid,
        id,
        ease: u8::try_from(ease).ok()?,
        kind: u8::try_from(kind).ok()?,
        factor: u32::try_from(factor).ok()?,
    };
    Some((row, card_type))
}

/// A review card's schedule: its `interval` in days rounded, at most `max_ivl` and at least 1, due
/// that many days after the engine day of its last kept review, `last_review`, with no fuzz.
fn scheduled(
    interval: f32,
    last_review: i64,
    day: EngineDay,
    max_ivl: u32,
) -> Result<Schedule, Refusal> {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a float-to-integer cast saturates: a negative or NaN interval reads 0, which the \
                  floor of 1 lifts, and one past the range reads its greatest, which the maximum \
                  holds"
    )]
    let rounded = interval.round() as u32;
    let ivl = rounded.min(max_ivl).max(1);
    let due = i32::try_from(day_of(day, last_review) + i64::from(ivl))
        .map_err(|_| failed("a review card's due is past the engine's range of days"))?;
    Ok(Schedule { ivl, due })
}

/// The engine day of the instant `at`, in milliseconds: today's day count, less one for each
/// whole day `at` lies before the instant today began, the next rollover less a day.
fn day_of(day: EngineDay, at: i64) -> i64 {
    let began = day.next_day_at - SECONDS_PER_DAY;
    i64::from(day.days_elapsed) + (at.div_euclid(MS_PER_SECOND) - began).div_euclid(SECONDS_PER_DAY)
}
