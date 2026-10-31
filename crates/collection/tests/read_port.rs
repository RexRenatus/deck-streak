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

#[test]
fn the_store_adapter_counts_each_review_and_keeps_each_note_apart() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("read.store");
    drop(Store::open(&path, SERVER).expect("the store opens"));
    raw(&path)
        .execute_batch(ROWS)
        .expect("the fixture rows load");
    let store = Store::open(&path, SERVER).expect("the store reopens");
    let both = BTreeSet::from([1, 2]);
    let two = BTreeSet::from([2]);
    let answers = store
        .read(|read| {
            Ok(vec![
                (
                    "study reviews through 1000",
                    format!("{:?}", read.study_reviews_through(1000, &both)?),
                    "2".to_owned(),
                ),
                (
                    "study reviews through 50",
                    format!("{:?}", read.study_reviews_through(50, &both)?),
                    "0".to_owned(),
                ),
                (
                    "reviewed notes of deck two, whose one review is retracted",
                    format!("{:?}", read.reviewed_notes(&two)?),
                    "{}".to_owned(),
                ),
                (
                    "note fields of both notes",
                    format!("{:?}", read.note_fields(&BTreeSet::from([1, 2]))?),
                    format!(
                        "{:?}",
                        [
                            NoteRead {
                                id: 1,
                                notetype_id: 1,
                                fields: vec!["front one".to_owned(), "back one".to_owned()],
                            },
                            NoteRead {
                                id: 2,
                                notetype_id: 1,
                                fields: vec!["front two".to_owned(), "back two".to_owned()],
                            },
                        ]
                    ),
                ),
            ])
        })
        .expect("the reads run");
    let mut findings = Vec::new();
    for (what, answered, expected) in examined("read(s)", answers) {
        if answered != expected {
            findings.push(format!("{what}: answered {answered}, expected {expected}"));
        }
    }
    assert_eq!(findings, Vec::<String>::new(), "the store adapter's counts");
}

/// Three decks, nine notes with a card each, and ten reviews, each kept out of the study reads by
/// one guard alone: one study review of each study kind in decks one and two (100 to 130), a
/// review answer of kind 4 (140), a rescheduling row of kind 5 (150), an unanswered review (160),
/// a retracted review (170), a review of a card in deck three (180), whose note alone uses the
/// second notetype, and one rescheduling row (190).
const GUARD_ROWS: &str = "
INSERT INTO notetypes VALUES (1, 'Basic', 'basic', 0, '', 0, '{}', 0),
  (2, 'Other', 'other', 0, '', 0, '{}', 0);
INSERT INTO fields VALUES (1, 0, 'Front', '{}'), (2, 0, 'Text', '{}');
INSERT INTO templates VALUES (1, 0, 'Card 1', '{{Front}}', '{{Front}}', '{}'),
  (2, 0, 'Other 1', '{{Text}}', '{{Text}}', '{}');
INSERT INTO presets VALUES (1, 'Default', '{}', 0);
INSERT INTO decks VALUES (1, 'One', 'one', 1, 0, '{}', 0), (2, 'Two', 'two', 1, 0, '{}', 0),
  (3, 'Three', 'three', 1, 0, '{}', 0);
INSERT INTO notes VALUES (1, 'g1', 1, '[\"one\"]', 'one', zeroblob(32), 0),
  (2, 'g2', 1, '[\"two\"]', 'two', zeroblob(32), 0),
  (3, 'g3', 1, '[\"three\"]', 'three', zeroblob(32), 0),
  (4, 'g4', 1, '[\"four\"]', 'four', zeroblob(32), 0),
  (5, 'g5', 1, '[\"five\"]', 'five', zeroblob(32), 0),
  (6, 'g6', 1, '[\"six\"]', 'six', zeroblob(32), 0),
  (7, 'g7', 1, '[\"seven\"]', 'seven', zeroblob(32), 0),
  (8, 'g8', 1, '[\"eight\"]', 'eight', zeroblob(32), 0),
  (9, 'g9', 2, '[\"nine\"]', 'nine', zeroblob(32), 0);
