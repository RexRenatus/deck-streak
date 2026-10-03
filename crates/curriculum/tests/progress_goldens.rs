//! Mastery, the unit parse, course progress and the curriculum constants equal the
//! predecessor's (SPEC-077 A2, A3, A4, A5).

// An integration test is test code: it panics on a malformed golden and prints the examined count.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::BTreeMap;

use deck_streak_curriculum::progress::{self, course_progress};
use deck_streak_ingest::memory_state::{self, MemoryState};
use deck_streak_ingest::reader::Card;
use deck_streak_kernel::courses::CEFR_BANDS;
use deck_streak_kernel::{CourseCode, Courses, Track};
use serde_json::{Value, json};

/// Whether two floats agree within the golden's tolerance.
fn near(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9
}

/// A whole number under `key`, or `default`.
fn whole(input: &Value, key: &str, default: i64) -> i64 {
    input.get(key).and_then(Value::as_i64).unwrap_or(default)
}

/// The card a golden input describes.
fn card_of(input: &Value, course: Option<CourseCode>) -> Card {
    let memory = input
        .get("stability")
        .and_then(Value::as_f64)
        .map(|stability| MemoryState {
            stability,
            difficulty: 0.0,
            decay: input
                .get("decay")
                .and_then(Value::as_f64)
                .unwrap_or(memory_state::DEFAULT_DECAY),
            desired_retention: None,
            last_review_sec: input.get("last_review_sec").and_then(Value::as_i64),
        });
    Card {
        id: 1,
        note_id: 1,
        deck_id: whole(input, "did", 1),
        original_deck_id: whole(input, "odid", 0),
        queue: whole(input, "queue", 0),
        kind: whole(input, "ctype", 0),
        due: 0,
        interval: whole(input, "ivl", 0),
        factor: 2500,
        reps: 0,
        lapses: 0,
        track: Track::Language,
        course,
        tier: None,
        memory,
    }
}

#[test]
fn card_mastery_matches_the_predecessors_golden() {
    let examined = golden::each_case("card_mastery", |case| {
        let card = card_of(&case.input["card"], None);
        let ours = progress::card_mastery(
            &card,
            whole(&case.input, "now_sec", 0),
            whole(
                &case.input,
                "mature_ivl",
                progress::PROGRESS_MATURE_IVL_DAYS,
            ),
        );
        let theirs = case.output.as_f64().expect("a mastery");
        assert!(
            near(ours, theirs),
            "the mastery of {}: ours {ours}, theirs {theirs}",
            case.input
        );
    });
    assert!(examined.count > 0);
}

#[test]
fn the_unit_parse_matches_the_predecessors_golden() {
    let examined = golden::each_case("unit_parse", |case| {
        let name = case.input["deck_name"].as_str().expect("a deck name");
        let ours = progress::parse_unit(name).map(i64::from);
        assert_eq!(ours, case.output.as_i64(), "the unit of {name:?}");
    });
    assert!(examined.count > 0);
    // A unit beyond u32 is no unit: the predecessor's integer falls in no configured band, so its
    // card is not counted (the course progress golden's `unit_beyond_u32` case).
    assert_eq!(
        progress::parse_unit("Unit 4294967296"),
        None,
        "a unit beyond u32 is no unit"
    );
}

/// The courses file text for a golden's courses.
fn courses_text(courses: &[Value]) -> String {
    let aliases = ['a', 'b', 'g', 'd', 'e', 'f'];
    let list: Vec<Value> = courses
        .iter()
        .enumerate()
        .map(|(index, course)| {
            json!({
                "code": course["code"],
                "name": course["name"],
                "flag": course["flag"],
                "deck_root": course["deck_root"],
                "alias": aliases[index].to_string(),
                "writing": false,
                "unit_bands": course["bands"],
            })
        })
        .collect();
    json!({"schema": "deckstreak.courses.v1", "courses": list}).to_string()
}

