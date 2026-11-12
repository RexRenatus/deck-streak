//! The owner's courses and the readings taxonomy agree, or start is refused (SPEC-071 A15; R3;
//! ADR-087): a language deck the two private files map to different codes refuses start, naming both
//! settings and neither value. Every deck, code and name is synthetic.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_coordination::courses::{CoursesDisagree, agree};
use deck_streak_kernel::Courses;
use deck_streak_kernel::courses::COURSES_FILE;
use deck_streak_readings::taxonomy::{READINGS_TAXONOMY, Taxonomy};
use serde_json::json;

/// The repository's neutral taxonomy example: "Tongue Alpha" is `qaa`, "Tongue Beta" is `qab`.
fn taxonomy() -> Taxonomy {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../deploy/config/readings-taxonomy.example.json");
    Taxonomy::load(&path).expect("the example taxonomy loads")
}

/// Courses rooting `decks`, each `(deck root, code, alias)`.
fn courses(decks: &[(&str, &str, &str)]) -> Courses {
    let courses: Vec<_> = decks
        .iter()
        .map(|(root, code, alias)| {
            json!({"code": code, "name": "Synthetic", "flag": "F", "deck_root": root,
                   "alias": alias, "writing": false, "unit_bands": {}})
        })
        .collect();
    Courses::parse(
        &json!({"schema": "deckstreak.courses.v1", "courses": courses, "focus_subjects": []})
            .to_string(),
    )
    .expect("the synthetic courses parse")
}

#[test]
fn a_taxonomy_language_mapped_to_another_code_refuses_start() {
    let taxonomy = taxonomy();
    // Agreeing files, a course the taxonomy never names, and no taxonomy at all: start goes on.
    let agreeing = courses(&[("Tongue Alpha", "qaa", "a"), ("Tongue Beta", "qab", "b")]);
    assert_eq!(agree(&agreeing, Some(&taxonomy)), Ok(()));
    let unnamed = courses(&[("Tongue Gamma", "qac", "c")]);
    assert_eq!(agree(&unnamed, Some(&taxonomy)), Ok(()));

    // One language deck, two codes: refused, naming both settings and neither value.
    let disagreeing = courses(&[("Tongue Alpha", "qaa", "a"), ("Tongue Beta", "qzz", "b")]);
    let refusal = agree(&disagreeing, Some(&taxonomy)).expect_err("two codes for one deck");
    assert_eq!(
        refusal,
        CoursesDisagree {
            courses: COURSES_FILE,
            taxonomy: READINGS_TAXONOMY,
        }
    );
    let said = refusal.to_string();
    assert!(
        said.contains(COURSES_FILE) && said.contains(READINGS_TAXONOMY),
        "{said}"
    );
    for value in ["Tongue Beta", "qzz", "qab"] {
        assert!(!said.contains(value), "{said} quotes {value}");
    }
    // Without the taxonomy there is nothing to disagree with.
    assert_eq!(agree(&disagreeing, None), Ok(()));
}
