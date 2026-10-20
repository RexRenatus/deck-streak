//! The replay of a deck set's review history into stock-field values (SPEC-386 R7 to R13,
//! ADR-400 D2 and D3), through the scheduling port (SPEC-494 R2 to R4, ADR-512 D2 and D3).
//!
//! The caller names a deck set, the preset's terms and the engine day it read; the core reads the
//! review rows of the cards whose home deck is in the set by one fixed statement, the isolated
//! FSRS-7 crate selects them, and the port's FSRS-7 implementation folds them and gives each
//! card's stock values and whole-day interval. Nothing is written: the write is the scheduler
//! switch, the owner's tap on one preset (#611), and [`CardReplay::stock_fields`] is the mapping
//! that write will use.

use std::collections::BTreeMap;

use anki_proto::cards::{Card, FsrsMemoryState};
use deck_streak_fsrs7::convert::{self, CardHistory, RevlogRow};

use crate::dispatch::{Dispatcher, Refusal, failed};
use crate::late::EngineDay;
use crate::scheduler::{EventKey, NO_NOTE, Recall, ReviewEvent, SchedulerPort};

pub use fsrs7_port::{FSRS7_VERSION, Fsrs7, Fsrs7State};

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

/// The refusal of a fold, an interval or a stock value of the port: today's refusal of the
/// scheduler's replay.
const CANNOT_REPLAY: &str = "the scheduler cannot replay a card's review history";

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

/// The scheduling port's FSRS-7 implementation (SPEC-494 R2, ADR-512 D2): the one place of the
/// core that calls the pinned revision's model, and it reads no clock.
mod fsrs7_port {
    use std::cell::OnceCell;
    use std::collections::BTreeMap;

    use deck_streak_fsrs7::convert::{self, RevlogRow};
    use deck_streak_fsrs7::replay::{self as scheduler, Method};
    use deck_streak_fsrs7::stock::{self, FSRS, MemoryState};

    use crate::scheduler as port;
    use port::SchedulerPort;

    /// The implementation's fixed identifier: the crate's name and the pin's 8-hex prefix, then
    /// two zero bytes.
    pub const FSRS7_VERSION: [u8; 32] = *b"deck-streak-fsrs7 rev 4bc0a097\0\0";

    /// The bytes a checkpoint starts with.
    const CHECKPOINT_MAGIC: [u8; 4] = *b"DSSP";

    /// The checkpoint's format.
    const CHECKPOINT_FORMAT: u8 = 1;

    /// A checkpoint's header: the magic, the format, the identifier and the event count as a u64,
    /// little-endian.
    const HEADER_LEN: usize = 45;

    /// One event in a checkpoint: card, note, deck and preset as i64, the grade as a u8, the
    /// answer time as a u32, the key as an i64 and the study day as an i32, all little-endian.
    const EVENT_LEN: usize = 49;

    /// FSRS-7 at the pinned revision, under one parameter vector.
    #[derive(Debug, Clone)]
    pub struct Fsrs7 {
        /// The model.
        model: FSRS,
        /// The bits of the parameters the model was made under.
        parameters: Vec<u32>,
    }

    /// The folded state: the events in key order, the last key, the bits of the parameters it was
    /// made under, and each card's memory state, replayed once after the last fold.
    #[derive(Debug, Clone)]
    pub struct Fsrs7State {
        /// The folded events, in key order.
        events: Vec<port::ReviewEvent>,
        /// The last folded key.
        last: Option<port::EventKey>,
        /// The bits of the parameters the state was made under.
        parameters: Vec<u32>,
        /// Each card's memory state from the batch replay of every folded event, reset on every
        /// fold.
        memory: OnceCell<Result<BTreeMap<i64, MemoryState>, port::FoldError>>,
    }

    impl Fsrs7 {
        /// FSRS-7 under `parameters`, empty for the pinned revision's defaults.
        ///
        /// # Errors
        ///
        /// [`port::FoldError::ModelRefused`] when the model refuses the vector.
        pub fn new(parameters: &[f32]) -> Result<Self, port::FoldError> {
            let model = FSRS::new(parameters).map_err(|_| port::FoldError::ModelRefused)?;
            Ok(Self {
                model,
                parameters: parameters.iter().copied().map(f32::to_bits).collect(),
            })
        }

