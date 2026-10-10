//! The deck gate (SPEC-381 R4; ADR-392 D2): the port a duty run asks, after the route check and
//! before the input gate, `compose` and the runner, whether the decks its cards come from may reach
//! an AI duty.
//!
//! The agent holds no rule about decks. The port's implementation, in the daemon's wiring, decides
//! with ingest's one rule over the learner's marks. A scope the gate cannot judge reads as kept
//! away, and so does a run whose cards carry no scope: the gate fails closed, and each refusal is
//! recorded `withheld` with its class, never skipped in silence.

use std::future::Future;
use std::pin::Pin;

/// The class a run is withheld under when a deck in its scope is kept away from AI.
pub const DECK_SENSITIVE: &str = "deck-sensitive";
/// The class a run is withheld under when its scope cannot be judged: the marks unreadable, a deck
/// unresolved, or cards with no scope.
pub const DECK_UNREADABLE: &str = "deck-unreadable";

/// One card's decks: the deck it belongs to and the deck it sits in now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardDecks {
    /// The card's home deck: the deck a filtered deck borrowed it from, else the deck it sits in.
    pub home: i64,
    /// The deck the card sits in now, a filtered deck while one borrows it.
    pub current: i64,
}

/// The deck scope of a run's cards: each card's home and current deck.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeckScope<'a> {
    /// Each card's decks, one entry per card.
    pub cards: &'a [CardDecks],
}

impl DeckScope<'_> {
    /// Whether the scope names no card.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

/// What the gate decides for a scope. Its counts are the only detail a refusal carries: never a
/// deck's name, a card's text or an id.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeckVerdict {
    /// Every card's decks may reach an AI duty.
    Admitted,
    /// `cards` of the scope's cards belong to, or sit in, a deck the learner keeps away.
    KeptAway {
        /// How many cards were kept away.
        cards: usize,
    },
    /// The gate could not judge `cards` of the scope's cards: the marks could not be read, or a
    /// deck is not in the collection's deck tree.
    Unreadable {
        /// How many cards could not be judged.
        cards: usize,
    },
}

/// The gate's answer, in flight.
pub type DeckFuture<'a> = Pin<Box<dyn Future<Output = DeckVerdict> + Send + 'a>>;

/// The deck gate, a port: the daemon's wiring implements it over ingest's marks.
pub trait DeckGate: Send + Sync {
    /// Judges `scope`. An implementation that cannot read the marks answers
    /// [`DeckVerdict::Unreadable`], never [`DeckVerdict::Admitted`].
    fn judge<'a>(&'a self, scope: DeckScope<'a>) -> DeckFuture<'a>;
}
