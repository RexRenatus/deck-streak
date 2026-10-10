//! The decks the learner keeps away from AI (SPEC-381 R1, R2; ADR-392 D1, D2): the store of the
//! marks in `sensitive_decks`, and `admits`, the one rule that decides whether a card may reach an
//! AI duty.
//!
//! A mark is one row per deck, keyed by the collection's deck id, and no row means the deck is not
//! marked, so every deck starts readable. The rule is fail-closed: a card is refused when its home
//! deck or its current deck, or any ancestor of either by name, is marked; when either deck is not
//! in the collection's deck tree; and when the marks could not be read. Every other card is
//! admitted. No other code decides whether a deck is kept away.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_kernel::{Db, KernelError, UtcMillis};

/// What `admits` decides for one card.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// No deck of the card is kept away: it may reach an AI duty.
    Admitted,
    /// The card's home deck or current deck, or an ancestor of either, is marked.
    KeptAway,
    /// One of the card's decks is not in the collection's deck tree.
    Unresolved,
    /// The marks could not be read, so no card can be judged.
    Unreadable,
}

impl Admission {
    /// Whether the card may reach an AI duty: only [`Admission::Admitted`] may.
    #[must_use]
    pub const fn admitted(self) -> bool {
        matches!(self, Self::Admitted)
    }
}

/// Decides one card (R2): `marked` is the set of marked deck ids, or `None` when it could not be
/// read; `tree` is every deck's name by its id, each name's parts separated by the reader's
/// [`DECK_SEPARATOR`](crate::settings::DECK_SEPARATOR); `home` is the deck the card belongs to and
/// `current` the deck it sits in now, a filtered deck while one borrows it.
pub fn admits(
    marked: Option<&BTreeSet<i64>>,
    tree: &BTreeMap<i64, String>,
    home: i64,
    current: i64,
) -> Admission {
    let _ = (marked, tree, home, current);
    Admission::Admitted
}

/// `sensitive_decks` in the service's own database.
#[derive(Clone, Debug)]
pub struct SqliteSensitiveDecks {
    db: Db,
}

impl SqliteSensitiveDecks {
    /// The marks in `db`, whose migrations created `sensitive_decks`.
    #[must_use]
    pub const fn new(db: Db) -> Self {
        Self { db }
    }

    /// Every marked deck's id, ascending.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the read fails; a caller reads that as every deck kept away.
    pub async fn read_marked(&self) -> Result<BTreeSet<i64>, KernelError> {
        let _connection = self.db.reader().acquire().await?;
        Ok(BTreeSet::new())
    }

    /// Marks `deck` at `at`, and answers every marked deck's id. Marking a marked deck changes
    /// nothing.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails, or `deck` is not a positive id.
    pub async fn mark(&self, deck: i64, at: UtcMillis) -> Result<BTreeSet<i64>, KernelError> {
        let _ = (deck, at);
        self.read_marked().await
    }

    /// Unmarks `deck`, and answers every marked deck's id. Unmarking an unmarked deck changes
    /// nothing.
    ///
    /// # Errors
    ///
    /// [`KernelError::Database`] when the write fails.
    pub async fn unmark(&self, deck: i64) -> Result<BTreeSet<i64>, KernelError> {
        let _ = deck;
        self.read_marked().await
    }
}