        /// The card's stock stability and difficulty, from the projection at `target` that
        /// gives its interval.
        ///
        /// # Errors
        ///
        /// A [`port::FoldError`] for an unknown card or a state made under other parameters.
        pub fn stock_values(
            &self,
            st: &Fsrs7State,
            card: i64,
            target: port::Recall,
        ) -> Result<(f32, f32), port::FoldError> {
            let state = self.memory(st, card)?;
            let projected = stock::project(&self.model, state, target.get());
            Ok((projected.stability, projected.difficulty))
        }

        /// Refuses a state made under other parameters.
        fn same_model(&self, st: &Fsrs7State) -> Result<(), port::FoldError> {
            if st.parameters == self.parameters {
                Ok(())
            } else {
                Err(port::FoldError::OtherModel)
            }
        }

        /// The card's memory state, from the one batch replay of the folded events.
        fn memory(&self, st: &Fsrs7State, card: i64) -> Result<MemoryState, port::FoldError> {
            self.same_model(st)?;
            let states = st
                .memory
                .get_or_init(|| self.replayed(&st.events))
                .as_ref()
                .map_err(|error| *error)?;
            states
                .get(&card)
                .copied()
                .ok_or(port::FoldError::UnknownCard { card })
        }

        /// Every folded card's memory state: the events as review rows sorted by card then id,
        /// as the replay's statement orders them (kind 1, factor 1 and the grade as the ease),
        /// selected by the crate's own history selection and replayed in one batch.
        fn replayed(
            &self,
            events: &[port::ReviewEvent],
        ) -> Result<BTreeMap<i64, MemoryState>, port::FoldError> {
            let mut rows: Vec<RevlogRow> = events
                .iter()
                .map(|event| RevlogRow {
                    cid: event.card,
                    id: event.key.0,
                    ease: event.grade,
                    kind: 1,
                    factor: 1,
                })
                .collect();
            rows.sort_by_key(|row| (row.cid, row.id));
            let histories = convert::histories(&rows);
            let items = histories
                .iter()
                .map(|history| history.item.clone())
                .collect();
            let states = scheduler::replay(Method::Batch, &self.model, items)
                .map_err(|_| port::FoldError::ModelRefused)?;
            Ok(histories
                .iter()
                .map(|history| history.cid)
                .zip(states)
                .collect())
        }
    }

    impl SchedulerPort for Fsrs7 {
        type State = Fsrs7State;

        fn version_id(&self) -> [u8; 32] {
            FSRS7_VERSION
        }

        fn new_state(&self) -> Fsrs7State {
            Fsrs7State {
                events: Vec::new(),
                last: None,
                parameters: self.parameters.clone(),
                memory: OnceCell::new(),
            }
        }

        fn fold(&self, st: &mut Fsrs7State, ev: &port::ReviewEvent) -> Result<(), port::FoldError> {
            self.same_model(st)?;
            port::check_event(st.last, ev)?;
            st.events.push(*ev);
            st.last = Some(ev.key);
            st.memory = OnceCell::new();
            Ok(())
        }

