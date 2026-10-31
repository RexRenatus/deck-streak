//! Each term the evaluator reads answers the cards its search names, held one term at a time over
//! small synthetic collections beside the engine's goldens (SPEC-514 R5): `selects` answers false
//! for a card the query leaves out and for an index past the cards; a deck search lists its own
//! cards by deck, queue and due, then the cards borrowed from it by original deck; the states,
//! a learning card's `prop:due`, `added:` and `rated:` hold at their boundaries; and the field,
//! note type and tag searches read names, fields and ranges as the pinned engine's search writer
//! does (`rslib/src/search/sqlwriter.rs`).

use deck_streak_search::{
    Card, Collection, Deck, EngineDay, Note, Notetype, Profile, Query, Review, Term, eval, selects,
};

/// A day on which day 100 is today, the next day begins at second 1,000,000 and it is now second
/// 990,000. One study day before the next day begins is second 913,600, so `added:1` and
/// `rated:1` count from millisecond 913,600,000.
const TODAY: EngineDay = EngineDay {
    days_elapsed: 100,
    next_day_at: 1_000_000,
    now: 990_000,
};

/// Parses under the operator profile, panicking with the refusal.
fn parsed(search: &str) -> Query {
    Profile::OPERATOR
        .parse(search)
        .unwrap_or_else(|refusal| panic!("{search:?} parses: {refusal}"))
}

/// A new card of note 0 in deck 2, every other number 0.
fn card(id: i64) -> Card {
    Card {
        id,
        note: 0,
        deck_id: 2,
        original_deck_id: 0,
        ord: 0,
        kind: 0,
        queue: 0,
        due: 0,
        original_due: 0,
        interval: 0,
        reps: 0,
        lapses: 0,
    }
}

fn deck(id: i64, name: &str) -> Deck {
    Deck {
        id,
        name: name.to_owned(),
    }
}

fn notetype(name: &str, fields: &[&str]) -> Notetype {
    Notetype {
        id: 1,
        name: name.to_owned(),
        fields: fields.iter().map(|&field| field.to_owned()).collect(),
        templates: vec!["Forward".to_owned()],
        sort_field: 0,
    }
}

fn note(notetype: usize, fields: &[&str], tags: &[&str]) -> Note {
    Note {
        id: 1,
        notetype,
        fields: fields.iter().map(|&field| field.to_owned()).collect(),
        tags: tags.iter().map(|&tag| tag.to_owned()).collect(),
    }
}

/// One note type with fields `fields`, and one card of each note, in order.
fn with_notes(fields: &[&str], notes: Vec<Note>) -> Collection {
    let cards = (0..notes.len())
        .map(|index| Card {
            note: index,
            ..card(i64::try_from(index).unwrap_or(i64::MAX) + 1)
        })
        .collect();
    Collection {
        decks: vec![deck(2, "A")],
        notetypes: vec![notetype("Basic", fields)],
        notes,
        cards,
        revlog: Vec::new(),
    }
}

/// The decks and cards given, over one note type and the one note every card reads.
fn with_cards(decks: Vec<Deck>, cards: Vec<Card>) -> Collection {
    Collection {
        decks,
        notetypes: vec![notetype("Basic", &["Prompt"])],
        notes: vec![note(0, &["amber"], &[])],
        cards,
        revlog: Vec::new(),
    }
}

#[test]
fn selects_answers_false_for_a_card_left_out_and_for_an_index_past_the_cards() {
    let rows = with_cards(
        vec![deck(2, "A"), deck(3, "B")],
        vec![
            card(1),
            Card {
                deck_id: 3,
                ..card(2)
            },
        ],
    );
    let query = parsed("deck:A");
    assert!(selects(&query, &rows, TODAY, 0), "the card in deck A");
    assert!(!selects(&query, &rows, TODAY, 1), "the card in deck B");
    assert!(
        !selects(&query, &rows, TODAY, 2),
        "the index one past the last card"
    );
}

#[test]
fn a_deck_search_lists_its_own_cards_by_queue_and_due_then_the_borrowed_by_original_deck() {
    // Deck 2 is a filtered deck holding two cards borrowed from A (3) and from its child A::B (4).
    let rows = with_cards(
        vec![deck(2, "F"), deck(3, "A"), deck(4, "A::B")],
        vec![
            Card {
                original_deck_id: 4,
                queue: 2,
                due: 1,
                ..card(1)
            },
            Card {
                original_deck_id: 3,
                queue: 2,
                due: 1,
                ..card(2)
            },
            Card {
                deck_id: 3,
                queue: 2,
                due: 10,
                ..card(3)
            },
            Card {
                deck_id: 3,
                queue: 2,
                due: 5,
                ..card(4)
            },
        ],
    );
    assert_eq!(eval(&parsed("deck:A"), &rows, TODAY), vec![3, 2, 1, 0]);
}

#[test]
fn the_every_deck_and_filtered_searches_list_their_cards_by_id() {
    // Deck 3 is named Filtered and holds two cards borrowed from A; the third card sits in A.
    let rows = with_cards(
        vec![deck(2, "A"), deck(3, "Filtered")],
        vec![
            Card {
                deck_id: 3,
                original_deck_id: 2,
                queue: 2,
                due: 9,
                ..card(1)
            },
            Card {
                deck_id: 3,
                original_deck_id: 2,
                queue: 2,
                due: 1,
                ..card(2)
            },
            card(3),
        ],
    );
    assert_eq!(eval(&parsed("deck:*"), &rows, TODAY), vec![0, 1, 2]);
    assert_eq!(eval(&parsed("deck:filtered"), &rows, TODAY), vec![0, 1]);
}

