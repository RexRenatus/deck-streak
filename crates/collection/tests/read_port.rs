//! The store adapter of the read port (SPEC-512 A6; ADR-530 D9): a retracted review leaves every
//! read, a review's deck is its card's current home deck, and the scope is the cards' home decks.

#![allow(clippy::expect_used, clippy::print_stdout, clippy::too_many_lines)]

mod support;

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_collection::{
    CardRead, CollectionRead, FieldRead, LogCounts, NoteRead, ReviewRead, Store, TemplateRead,
};
use support::{SERVER, examined, raw};

/// Two decks, two notes with a card each, and four reviews: a review and a learn answer of card
/// 10, a review of card 11 that is retracted, and a rescheduling row of card 10.
const ROWS: &str = "
INSERT INTO notetypes VALUES (1, 'Basic', 'basic', 0, '', 0, '{}', 0);
INSERT INTO fields VALUES (1, 0, 'Front', '{}'), (1, 1, 'Back', '{}');
INSERT INTO templates VALUES (1, 0, 'Card 1', '{{Front}}', '{{Back}}', '{}');
INSERT INTO presets VALUES (1, 'Default', '{}', 0);
INSERT INTO decks VALUES (1, 'Default', 'default', 1, 0, '{}', 0), (2, 'Two', 'two', 1, 0, '{}', 0);
INSERT INTO notes VALUES (1, 'g1', 1, '[\"front one\",\"back one\"]', 'front one', zeroblob(32), 0),
  (2, 'g2', 1, '[\"front two\",\"back two\"]', 'front two', zeroblob(32), 0);
INSERT INTO note_tags VALUES (1, 'ds', 'ds'), (1, 'law', 'Law');
INSERT INTO cards VALUES (10, 1, 0, 1, NULL, 0, NULL, 0, 0), (11, 2, 0, 2, NULL, 1, NULL, 0, 0);
INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease,
  duration_ms, study_day, server_seq) VALUES
  (100, zeroblob(16), 1, 10, 1, 1, 1, 1, 3, 4000, 0, 1),
  (200, zeroblob(16), 2, 11, 2, 2, 1, 1, 3, 5000, 0, 2),
  (300, zeroblob(16), 3, 10, 1, 2, 1, 0, 1, 6000, 0, 3),
  (400, zeroblob(16), 4, 10, 1, 1, 1, 4, 0, 0, 0, 4);
INSERT INTO review_retraction VALUES (200, zeroblob(16), 5, 0);
";

#[test]
fn a_retracted_review_leaves_every_read_of_the_store_adapter() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("read.store");
    drop(Store::open(&path, SERVER).expect("the store opens"));
    let loaded = raw(&path).execute_batch(ROWS);
    let store = Store::open(&path, SERVER).expect("the store reopens");
    let both = BTreeSet::from([1, 2]);
    let two = BTreeSet::from([2]);
    let review = |id, card_id, ease, duration_ms, kind| ReviewRead {
        id,
        card_id,
        ease,
        duration_ms,
        kind,
    };
    let answers = store
        .read(|read| {
            Ok(vec![
                (
                    "deck names",
                    format!("{:?}", read.deck_names()?),
                    format!(
                        "{:?}",
                        BTreeMap::from([(1, "Default".to_owned()), (2, "Two".to_owned())])
                    ),
                ),
                (
                    "cards of both decks",
                    format!("{:?}", read.cards_in(&both)?),
                    format!(
                        "{:?}",
                        [
                            CardRead {
                                id: 10,
                                note_id: 1,
                                home_deck_id: 1,
                                suspended: false,
                                tags: BTreeSet::from(["Law".to_owned(), "ds".to_owned()]),
                            },
                            CardRead {
                                id: 11,
                                note_id: 2,
                                home_deck_id: 2,
                                suspended: true,
                                tags: BTreeSet::new(),
                            },
                        ]
                    ),
                ),
                (
                    "study reviews of both decks",
                    format!("{:?}", read.study_reviews_after(0, &both)?),
                    format!(
                        "{:?}",
                        [review(100, 10, 3, 4000, 1), review(300, 10, 1, 6000, 0)]
                    ),
                ),
                (
                    "study reviews of deck two, by the card's home deck",
                    format!("{:?}", read.study_reviews_after(0, &two)?),
                    "[]".to_owned(),
                ),
                (
                    "study review ids, one at a time",
                    format!("{:?}", read.study_review_ids_after(0, &both, 1)?),
                    "[100]".to_owned(),
                ),
                (
                    "study review ids above 100",
                    format!("{:?}", read.study_review_ids_after(100, &both, 10)?),
                    "[300]".to_owned(),
                ),
                (
                    "study reviews through 250",
                    format!("{:?}", read.study_reviews_through(250, &both)?),
                    "1".to_owned(),
                ),
                (
                    "the newest review",
                    format!("{:?}", read.newest_review()?),
                    "Some(400)".to_owned(),
                ),
                (
                    "templates",
                    format!("{:?}", read.templates_in(&both)?),
                    format!(
                        "{:?}",
                        [TemplateRead {
                            notetype_id: 1,
                            ord: 0,
                            notetype_name: "Basic".to_owned(),
                            name: "Card 1".to_owned(),
                        }]
                    ),
                ),
                (
                    "fields",
                    format!("{:?}", read.fields_in(&two)?),
                    format!(
                        "{:?}",
                        [
                            FieldRead {
                                notetype_id: 1,
                                ord: 0,
                                name: "Front".to_owned(),
                            },
                            FieldRead {
                                notetype_id: 1,
                                ord: 1,
                                name: "Back".to_owned(),
                            },
                        ]
                    ),
                ),
                (
                    "reviewed notes",
                    format!("{:?}", read.reviewed_notes(&both)?),
                    "{1}".to_owned(),
                ),
                (
                    "note fields",
                    format!("{:?}", read.note_fields(&BTreeSet::from([2]))?),
                    format!(
                        "{:?}",
                        [NoteRead {
                            id: 2,
                            notetype_id: 1,
                            fields: vec!["front two".to_owned(), "back two".to_owned()],
                        }]
                    ),
                ),
                (
                    "cards studied after 0",
                    format!("{:?}", read.studied_cards_after(0)?),
                    "{10}".to_owned(),
                ),
                (
                    "counts",
                    format!("{:?}", read.counts()?),
                    format!(
                        "{:?}",
                        LogCounts {
                            cards: 2,
                            notes: 2,
                            review_rows: 4,
                            reschedule_rows: 1,
                        }
                    ),
                ),
            ])
        })
        .expect("the reads run");
    let mut findings: Vec<String> = loaded
        .err()
        .map(|error| format!("the fixture rows were refused: {error}"))
        .into_iter()
        .collect();
    for (what, answered, expected) in examined("read(s)", answers) {
        if answered != expected {
            findings.push(format!("{what}: answered {answered}, expected {expected}"));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the store adapter");
}