        fn predict(
            &self,
            st: &Fsrs7State,
            card: i64,
            at_day: i32,
        ) -> Result<port::Recall, port::FoldError> {
            let state = self.memory(st, card)?;
            let last_day = st
                .events
                .iter()
                .rev()
                .find(|event| event.card == card)
                .map(|event| event.study_day)
                .ok_or(port::FoldError::UnknownCard { card })?;
            let elapsed = at_day
                .checked_sub(last_day)
                .filter(|days| *days >= 0)
                .ok_or(port::FoldError::BeforeLastReview { card, at_day })?;
            #[allow(
                clippy::cast_precision_loss,
                reason = "a count of whole days between two study days is exact in an f32 up to \
                          2^24 days, far past any learner's history"
            )]
            let recall = self.model.current_retrievability(state, elapsed as f32);
            port::Recall::clamped(recall).ok_or(port::FoldError::NotFinite { card })
        }

        fn interval_days(
            &self,
            st: &Fsrs7State,
            card: i64,
            target: port::Recall,
        ) -> Result<u32, port::FoldError> {
            let state = self.memory(st, card)?;
            let interval = stock::project(&self.model, state, target.get()).interval;
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a float-to-integer cast saturates: a negative or NaN interval reads 0, which the \
                          floor of 1 lifts, and one past the range reads its greatest, which the maximum \
                          holds"
            )]
            let whole = interval.round() as u32;
            Ok(whole)
        }

        fn checkpoint(&self, st: &Fsrs7State) -> Vec<u8> {
            let mut bytes = Vec::with_capacity(
                HEADER_LEN.saturating_add(st.events.len().saturating_mul(EVENT_LEN)),
            );
            bytes.extend_from_slice(&CHECKPOINT_MAGIC);
            bytes.push(CHECKPOINT_FORMAT);
            bytes.extend_from_slice(&FSRS7_VERSION);
            bytes.extend_from_slice(&(st.events.len() as u64).to_le_bytes());
            for event in &st.events {
                bytes.extend_from_slice(&event.card.to_le_bytes());
                bytes.extend_from_slice(&event.note.to_le_bytes());
                bytes.extend_from_slice(&event.deck.to_le_bytes());
                bytes.extend_from_slice(&event.preset.to_le_bytes());
                bytes.push(event.grade);
                bytes.extend_from_slice(&event.answer_ms.to_le_bytes());
                bytes.extend_from_slice(&event.key.0.to_le_bytes());
                bytes.extend_from_slice(&event.study_day.to_le_bytes());
            }
            bytes
        }

        fn restore(&self, bytes: &[u8]) -> Result<Fsrs7State, port::LoadError> {
            if bytes.get(..CHECKPOINT_MAGIC.len()) != Some(&CHECKPOINT_MAGIC[..]) {
                return Err(port::LoadError::NotACheckpoint);
            }
            if bytes.len() < HEADER_LEN {
                return Err(port::LoadError::Truncated);
            }
            let mut reader = Reader(bytes.get(CHECKPOINT_MAGIC.len()..).unwrap_or_default());
            if reader.array::<1>().map(|[format]| format) != Some(CHECKPOINT_FORMAT) {
                return Err(port::LoadError::NotACheckpoint);
            }
            let version = reader
                .take(FSRS7_VERSION.len())
                .ok_or(port::LoadError::Truncated)?;
            if version != FSRS7_VERSION.as_slice() {
                return Err(port::LoadError::OtherModel);
            }
            let count = reader
                .array()
                .map(u64::from_le_bytes)
                .ok_or(port::LoadError::Truncated)?;
            let expected = usize::try_from(count)
                .ok()
                .and_then(|count| count.checked_mul(EVENT_LEN))
                .and_then(|events| events.checked_add(HEADER_LEN))
                .ok_or(port::LoadError::NotACheckpoint)?;
            if bytes.len() < expected {
                return Err(port::LoadError::Truncated);
            }
            if bytes.len() > expected {
                return Err(port::LoadError::NotACheckpoint);
            }
            let mut st = self.new_state();
            while let Some(event) = reader.event() {
                self.fold(&mut st, &event)
                    .map_err(port::LoadError::Refused)?;
            }
            Ok(st)
        }
    }

    /// A checkpoint's bytes, read from the front.
    struct Reader<'a>(&'a [u8]);

    impl<'a> Reader<'a> {
        /// The next `len` bytes, or `None` when fewer remain.
        fn take(&mut self, len: usize) -> Option<&'a [u8]> {
            let (head, rest) = self.0.split_at_checked(len)?;
            self.0 = rest;
            Some(head)
        }

        /// The next `N` bytes as an array.
        fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
            self.take(N).and_then(|head| head.try_into().ok())
        }

        /// The next event, or `None` when a whole one does not remain.
        fn event(&mut self) -> Option<port::ReviewEvent> {
            Some(port::ReviewEvent {
                card: i64::from_le_bytes(self.array()?),
                note: i64::from_le_bytes(self.array()?),
                deck: i64::from_le_bytes(self.array()?),
                preset: i64::from_le_bytes(self.array()?),
                grade: u8::from_le_bytes(self.array()?),
                answer_ms: u32::from_le_bytes(self.array()?),
                key: port::EventKey(i64::from_le_bytes(self.array()?)),
                study_day: i32::from_le_bytes(self.array()?),
            })
        }
    }
}

