//! A unit outside every configured band is not counted (SPEC-077 A6).

// An integration test is test code.
#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use deck_streak_curriculum::progress::course_progress;
use deck_streak_curriculum::unit_bands::band_for_unit;
use deck_streak_ingest::reader::Card;
use deck_streak_kernel::{CourseCode, Courses, Track};

const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[{"code":"be","name":"Beta",
"flag":"b","deck_root":"Beta Course","alias":"b","writing":false,
"unit_bands":{"A1":[1,4],"B1":[10,14],"C2":[20,20]}}]}"#;

fn card(did: i64) -> Card {
    Card {
        id: did,
        note_id: did,
        deck_id: did,
        original_deck_id: 0,
        queue: 2,
        kind: 2,
        due: 0,
        interval: 30,
        factor: 2500,
        reps: 0,
        lapses: 0,
        track: Track::Language,
        course: CourseCode::new("be"),
        tier: None,
        memory: None,
    }
}

#[test]
fn a_unit_outside_every_configured_band_is_not_counted() {
    let courses = Courses::parse(COURSES).expect("the courses parse");
    let course = &courses.courses()[0];
    assert_eq!(band_for_unit(course, 3), Some("A1"));
    assert_eq!(band_for_unit(course, 12), Some("B1"));
    assert_eq!(band_for_unit(course, 20), Some("C2"));
    for gap in [0, 5, 9, 15, 19, 21, 1000] {
        assert_eq!(
            band_for_unit(course, gap),
            None,
            "unit {gap} sits in no band"
        );
    }
    let decks: BTreeMap<i64, String> = BTreeMap::from([
        (1, "Beta Course\u{1f}Unit 07".to_owned()),
        (2, "Beta Course\u{1f}Unit 03".to_owned()),
        (3, "Beta Course\u{1f}no unit".to_owned()),
    ]);
    let cards = [card(1), card(2), card(3)];
    let progress = course_progress(&cards, &decks, &courses, 1_700_000_000);
    assert_eq!(progress.len(), 1, "one course holds a counted card");
    assert_eq!(
        progress[0].total_cards, 1,
        "only the unit inside a band counts"
    );
}