#[test]
fn course_progress_matches_the_predecessors_golden() {
    let examined = golden::each_case("course_progress", |case| {
        let courses_in = case.input["courses"].as_array().expect("courses");
        let courses = Courses::parse(&courses_text(courses_in)).expect("the courses parse");
        let decks: BTreeMap<i64, String> = case.input["decks"]
            .as_object()
            .expect("decks")
            .iter()
            .map(|(id, name)| {
                (
                    id.parse().expect("a deck id"),
                    name.as_str().expect("a name").to_owned(),
                )
            })
            .collect();
        let cards: Vec<Card> = case.input["cards"]
            .as_array()
            .expect("cards")
            .iter()
            .map(|input| {
                let did = whole(input, "odid", 0);
                let did = if did == 0 {
                    whole(input, "did", 0)
                } else {
                    did
                };
                let root = decks.get(&did).and_then(|name| name.split('\u{1f}').next());
                let course = courses_in
                    .iter()
                    .find(|course| Some(course["deck_root"].as_str().unwrap_or_default()) == root)
                    .and_then(|course| {
                        CourseCode::new(course["code"].as_str().unwrap_or_default())
                    });
                card_of(input, course)
            })
            .collect();
        let ours = course_progress(&cards, &decks, &courses, whole(&case.input, "now_sec", 0));
        let theirs = case.output.as_array().expect("progress list");
        assert_eq!(ours.len(), theirs.len(), "the courses of {}", case.input);
        for (mine, want) in ours.iter().zip(theirs) {
            assert_eq!(mine.code.as_str(), want["code"].as_str().expect("code"));
            assert_eq!(mine.name, want["name"].as_str().expect("name"));
            assert_eq!(mine.flag, want["flag"].as_str().expect("flag"));
            assert_eq!(
                i64::from(mine.total_cards),
                want["total_cards"].as_i64().expect("total")
            );
            assert_eq!(
                i64::from(mine.mature_cards),
                want["mature_cards"].as_i64().expect("mature")
            );
            assert!(near(
                mine.mastery_pct,
                want["mastery_pct"].as_f64().expect("pct")
            ));
            assert_eq!(
                mine.current_band,
                want["current_band"].as_str().expect("band")
            );
            assert_eq!(
                mine.current_unit.map(i64::from),
                want["current_unit"].as_i64()
            );
            let bands = want["bands"].as_array().expect("bands");
            assert_eq!(mine.bands.len(), bands.len());
            for (band, want) in mine.bands.iter().zip(bands) {
                assert_eq!(band.band, want["band"].as_str().expect("band name"));
                assert_eq!(
                    i64::from(band.total),
                    want["total"].as_i64().expect("total")
                );
                assert_eq!(
                    i64::from(band.mature),
                    want["mature"].as_i64().expect("mature")
                );
                assert!(near(band.pct, want["pct"].as_f64().expect("pct")));
                assert_eq!(band.achieved, want["achieved"].as_bool().expect("achieved"));
            }
        }
    });
    assert!(examined.count > 0);
}

#[test]
fn the_curriculum_constants_equal_the_predecessors() {
    let examined = golden::each_case("curriculum.constants", |case| {
        let name = case.input["name"].as_str().expect("a name");
        let ours: Value = match name {
            "curriculum.CEFR_BANDS" => json!(CEFR_BANDS),
            "curriculum.CEFR_BAND_ACHIEVED_PCT" => json!(progress::CEFR_BAND_ACHIEVED_PCT),
            "curriculum.PROGRESS_MATURE_IVL_DAYS" => json!(progress::PROGRESS_MATURE_IVL_DAYS),
            "curriculum.PROGRESS_STABILITY_TARGET_DAYS" => {
                json!(progress::PROGRESS_STABILITY_TARGET_DAYS)
            }
            "curriculum.MATURE_MASTERY_THRESHOLD" => json!(progress::MATURE_MASTERY_THRESHOLD),
            "fsrs.DEFAULT_DECAY" => json!(memory_state::DEFAULT_DECAY),
            "fsrs._MIN_ABS_DECAY" => json!(progress::MIN_ABS_DECAY),
            "fsrs._MAX_ABS_DECAY" => json!(progress::MAX_ABS_DECAY),
            "constants.XP_BONUS_BAND_UP" => json!(progress::XP_BONUS_BAND_UP),
            "constants.MASTERY_LEECH_PENALTY" => {
                json!(deck_streak_curriculum::law::MASTERY_LEECH_PENALTY)
            }
            "constants.MASTERY_LEECH_PENALTY_CAP" => {
                json!(deck_streak_curriculum::law::MASTERY_LEECH_PENALTY_CAP)
            }
            other => panic!("a constant the golden names and nothing here pins: {other}"),
        };
        let same = match (ours.as_f64(), case.output.as_f64()) {
            (Some(a), Some(b)) => a.to_bits() == b.to_bits(),
            _ => ours == case.output,
        };
        assert!(same, "{name}: ours {ours}, theirs {}", case.output);
    });
    assert!(examined.count > 0);
}
