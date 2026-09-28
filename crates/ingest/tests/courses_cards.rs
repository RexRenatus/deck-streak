//! A card's course is the course whose deck root EQUALS its home deck's top-level name, or none
//! (SPEC-071 A12; R2; ADR-087): never a course whose root the name only starts with, and always by
//! the home deck, even while a filtered deck borrows the card. Every deck and course is synthetic.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use deck_streak_ingest::reader::course_of;
use deck_streak_ingest::settings::DECK_SEPARATOR;
use deck_streak_kernel::Courses;
use support::Fixture;
use support::synthetic::{self, PlannedCard};

const ENDPOINT: &str = "http://127.0.0.1:9/";

/// Two synthetic courses: one whose root another deck's name extends, one whose root is two words.
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

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

#[tokio::test]
async fn a_cards_course_is_the_course_whose_root_is_its_top_level_name() {
    let courses = courses();
    let sep = DECK_SEPARATOR;
    let names: Vec<(String, Option<&str>)> = vec![
        ("Qaa".to_owned(), Some("qaa")),
        (format!("Qaa{sep}Unit 01"), Some("qaa")),
        (format!("Qaa{sep}Unit 02{sep}Drill"), Some("qaa")),
        ("Qab Deck".to_owned(), Some("qab")),
        (format!("Qab Deck{sep}Unit 03"), Some("qab")),
        // A deck whose top-level name only starts with a root is in no course.
        ("Qaa Arts".to_owned(), None),
        (format!("Qaa Arts{sep}Unit 01"), None),
        ("Qab".to_owned(), None),
        // A subdeck named like a root is not the root.
        (format!("Other{sep}Qaa"), None),
        ("qaa".to_owned(), None),
        (String::new(), None),
    ];
    for (name, expected) in examined("deck names", names) {
        let course = course_of(&courses, &name);
        assert_eq!(
            course.as_ref().map(|code| code.as_str()),
            expected,
            "the course of {name:?}"
        );
    }
    // No courses configured: no card has one.
    assert_eq!(course_of(&Courses::default(), "Qaa"), None);

    // Through a read of a collection: a card's course follows its home deck, including a card a
    // filtered deck borrows.
    let fixture = Fixture::new(ENDPOINT);
    let planned = [
        PlannedCard {
            id: 1,
            deck: "Qaa::Unit 01",
            filtered: false,
        },
        PlannedCard {
            id: 2,
            deck: "Qab Deck",
            filtered: true,
        },
        PlannedCard {
            id: 3,
            deck: "Qaa Arts",
            filtered: false,
        },
        PlannedCard {
            id: 4,
            deck: "Other::Qaa",
            filtered: false,
        },
        PlannedCard {
            id: 5,
            deck: "Qaa",
            filtered: true,
        },
    ];
    synthetic::build_planned(&fixture.copy(), &[], &planned, &[]);
    let reader = synthetic::reader(&fixture.settings(), "", None, support::clock_at(0))
        .with_courses(courses);
    let data = reader.read(0).await.expect("the copy reads");
    let read: Vec<(i64, Option<String>)> = data
        .cards
        .iter()
        .map(|card| (card.id, card.course.map(|code| code.as_str().to_owned())))
        .collect();
    let expected: Vec<(i64, Option<String>)> = vec![
        (1, Some("qaa".to_owned())),
        (2, Some("qab".to_owned())),
        (3, None),
        (4, None),
        (5, Some("qaa".to_owned())),
    ];
    assert_eq!(examined("cards read", read), expected);
}