INSERT INTO cards VALUES (10, 1, 0, 1, NULL, 0, NULL, 0, 0), (11, 2, 0, 1, NULL, 0, NULL, 0, 0),
  (12, 3, 0, 2, NULL, 0, NULL, 0, 0), (13, 4, 0, 2, NULL, 0, NULL, 0, 0),
  (14, 5, 0, 1, NULL, 0, NULL, 0, 0), (15, 6, 0, 1, NULL, 0, NULL, 0, 0),
  (16, 7, 0, 1, NULL, 0, NULL, 0, 0), (17, 8, 0, 1, NULL, 0, NULL, 0, 0),
  (18, 9, 0, 3, NULL, 0, NULL, 0, 0);
INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease,
  duration_ms, study_day, server_seq) VALUES
  (100, zeroblob(16), 1, 10, 1, 1, 1, 0, 1, 1000, 0, 1),
  (110, zeroblob(16), 2, 11, 2, 1, 1, 1, 1, 1000, 0, 2),
  (120, zeroblob(16), 3, 12, 3, 2, 1, 2, 2, 1000, 0, 3),
  (130, zeroblob(16), 4, 13, 4, 2, 1, 3, 4, 1000, 0, 4),
  (140, zeroblob(16), 5, 14, 5, 1, 1, 4, 1, 1000, 0, 5),
  (150, zeroblob(16), 6, 15, 6, 1, 1, 5, 0, 0, 0, 6),
  (160, zeroblob(16), 7, 16, 7, 1, 1, 1, 0, 1000, 0, 7),
  (170, zeroblob(16), 8, 17, 8, 1, 1, 1, 3, 1000, 0, 8),
  (180, zeroblob(16), 9, 18, 9, 3, 1, 1, 3, 1000, 0, 9),
  (190, zeroblob(16), 10, 10, 1, 1, 1, 4, 0, 0, 0, 10);
INSERT INTO review_retraction VALUES (170, zeroblob(16), 11, 0);
";

#[test]
fn each_guard_of_the_read_statements_alone_keeps_its_review_out() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("guards.store");
    drop(Store::open(&path, SERVER).expect("the store opens"));
    let loaded = raw(&path).execute_batch(GUARD_ROWS);
    let store = Store::open(&path, SERVER).expect("the store reopens");
    let scope = BTreeSet::from([1, 2]);
    let ids = |reviews: Vec<ReviewRead>| {
        format!(
            "{:?}",
            reviews.iter().map(|review| review.id).collect::<Vec<_>>()
        )
    };
    let answers = store
        .read(|read| {
            Ok(vec![
                (
                    "study reviews of decks one and two",
                    ids(read.study_reviews_after(0, &scope)?),
                    "[100, 110, 120, 130]",
                ),
                (
                    "study reviews above 110",
                    ids(read.study_reviews_after(110, &scope)?),
                    "[120, 130]",
                ),
                (
                    "study review ids of decks one and two",
                    format!("{:?}", read.study_review_ids_after(0, &scope, 10)?),
                    "[100, 110, 120, 130]",
                ),
                (
                    "study review ids above 110",
                    format!("{:?}", read.study_review_ids_after(110, &scope, 10)?),
                    "[120, 130]",
                ),
                (
                    "study review ids, two at a time",
                    format!("{:?}", read.study_review_ids_after(0, &scope, 2)?),
                    "[100, 110]",
                ),
                (
                    "study reviews through 1000",
                    format!("{:?}", read.study_reviews_through(1000, &scope)?),
                    "4",
                ),
                (
                    "study reviews through 120",
                    format!("{:?}", read.study_reviews_through(120, &scope)?),
                    "3",
                ),
                (
                    "cards studied after 0, in every deck",
                    format!("{:?}", read.studied_cards_after(0)?),
                    "{10, 11, 12, 13, 18}",
                ),
                (
                    "cards studied after 110",
                    format!("{:?}", read.studied_cards_after(110)?),
                    "{12, 13, 18}",
                ),
                (
                    "notes reviewed in decks one and two",
                    format!("{:?}", read.reviewed_notes(&scope)?),
                    "{2}",
                ),
                (
                    "cards of decks one and two",
                    format!(
                        "{:?}",
                        read.cards_in(&scope)?
                            .iter()
                            .map(|card| card.id)
                            .collect::<Vec<_>>()
                    ),
                    "[10, 11, 12, 13, 14, 15, 16, 17]",
                ),
                (
                    "templates of decks one and two",
                    format!(
                        "{:?}",
                        read.templates_in(&scope)?
                            .iter()
                            .map(|template| (template.notetype_id, template.name.clone()))
                            .collect::<Vec<_>>()
                    ),
                    "[(1, \"Card 1\")]",
                ),
                (
                    "fields of decks one and two",
                    format!(
                        "{:?}",
                        read.fields_in(&scope)?
                            .iter()
                            .map(|field| (field.notetype_id, field.name.clone()))
                            .collect::<Vec<_>>()
                    ),
                    "[(1, \"Front\")]",
                ),
                (
                    "fields of note 2",
                    format!(
                        "{:?}",
                        read.note_fields(&BTreeSet::from([2]))?
                            .iter()
                            .map(|note| note.id)
                            .collect::<Vec<_>>()
                    ),
                    "[2]",
                ),
                (
                    "rescheduling rows",
                    format!("{:?}", read.counts()?.reschedule_rows),
                    "1",
                ),
            ])
        })
        .expect("the reads run");
    let mut findings: Vec<String> = loaded
        .err()
        .map(|error| format!("the fixture rows were refused: {error}"))
        .into_iter()
        .collect();
    for (what, answered, expected) in examined("guarded read(s)", answers) {
        if answered != expected {
            findings.push(format!("{what}: answered {answered}, expected {expected}"));
        }
    }
    assert_eq!(
        findings,
        Vec::<String>::new(),
        "each guard of the read statements"
    );
}

