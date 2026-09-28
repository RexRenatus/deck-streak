//! The read (SPEC-023 A1, A2, A6): the copy is read only through a read-only connection, inside the
//! owner's scope, and with no collation registered.
//!
//! Each test builds the scoped collection with Anki's own engine where the syncer keeps its copy:
//! the engine's `decks` table collates its names `unicase`, and the engine's own filtered deck
//! borrows two cards. The collection reader then reads it as a cycle does.

mod support;

use std::error::Error;

use deck_streak_ingest::reader::ReadError;
use deck_streak_kernel::{Track, UtcMillis};
use support::Fixture;
use support::synthetic::{self, FILTERED_DECK, SCOPED_FLOOR, scoped_review_id};

/// An endpoint no test contacts: the reader never syncs.
const ENDPOINT: &str = "http://127.0.0.1:9/";

/// Writes of every kind, the last after lifting `query_only` on its own connection, which leaves
/// the `mode=ro` open to refuse it.
const WRITES: [&str; 5] = [
    "INSERT INTO revlog (id, cid, usn, ease, ivl, lastIvl, factor, time, type) \
     VALUES (1, 1001, 0, 3, 1, 0, 2500, 4000, 1)",
    "UPDATE cards SET due = 0",
    "DELETE FROM decks",
    "CREATE TABLE planted (id INTEGER)",
    "PRAGMA query_only = OFF; DELETE FROM revlog",
];

/// Every message of `error`'s chain, joined.
fn chain(error: &dyn Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

#[tokio::test]
async fn a_write_through_the_collection_reader_fails() {
    let fixture = Fixture::new(ENDPOINT);
    synthetic::build_scoped(&fixture.copy());
    let reader = synthetic::reader(&fixture.settings(), "", None, support::clock_at(0));
    let before = reader.read(0).await.expect("the copy reads");

    for statement in WRITES {
        let refused = reader.execute_statement(statement).await;
        assert!(
            matches!(refused, Err(ReadError::WriteRefused)),
            "a write through the reader was not refused as read-only: {statement}: {refused:?}"
        );
    }
    // The same connection runs a read, which changes nothing, and the copy is as it was.
    let counted = reader
        .execute_statement("SELECT count(*) FROM revlog")
        .await
        .expect("a read runs through the reader");
    assert_eq!(counted, 0);
    let after = reader.read(0).await.expect("the copy reads");
    assert_eq!(after, before, "no write reached the copy");
    assert_eq!(after.cards.len(), 7, "every card of every deck");
    assert_eq!(after.reviews.len(), 10, "every study event of the log");
}

#[tokio::test]
async fn only_scoped_cards_and_their_study_reviews_are_read() {
    let fixture = Fixture::new(ENDPOINT);
    let planned = synthetic::build_scoped(&fixture.copy());
    let reader = synthetic::reader(
        &fixture.settings(),
        "Law, Language",
        Some("Law"),
        support::clock_at(0),
    );
    let read = reader.read(SCOPED_FLOOR).await.expect("the copy reads");

    let cards: Vec<i64> = read.cards.iter().map(|card| card.id).collect();
    assert_eq!(
        cards,
        [1001, 1002, 1003, 1004],
        "the cards whose home deck is under Law or Language, the borrowed Law card included"
    );
    let reviews: Vec<(i64, i64, i64, i64)> = read
        .reviews
        .iter()
        .map(|review| (review.id, review.card_id, review.kind, review.ease))
        .collect();
    assert_eq!(
        reviews,
        [
            (scoped_review_id(1), 1001, 0, 3),
            (scoped_review_id(2), 1001, 1, 2),
            (scoped_review_id(3), 1002, 2, 1),
            (scoped_review_id(4), 1002, 3, 4),
            (scoped_review_id(5), 1003, 0, 1),
        ],
        "the study events of the cards in scope after the floor, oldest first"
    );
    // Each review carries its row, and each card its decks and track.
    let first = read.reviews[0];
    assert_eq!(
        (
            first.interval,
            first.last_interval,
            first.factor,
            first.taken_ms
        ),
        (1, 0, 2500, 4000)
    );
    let tracks: Vec<Track> = read.cards.iter().map(|card| card.track).collect();
    assert_eq!(
        tracks,
        [Track::Law, Track::Law, Track::Language, Track::Language]
    );
    let borrowed = read.cards[1];
    assert_eq!(
        (borrowed.deck_id, borrowed.original_deck_id),
        (planned.deck(FILTERED_DECK), planned.deck("Law::Torts"))
    );
    assert_eq!(
        read.created_at,
        UtcMillis::from_epoch_millis(synthetic::creation_secs(&fixture.copy()) * 1000)
    );
}

#[tokio::test]
async fn a_unicase_collated_deck_table_is_read_without_the_collation() {
    let fixture = Fixture::new(ENDPOINT);
    synthetic::build_scoped(&fixture.copy());
    let reader = synthetic::reader(&fixture.settings(), "", None, support::clock_at(0));
    let read = reader.read(0).await.expect("the copy reads");

    // Every deck's stored name, read with no collation registered, is the one the engine reads
    // through its own connection, which registers `unicase`.
    let stored = synthetic::stored_deck_names(&fixture.copy());
    assert_eq!(read.deck_names, stored);
    assert!(
        stored.values().any(|name| name == "Law\u{1f}Evidence"),
        "{stored:?}"
    );
    // The fixture is the trap: the engine declared the name column `unicase`...
    let declared = synthetic::decks_table_sql(&fixture.copy());
    assert!(
        declared.to_lowercase().contains("collate unicase"),
        "{declared}"
    );
    // ...and the reader's connection registers no such collation, so an ordering by name fails
    // there: the read above touched no name in SQL.
    let ordered = reader
        .execute_statement("SELECT id FROM decks ORDER BY name")
        .await
        .map_err(|error| chain(&error));
    assert!(
        ordered
            .as_ref()
            .is_err_and(|message| message.contains("no such collation sequence: unicase")),
        "an ordering by a unicase name needs the collation: {ordered:?}"
    );
}
