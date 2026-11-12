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

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

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

/// What an in-test subscriber saw: one entry per event, holding its `held_back` field when it
/// carried one.
#[derive(Clone, Default)]
struct Events(std::sync::Arc<std::sync::Mutex<Vec<Option<u64>>>>);

/// Reads the `held_back` field of one event.
struct HeldBack(Option<u64>);

impl tracing::field::Visit for HeldBack {
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "held_back" {
            self.0 = Some(value);
        }
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
}

impl tracing::Subscriber for Events {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let mut seen = HeldBack(None);
        event.record(&mut seen);
        self.0.lock().expect("the event list").push(seen.0);
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

/// The deck tree of the logging tests: `ALPHA` and its child, `BETA`, and a filtered deck.
fn logging_tree() -> BTreeMap<i64, String> {
    BTreeMap::from([
        (ALPHA, "Alpha".to_owned()),
        (ALPHA_VERBS, format!("Alpha{DECK_SEPARATOR}Verbs")),
        (BETA, "Beta".to_owned()),
        (FILTERED, "Filtered".to_owned()),
    ])
}

/// Runs `hold_back_sensitive` with `marked`, and answers the events it logged. The binding keeps
/// the lock's guard from outliving `events`.
#[allow(clippy::let_and_return)]
fn events_of(queries: Vec<DaySetQuery>, marked: &BTreeSet<i64>) -> Vec<Option<u64>> {
    let events = Events::default();
    let kept = log_capture::with_capture(events.clone(), || {
        hold_back_sensitive(queries, &logging_tree(), marked)
    });
    drop(kept);
    let seen = events.0.lock().expect("the event list").clone();
    seen
}

#[test]
fn the_cards_held_back_are_logged_once_as_one_count() {
    // Three cards are held back over two queries and two are kept: a count that is added up
    // wrongly, or taken from the kept cards, differs from 3.
    let queries = vec![
        DaySetQuery {
            root: "Alpha".to_owned(),
            cards: vec![
                card(101, ALPHA, 0),
                card(102, ALPHA_VERBS, 0),
                card(103, BETA, 0),
            ],
        },
        DaySetQuery {
            root: "Beta".to_owned(),
            cards: vec![card(301, BETA, 0), card(302, FILTERED, ALPHA)],
        },
    ];
    assert_eq!(events_of(queries, &BTreeSet::from([ALPHA])), [Some(3)]);
}

#[test]
fn nothing_held_back_logs_nothing() {
    let queries = vec![DaySetQuery {
        root: "Beta".to_owned(),
        cards: vec![card(301, BETA, 0), card(302, FILTERED, BETA)],
    }];
    let events = events_of(queries, &BTreeSet::from([ALPHA]));
    assert_eq!(events, Vec::<Option<u64>>::new());
}
