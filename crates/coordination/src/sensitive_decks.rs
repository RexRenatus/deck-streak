//! The decks the learner keeps away from AI, as the API serves them (SPEC-381 R7; ADR-392 D1, D7):
//! the marked set, a mark and an unmark, each answering the set it leaves.
//!
//! The store and the rule are ingest's; this module only says which of the store's calls each
//! route makes. A deck id is taken as given, so a deck the server has not ingested yet can be kept
//! away before its first sync reaches the server.

use std::collections::BTreeSet;

use deck_streak_ingest::sensitive::SqliteSensitiveDecks;
use deck_streak_kernel::{Db, KernelError, UtcMillis};

/// Every marked deck's id, ascending.
///
/// # Errors
///
/// [`KernelError::Database`] when the read fails.
pub async fn list(db: &Db) -> Result<BTreeSet<i64>, KernelError> {
    SqliteSensitiveDecks::new(db.clone()).read_marked().await
}

/// Keeps `deck` away from AI from `at` on, and answers every marked deck's id. A deck already
/// marked keeps its first mark.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn mark(db: &Db, deck: i64, at: UtcMillis) -> Result<BTreeSet<i64>, KernelError> {
    SqliteSensitiveDecks::new(db.clone()).mark(deck, at).await
}

/// Lets `deck` reach AI again, and answers every marked deck's id. A deck not marked changes
/// nothing.
///
/// # Errors
///
/// [`KernelError::Database`] when the write fails.
pub async fn unmark(db: &Db, deck: i64) -> Result<BTreeSet<i64>, KernelError> {
    SqliteSensitiveDecks::new(db.clone()).unmark(deck).await
}
