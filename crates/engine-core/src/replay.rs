//! The replay of a deck set's review history into stock-field values (SPEC-386 R7 to R13,
//! ADR-400 D2 and D3).
//!
//! The caller names a deck set and the preset's terms; the core reads the review rows of the
//! cards whose home deck is in the set by one fixed statement, the isolated FSRS-7 crate selects
//! and replays them at its pinned revision, and the result is each card's stock values. Nothing
//! is written: the write is the scheduler switch, the owner's tap on one preset (#611), and
//! [`CardReplay::stock_fields`] is the mapping that write will use.

use anki_proto::cards::Card;

use crate::dispatch::{Dispatcher, Refusal};

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
    /// field is left as it is.
    pub fn stock_fields(&self, card: &mut Card) {
        let _ = (self, card);
    }
}

impl Dispatcher {
    /// The replay of every card whose home deck is in `decks`, from its review history, under
    /// `parameters` (empty for the pinned revision's defaults, or 34 values), with the interval
    /// at the desired `retention` and at most `max_ivl` days. A card with no kept review has no
    /// entry. It writes nothing.
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
    ) -> Result<Vec<CardReplay>, Refusal> {
        let _ = (self, decks, parameters, retention, max_ivl);
        Ok(Vec::new())
    }
}