#[test]
fn a_deck_search_selects_a_card_borrowed_from_it_and_no_card_borrowed_from_another() {
    // Deck 4 is a filtered deck holding one card borrowed from A (2) and one from B (3).
    let rows = with_cards(
        vec![deck(2, "A"), deck(3, "B"), deck(4, "F")],
        vec![
            Card {
                deck_id: 4,
                original_deck_id: 2,
                ..card(1)
            },
            Card {
                deck_id: 4,
                original_deck_id: 3,
                ..card(2)
            },
            card(3),
            Card {
                deck_id: 3,
                ..card(4)
            },
        ],
    );
    assert_eq!(eval(&parsed("deck:A"), &rows, TODAY), vec![2, 0]);
}

#[test]
fn is_new_and_is_due_hold_through_today_and_the_learn_ahead_limit() {
    let rows = with_cards(
        vec![deck(2, "A")],
        vec![
            Card {
                kind: 2,
                queue: 2,
                due: 100,
                ..card(1)
            },
            Card {
                kind: 2,
                queue: 2,
                due: 101,
                ..card(2)
            },
            Card {
                kind: 1,
                queue: 1,
                due: 991_200,
                ..card(3)
            },
            Card {
                kind: 1,
                queue: 1,
                due: 991_201,
                ..card(4)
            },
            card(5),
        ],
    );
    assert_eq!(
        eval(&parsed("is:due"), &rows, TODAY),
        vec![0, 2],
        "the review card due today and the learning card due 1,200 seconds from now"
    );
    assert_eq!(eval(&parsed("is:new"), &rows, TODAY), vec![4]);
}

#[test]
fn prop_due_counts_a_learning_cards_days_from_the_next_days_start() {
    // Due one day and two days after the next day begins.
    let rows = with_cards(
        vec![deck(2, "A")],
        vec![
            Card {
                kind: 1,
                queue: 1,
                due: 1_086_400,
                ..card(1)
            },
            Card {
                kind: 1,
                queue: 1,
                due: 1_172_800,
                ..card(2)
            },
        ],
    );
    assert_eq!(eval(&parsed("prop:due=1"), &rows, TODAY), vec![0]);
}

#[test]
fn added_selects_a_card_added_after_the_cutoff_and_not_one_added_at_it() {
    let rows = with_cards(
        vec![deck(2, "A")],
        vec![card(913_600_000), card(913_600_001)],
    );
    assert_eq!(eval(&parsed("added:1"), &rows, TODAY), vec![1]);
}

#[test]
fn rated_selects_a_card_answered_from_the_cutoff_on_by_its_own_button() {
    let mut rows = with_cards(vec![deck(2, "A")], vec![card(1), card(2), card(3)]);
    // Card 0 answered with button 3 and card 1 rescheduled by hand (button 0), both at the cutoff.
    rows.revlog = vec![
        Review {
            id: 913_600_000,
            card: 0,
            ease: 3,
        },
        Review {
            id: 913_600_000,
            card: 1,
            ease: 0,
        },
    ];
    assert_eq!(eval(&parsed("rated:1"), &rows, TODAY), vec![0]);
    assert_eq!(eval(&parsed("rated:1:3"), &rows, TODAY), vec![0]);
}

#[test]
fn a_field_search_reads_the_named_field_alone() {
    let rows = with_notes(
        &["Prompt", "Answer"],
        vec![
            note(0, &["amber", "dune"], &[]),
            note(0, &["dune", "amber"], &[]),
        ],
    );
    assert_eq!(eval(&parsed("Prompt:amber"), &rows, TODAY), vec![0]);
    assert_eq!(eval(&parsed("Answer:amber"), &rows, TODAY), vec![1]);
}

#[test]
fn a_field_name_glob_over_adjacent_fields_reads_them_as_one_span_unless_the_text_is_any() {
    let rows = with_notes(
        &["F1", "F2"],
        vec![
            note(0, &["x", "y"], &[]),
            note(0, &["", ""], &[]),
            note(0, &["", "y"], &[]),
        ],
    );
    assert_eq!(
        eval(&parsed("F*:x*y"), &rows, TODAY),
        vec![0],
        "the text runs from the first matched field into the second"
    );
    assert_eq!(
        eval(&parsed("F*:_*"), &rows, TODAY),
        vec![0, 2],
        "each matched field is tested for a character alone"
    );
}

#[test]
fn a_note_type_search_selects_its_own_notes_alone() {
    let rows = Collection {
        notetypes: vec![notetype("Basic", &["Prompt"]), notetype("Cloze", &["Text"])],
        ..with_notes(
            &["Prompt"],
            vec![note(0, &["amber"], &[]), note(1, &["amber"], &[])],
        )
    };
    assert_eq!(eval(&parsed("note:Basic"), &rows, TODAY), vec![0]);
}

#[test]
fn a_tag_search_selects_its_tag_and_one_holding_a_space_selects_nothing() {
    let rows = with_notes(
        &["Prompt"],
        vec![note(0, &["amber"], &["ds"]), note(0, &["amber"], &["a b"])],
    );
    let tag = |tag: &str| Query::Term(Term::Tag(tag.to_owned()));
    assert_eq!(eval(&parsed("tag:ds"), &rows, TODAY), vec![0]);
    assert_eq!(
        eval(&tag("a*"), &rows, TODAY),
        vec![1],
        "the control: a glob reaches the stored tag"
    );
    assert_eq!(
        eval(&tag("a b"), &rows, TODAY),
        Vec::<usize>::new(),
        "the engine writes a tag search holding a space as false"
    );
}
