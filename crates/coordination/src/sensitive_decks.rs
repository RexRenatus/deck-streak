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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[tokio::test]
    async fn list_mark_and_unmark_answer_the_exact_set() {
        let directory = tempfile::tempdir().expect("a scratch directory");
        let db = Db::open(&directory.path().join("deckstreak.db"))
            .await
            .expect("the database opens");
        let at = UtcMillis::from_epoch_millis(1_700_000_000_000);
        let empty = list(&db).await.expect("the first list");
        let first = mark(&db, 5, at).await.expect("the first mark");
        let second = mark(&db, 9, at).await.expect("the second mark");
        let listed = list(&db).await.expect("the second list");
        let unmarked = unmark(&db, 5).await.expect("the unmark");
        assert_eq!(
            (empty, first, second, listed, unmarked),
            (
                BTreeSet::new(),
                BTreeSet::from([5]),
                BTreeSet::from([5, 9]),
                BTreeSet::from([5, 9]),
                BTreeSet::from([9]),
            )
        );
        db.close().await;
    }
}
