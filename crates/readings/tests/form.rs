//! The form of a reading and its word target (SPEC-046 A1, R3, R4).

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_readings::form::{
    BAND_CEILING_WORDS, BAND_FLOOR_WORDS, Form, WORDS_PER_NEW_CARD, corpus_json, word_target,
};
use deck_streak_readings::seed::{Seed, SeedNote, Track};

#[test]
fn the_word_target_grows_with_new_cards_inside_the_band() {
    assert_eq!(BAND_FLOOR_WORDS, 800, "the band's floor");
    assert_eq!(BAND_CEILING_WORDS, 1500, "the band's ceiling");
    assert_eq!(WORDS_PER_NEW_CARD, 50, "the words each new card adds");
    assert_eq!(word_target(1), 800, "one card sits on the floor");
    assert_eq!(word_target(2), 850, "each further card adds fifty words");
    assert_eq!(word_target(3), 900);
    assert_eq!(
        word_target(15),
        1500,
        "the ceiling is reached at fifteen cards"
    );
    assert_eq!(word_target(14), 1450, "and not before");
    assert_eq!(word_target(16), 1500, "the ceiling holds");
    assert_eq!(word_target(10_000), 1500, "however many cards");
    assert_eq!(word_target(u32::MAX), 1500, "at the widest count");
    assert_eq!(word_target(0), 800, "no card is read as one");
    let mut last = 0;
    for n in 1..=40 {
        let target = word_target(n);
        assert!(target >= last, "the target never shrinks: {n}");
        assert!(
            (800..=1500).contains(&target),
            "the target stays in the band: {n}"
        );
        last = target;
    }
    println!("examined 40 word targets");
}

fn seed() -> Seed {
    Seed {
        card_ids: vec![11, 12, 13],
        notes: vec![
            SeedNote {
                id: 401,
                text: "Relevance is a low bar".to_owned(),
            },
            SeedNote {
                id: 402,
                text: "Hearsay is an out-of-court statement".to_owned(),
            },
        ],
        new_words: vec!["casa".to_owned(), "perro".to_owned()],
    }
}

#[test]
fn a_law_form_lists_its_sections_with_the_retrieval_last() {
    let form = Form::of(Track::Law);
    assert_eq!(
        form.sections(),
        [
            "reading",
            "issue",
            "rule",
            "application",
            "conclusion",
            "retrieval"
        ],
        "the law form's sections, in order"
    );
    assert_eq!(
        form.list_checked(),
        ["reading", "issue", "rule", "application", "conclusion"],
        "every law section but the retrieval is prose"
    );
}

#[test]
fn a_language_form_lists_its_sections_with_the_retrieval_last() {
    let form = Form::of(Track::Language);
    assert_eq!(
        form.sections(),
        [
            "reading",
            "glosses",
            "grammar",
            "pronunciation",
            "culture",
            "retrieval"
        ],
        "the language form's sections, in order"
    );
    assert_eq!(
        form.list_checked(),
        ["reading"],
        "the glosses and the retrieval are lists by their packs' contracts"
    );
}

#[test]
fn the_engine_writes_the_frontmatter_keys_the_gates_read() {
    let law = Form::of(Track::Law).frontmatter_extra(&seed());
    assert_eq!(law, "sources: [\"n401\", \"n402\"]\nx-new-cards: 3\n");
    let language = Form::of(Track::Language).frontmatter_extra(&seed());
    assert_eq!(language, "x-new-words: [\"casa\", \"perro\"]\n");
}

#[test]
fn the_instruction_names_the_sections_and_never_the_rejected_text() {
    let instruction = Form::of(Track::Law).instruction(&seed());
    for section in [
        "reading",
        "issue",
        "rule",
        "application",
        "conclusion",
        "retrieval",
    ] {
        assert!(
            instruction.contains(&format!("<!-- section:{section} -->")),
            "the instruction marks the {section} section"
        );
    }
    assert!(
        instruction.contains("[@n401]"),
        "the instruction names each note's citation key"
    );
}

#[test]
fn the_corpus_names_the_subject_and_every_note_as_a_source() {
    let seed = Seed {
        card_ids: vec![1, 2],
        notes: vec![
            SeedNote {
                id: 401,
                text: "A synthetic rule".to_owned(),
            },
            SeedNote {
                id: 402,
                text: "Another synthetic rule".to_owned(),
            },
        ],
        new_words: Vec::new(),
    };
    let corpus: serde_json::Value =
        serde_json::from_str(&corpus_json(&seed, "law/evidence")).expect("the corpus is JSON");
    assert_eq!(corpus["schema"], "phx.law.corpus.v1");
    assert_eq!(corpus["subject"], "law/evidence");
    let sources = corpus["sources"].as_array().expect("the sources");
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0]["id"], "n401");
    assert_eq!(sources[0]["title"], "n401");
    assert_eq!(sources[0]["text"], "A synthetic rule");
    assert_eq!(sources[1]["id"], "n402");
    assert_eq!(sources[1]["text"], "Another synthetic rule");
}