/// One deck, two notes, three cards (two of the first note), and four review rows: two study
/// answers, a retracted one, and a rescheduling row, so no two counts are equal.
const COUNT_ROWS: &str = "
INSERT INTO notetypes VALUES (1, 'Basic', 'basic', 0, '', 0, '{}', 0);
INSERT INTO presets VALUES (1, 'Default', '{}', 0);
INSERT INTO decks VALUES (1, 'One', 'one', 1, 0, '{}', 0);
INSERT INTO notes VALUES (1, 'g1', 1, '[\"one\"]', 'one', zeroblob(32), 0),
  (2, 'g2', 1, '[\"two\"]', 'two', zeroblob(32), 0);
INSERT INTO cards VALUES (10, 1, 0, 1, NULL, 0, NULL, 0, 0), (11, 1, 1, 1, NULL, 0, NULL, 0, 0),
  (12, 2, 0, 1, NULL, 0, NULL, 0, 0);
INSERT INTO review_log (id, origin, seq, card_id, note_id, deck_id, preset_id, kind, ease,
  duration_ms, study_day, server_seq) VALUES
  (100, zeroblob(16), 1, 10, 1, 1, 1, 1, 3, 1000, 0, 1),
  (110, zeroblob(16), 2, 11, 1, 1, 1, 0, 1, 1000, 0, 2),
  (120, zeroblob(16), 3, 12, 2, 1, 1, 1, 2, 1000, 0, 3),
  (130, zeroblob(16), 4, 10, 1, 1, 1, 4, 0, 0, 0, 4);
INSERT INTO review_retraction VALUES (120, zeroblob(16), 5, 0);
";

#[test]
fn each_count_of_the_store_reads_its_own_table() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("counts.store");
    drop(Store::open(&path, SERVER).expect("the store opens"));
    let loaded = raw(&path).execute_batch(COUNT_ROWS);
    let store = Store::open(&path, SERVER).expect("the store reopens");
    let mut findings: Vec<String> = loaded
        .err()
        .map(|error| format!("the fixture rows were refused: {error}"))
        .into_iter()
        .collect();
    let counts = store
        .read(|read| read.counts())
        .map(|counts| format!("{counts:?}"));
    let expected = format!(
        "{:?}",
        LogCounts {
            cards: 3,
            notes: 2,
            review_rows: 4,
            reschedule_rows: 1,
        }
    );
    match counts {
        Ok(counts) if counts == expected => {}
        other => findings.push(format!(
            "the counts answered {other:?}, expected {expected}"
        )),
    }
    assert_eq!(findings, Vec::<String>::new(), "the store's counts");
}
