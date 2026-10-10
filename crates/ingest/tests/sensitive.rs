//! The decks the learner keeps away from AI (SPEC-381 A2, A3, A11; R1, R2, R10): `admits` refuses a
//! card of a marked deck, of a deck under one, and one a filtered deck borrowed from one; it refuses
//! a card whose deck the tree cannot resolve and every card when the marks cannot be read; and the
//! marks are exported and erased, so after an erase every deck is readable again.
//!
//! Every deck name and id is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_ingest::data_rights::{IngestDataRights, SENSITIVE_DECKS_TABLE};
use deck_streak_ingest::sensitive::{Admission, SqliteSensitiveDecks, admits};
use deck_streak_ingest::settings::DECK_SEPARATOR;
use deck_streak_kernel::{DataRights, Db};
use serde_json::{Value, json};
use tempfile::TempDir;

/// The law root.
const LAW: i64 = 1;
/// A subject under the law root.
const EVIDENCE: i64 = 2;
/// A language root, never marked.
const LANGUAGE: i64 = 3;
/// A filtered deck, which borrows cards from the others.
const FILTERED: i64 = 4;
/// A root whose name begins with the law root's name and is not under it.
const LAWYER: i64 = 5;
/// A deck id the tree does not hold.
const UNKNOWN: i64 = 99;

/// The collection's deck tree: each name's parts joined by the reader's separator.
fn tree() -> BTreeMap<i64, String> {
    BTreeMap::from([
        (LAW, "Law".to_owned()),
        (EVIDENCE, format!("Law{DECK_SEPARATOR}Evidence")),
        (LANGUAGE, "Language".to_owned()),
        (FILTERED, "Filtered".to_owned()),
        (LAWYER, "Lawyer".to_owned()),
    ])
}

/// A database every migration has run on, in a scratch directory that lives as long as it.
async fn migrated() -> (TempDir, Db) {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let db = Db::open(&directory.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    (directory, db)
}

#[test]
fn a_marked_deck_refuses_its_own_cards_its_childrens_and_the_cards_a_filtered_deck_borrowed() {
    let tree = tree();
    let law = BTreeSet::from([LAW]);
    let filtered = BTreeSet::from([FILTERED]);
    let decided: Vec<(&str, Admission)> = vec![
        ("own card", admits(Some(&law), &tree, LAW, LAW)),
        (
            "child's card",
            admits(Some(&law), &tree, EVIDENCE, EVIDENCE),
        ),
        (
            "borrowed from a child",
            admits(Some(&law), &tree, EVIDENCE, FILTERED),
        ),
        (
            "sits in a marked filtered deck",
            admits(Some(&filtered), &tree, LANGUAGE, FILTERED),
        ),
        (
            "unmarked root",
            admits(Some(&law), &tree, LANGUAGE, LANGUAGE),
        ),
        (
            "name prefix only",
            admits(Some(&law), &tree, LAWYER, LAWYER),
        ),
        (
            "borrowed from an unmarked deck",
            admits(Some(&law), &tree, LANGUAGE, FILTERED),
        ),
    ];
    assert_eq!(
        decided,
        [
            ("own card", Admission::KeptAway),
            ("child's card", Admission::KeptAway),
            ("borrowed from a child", Admission::KeptAway),
            ("sits in a marked filtered deck", Admission::KeptAway),
            ("unmarked root", Admission::Admitted),
            ("name prefix only", Admission::Admitted),
            ("borrowed from an unmarked deck", Admission::Admitted),
        ]
    );
}

#[tokio::test]
async fn an_unresolved_deck_or_an_unreadable_set_reads_as_kept_away() {
    let tree = tree();
    let none = BTreeSet::new();
    // The marks' table moved away: the store's read fails, and the rule reads that as kept away.
    let (_directory, db) = migrated().await;
    let mut write = db.write().await.expect("a write");
    sqlx::query("ALTER TABLE sensitive_decks RENAME TO sensitive_decks_moved")
        .execute(&mut *write)
        .await
        .expect("the table moves");
    write.commit().await.expect("the move commits");
    let unreadable = SqliteSensitiveDecks::new(db.clone()).read_marked().await;
    let decided: Vec<(&str, Admission)> = vec![
        (
            "home deck unknown",
            admits(Some(&none), &tree, UNKNOWN, LANGUAGE),
        ),
        (
            "current deck unknown",
            admits(Some(&none), &tree, LANGUAGE, UNKNOWN),
        ),
        ("marks unread", admits(None, &tree, LANGUAGE, LANGUAGE)),
        (
            "marks' read failed",
            admits(unreadable.as_ref().ok(), &tree, LANGUAGE, LANGUAGE),
        ),
        ("control", admits(Some(&none), &tree, LANGUAGE, LANGUAGE)),
    ];
    assert_eq!(
        decided,
        [
            ("home deck unknown", Admission::Unresolved),
            ("current deck unknown", Admission::Unresolved),
            ("marks unread", Admission::Unreadable),
            ("marks' read failed", Admission::Unreadable),
            ("control", Admission::Admitted),
        ]
    );
    db.close().await;
}

#[tokio::test]
async fn the_marks_are_exported_and_an_erase_leaves_every_deck_readable() {
    let tree = tree();
    let (_directory, db) = migrated().await;
    // Two marks, written as the table holds them, so the oracle is not the store's own write.
    let mut write = db.write().await.expect("a write");
    for (deck, at) in [(LAW, 1_700_000_000_000_i64), (FILTERED, 1_700_000_000_500)] {
        sqlx::query("INSERT INTO sensitive_decks (deck_id, created_at) VALUES (?1, ?2)")
            .bind(deck)
            .bind(at)
            .execute(&mut *write)
            .await
            .expect("a mark");
    }
    write.commit().await.expect("the marks commit");
    let mut write = db.write().await.expect("a write");
    let exported = IngestDataRights
        .export(&mut write)
        .await
        .expect("the export reads");
    let marks: Option<&Vec<Value>> = exported
        .iter()
        .find(|table| table.table == SENSITIVE_DECKS_TABLE)
        .map(|table| &table.rows);
    assert_eq!(
        marks,
        Some(&vec![
            json!({ "deck_id": LAW, "created_at": 1_700_000_000_000_i64 }),
            json!({ "deck_id": FILTERED, "created_at": 1_700_000_000_500_i64 }),
        ]),
        "the export carries every mark, every column"
    );
    IngestDataRights.erase(&mut write).await.expect("the erase");
    write.commit().await.expect("the erase commits");

    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM sensitive_decks")
        .fetch_one(db.reader())
        .await
        .expect("the table reads");
    assert_eq!(left, 0, "the erase left a mark behind");
    let after = SqliteSensitiveDecks::new(db.clone())
        .read_marked()
        .await
        .expect("the marks read");
    assert_eq!(
        admits(Some(&after), &tree, EVIDENCE, EVIDENCE),
        Admission::Admitted,
        "after an erase every deck is readable again"
    );
    db.close().await;
}