impl Dispatcher {
    /// The replay of every card whose home deck is in `decks`, from its review history, under
    /// `parameters` (empty for the pinned revision's defaults, or 34 values), with the interval
    /// at the desired `retention` and at most `max_ivl` days, and a review card's due counted in
    /// `day`, the engine day its caller read. A card with no kept review has no entry. It writes
    /// nothing, and it makes no engine call but its one read: the engine's own day read unburies
    /// cards on a new day, so the day is the caller's (SPEC-386 R8, R10). Each card's kept reviews
    /// are folded into one FSRS-7 state of the scheduling port, each event carrying the engine day
    /// of its review, and a review card's whole days come only from the port (SPEC-494 R3).
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the parameter vector is neither empty nor 34 values long, when
    /// the model refuses it, when the retention is not a recall in its range, or when the engine
    /// cannot run the read.
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
        let fsrs7 = Fsrs7::new(parameters)
            .map_err(|_| failed("the scheduler refuses the replay's parameter vector"))?;
        let target = Recall::new(retention)
            .ok_or_else(|| failed("the replay's retention is not a recall in its range"))?;
        let mut rows = Vec::new();
        let mut types = BTreeMap::new();
        for row in self.history(decks)? {
            let (revlog, card_type) = decoded(row)
                .ok_or_else(|| failed("a review row's ease, kind or factor is out of its range"))?;
            types.insert(revlog.cid, card_type);
            rows.push(revlog);
        }
        let histories = convert::histories(&rows);
        let mut events = kept_events(&rows, &histories, day)?;
        events.sort_by_key(|event| event.key);
        let mut state = fsrs7.new_state();
        for event in &events {
            fsrs7
                .fold(&mut state, event)
                .map_err(|_| failed(CANNOT_REPLAY))?;
        }
        histories
            .iter()
            .map(|history| {
                let (stability, difficulty) = fsrs7
                    .stock_values(&state, history.cid, target)
                    .map_err(|_| failed(CANNOT_REPLAY))?;
                let schedule = if types.get(&history.cid) == Some(&REVIEW_TYPE) {
                    let rounded = fsrs7
                        .interval_days(&state, history.cid, target)
                        .map_err(|_| failed(CANNOT_REPLAY))?;
                    Some(scheduled(rounded, history.last_id, day, max_ivl)?)
                } else {
                    None
                };
                Ok(CardReplay {
                    card: history.cid,
                    last_review: history.last_id,
                    stability,
                    difficulty,
                    schedule,
                })
            })
            .collect()
    }
}

/// The port's events of every history: per card, its rows with a rating and a kind the crate
/// keeps, by id, the last as many as its history holds, each checked against the history's last
/// id and ratings; each event keyed by its row's id and dated by its engine day, with no note,
/// deck, preset or answer time, which the replay's statement does not read.
fn kept_events(
    rows: &[RevlogRow],
    histories: &[CardHistory],
    day: EngineDay,
) -> Result<Vec<ReviewEvent>, Refusal> {
    let mut kept: BTreeMap<i64, Vec<&RevlogRow>> = BTreeMap::new();
    for row in rows
        .iter()
        .filter(|row| row.ease != 0 && !convert::DROPPED_KINDS.contains(&row.kind))
    {
        kept.entry(row.cid).or_default().push(row);
    }
    let mut events = Vec::new();
    for history in histories {
        let mut card = kept.remove(&history.cid).unwrap_or_default();
        card.sort_by_key(|row| row.id);
        let reviews = &history.item.reviews;
        let selected = &card[card.len().saturating_sub(reviews.len())..];
        let agrees = selected.len() == reviews.len()
            && selected.last().map(|row| row.id) == Some(history.last_id)
            && selected
                .iter()
                .zip(reviews)
                .all(|(row, review)| u32::from(row.ease) == review.rating);
        if !agrees {
            return Err(failed(
                "the replay's events disagree with the history selection",
            ));
        }
        for row in selected {
            let study_day = i32::try_from(day_of(day, row.id))
                .map_err(|_| failed("a review's engine day is past the engine's range of days"))?;
            events.push(ReviewEvent {
                card: history.cid,
                note: NO_NOTE,
                deck: 0,
                preset: 0,
                grade: row.ease,
                answer_ms: 0,
                key: EventKey(row.id),
                study_day,
            });
        }
    }
    Ok(events)
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

/// A review card's schedule: its whole-day interval `rounded`, from the port, at most `max_ivl`
/// and at least 1, due that many days after the engine day of its last kept review,
/// `last_review`, with no fuzz.
fn scheduled(
    rounded: u32,
    last_review: i64,
    day: EngineDay,
    max_ivl: u32,
) -> Result<Schedule, Refusal> {
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
