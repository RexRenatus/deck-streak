//! The badge catalog is the predecessor's (SPEC-073 A1, A2; R2): 40 badges with their key, name,
//! emoji, description and tier, the five course-named descriptions rendered from the configured
//! courses.

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_kernel::Courses;
use deck_streak_progression::badges::catalog::catalog;

/// The five badges whose descriptions name the predecessor's own courses (R2).
const COURSE_NAMED: [&str; 5] = [
    "ink_week",
    "ink_month",
    "ink_century",
    "bookworm_week",
    "polyglot_reader",
];

/// Two synthetic courses, one carrying the writing habit.
const COURSES: &str = r#"{
  "schema": "deckstreak.courses.v1",
  "courses": [
    {"code": "qaa", "name": "Course Qaa", "flag": "F", "deck_root": "Qaa", "alias": "a",
     "writing": false, "unit_bands": {"A1": [1, 5]}},
    {"code": "qab", "name": "Course Qab", "flag": "F", "deck_root": "Qab Deck", "alias": "b",
     "writing": true, "unit_bands": {}}
  ],
  "focus_subjects": []
}"#;

fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

#[test]
fn the_catalog_matches_the_parity_golden() {
    let ours = catalog(&courses());
    let mut compared = 0_usize;
    golden::each_case("badge_catalog", |case| {
        let rows = case.output.as_array().expect("the catalog's rows");
        assert_eq!(ours.len(), rows.len(), "the catalog's size");
        for (badge, row) in ours.iter().zip(rows) {
            let field = |at: usize| row[at].as_str().expect("a text field").to_owned();
            assert_eq!(badge.key, field(0), "the key at {compared}");
            assert_eq!(badge.name, field(1), "the name of {}", badge.key);
            assert_eq!(badge.emoji, field(2), "the emoji of {}", badge.key);
            if !COURSE_NAMED.contains(&badge.key.as_str()) {
                assert_eq!(
                    badge.description,
                    field(3),
                    "the description of {}",
                    badge.key
                );
            }
            assert_eq!(
                i64::from(badge.tier),
                row[4].as_i64().expect("a tier"),
                "the tier of {}",
                badge.key
            );
            compared += 1;
        }
    });
    println!("examined {compared} badge(s), refused 0");
    assert_eq!(compared, 40, "every badge of the catalog is compared");
}

#[test]
fn the_course_named_descriptions_come_from_the_configured_courses() {
    let ours = catalog(&courses());
    let description = |key: &str| {
        ours.iter()
            .find(|badge| badge.key == key)
            .map(|badge| badge.description.clone())
            .unwrap_or_default()
    };
    let expected = [
        ("ink_week", "7-day Course Qab writing streak"),
        ("ink_month", "30-day Course Qab writing streak"),
        ("ink_century", "100-day Course Qab writing streak"),
        (
            "bookworm_week",
            "Hit the reading goal in Course Qaa and Course Qab",
        ),
        ("polyglot_reader", "Read in all 2 courses in one week"),
    ];
    println!("examined {} course-named description(s)", expected.len());
    for (key, words) in expected {
        assert_eq!(description(key), words, "the description of {key}");
    }
    // No configured course: the generic wording, naming no course at all.
    let bare = catalog(&Courses::default());
    let generic = |key: &str| {
        bare.iter()
            .find(|badge| badge.key == key)
            .map(|badge| badge.description.clone())
            .unwrap_or_default()
    };
    assert_eq!(generic("ink_week"), "7-day writing streak");
    assert_eq!(
        generic("bookworm_week"),
        "Hit the reading goal in every course"
    );
    assert_eq!(
        generic("polyglot_reader"),
        "Read in every course in one week"
    );
    for key in COURSE_NAMED {
        assert!(
            !generic(key).contains("CJK"),
            "{key} names no course of the predecessor's"
        );
    }
}
