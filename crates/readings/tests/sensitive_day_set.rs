//! The day set holds back the decks the learner keeps away from AI (SPEC-381 A7, R3): every card
//! whose home deck or current deck, or an ancestor of either, is marked is held back before the day
//! set is resolved, and every other card is kept.
//!
//! The deck tree, the taxonomy and the queued cards are synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_ingest::settings::DECK_SEPARATOR;
use deck_streak_readings::day_set::{
    DaySetQuery, QueuedCard, hold_back_sensitive, resolve_day_sets,
};
use deck_streak_readings::taxonomy::Taxonomy;

/// A language root the learner keeps away from AI.
const ALPHA: i64 = 1;
/// A deck under it.
const ALPHA_VERBS: i64 = 2;
/// A language root never marked.
const BETA: i64 = 3;
/// A filtered deck, which borrows cards from the others.
const FILTERED: i64 = 4;

/// A taxonomy whose two language decks are the two roots.
const TAXONOMY: &str = r#"{
  "schema": "deckstreak.readings.taxonomy.v1",
  "law": {"roots": ["Casebook"], "bands": []},
  "languages": [
    {"deck": "Alpha", "code": "qaa", "display": "Alpha", "term_field": "Headword"},
    {"deck": "Beta", "code": "qab", "display": "Beta", "term_field": "Headword"}
  ],
  "writing_roots": []
}"#;

/// A queued card `id` sitting in `deck`, borrowed from `original` when it is not 0.
const fn card(id: i64, deck: i64, original: i64) -> QueuedCard {
    QueuedCard {
        id,
        note_id: Some(id),
        deck_id: deck,
        original_deck_id: original,
    }
}

#[test]
fn a_kept_away_decks_cards_are_never_selected_into_a_day_set() {
    let deck_names = BTreeMap::from([
        (ALPHA, "Alpha".to_owned()),
        (ALPHA_VERBS, format!("Alpha{DECK_SEPARATOR}Verbs")),
        (BETA, "Beta".to_owned()),
        (FILTERED, "Filtered".to_owned()),
    ]);
    let queries = vec![
        DaySetQuery {
            root: "Alpha".to_owned(),
            cards: vec![
                card(101, ALPHA, 0),
                card(102, ALPHA_VERBS, 0),
                card(103, FILTERED, ALPHA_VERBS),
            ],
        },
        DaySetQuery {
            root: "Beta".to_owned(),
            cards: vec![card(301, BETA, 0), card(302, FILTERED, BETA)],
        },
    ];
    let kept = hold_back_sensitive(queries, &deck_names, &BTreeSet::from([ALPHA]));
    let selected: Vec<(&str, Vec<i64>)> = kept
        .iter()
        .map(|query| {
            (
                query.root.as_str(),
                query.cards.iter().map(|card| card.id).collect(),
            )
        })
        .collect();
    assert_eq!(
        selected,
        [("Alpha", vec![]), ("Beta", vec![301, 302])],
        "every card of the kept-away root, of its child and borrowed from it is held back"
    );
    // What is kept is resolved as before: the unmarked root's cards, the borrowed one too, are its
    // topic's day set, and no held card reaches any day set.
    let taxonomy = Taxonomy::parse(TAXONOMY).expect("the taxonomy");
    let resolution = resolve_day_sets(&kept, &deck_names, &taxonomy, &[]);
    let day_sets: Vec<(&str, Vec<i64>)> = resolution
        .active
        .iter()
        .map(|day_set| (day_set.topic.as_str(), day_set.card_ids.clone()))
        .collect();
    assert_eq!(day_sets, [("language/qab", vec![301, 302])]);
}
