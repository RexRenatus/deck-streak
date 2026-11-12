//! Sessions, their effort and their eligibility equal the predecessor's (SPEC-081 A1).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_ingest::reader::Review;
use deck_streak_quests::sessions::{
    RealEffort, Session, eligible_sessions, meets_chest_floor, sessions_from_reviews,
};
use serde_json::{Value, json};

/// A whole number under `key`.
fn whole(value: &Value, key: &str) -> i64 {
    value[key].as_i64().expect("a whole number in the case")
}

/// The reviews a case holds, as the ingest context reads them.
fn reviews_of(input: &Value) -> Vec<Review> {
    input["reviews"]
        .as_array()
        .expect("a case's reviews")
        .iter()
        .map(|row| Review {
            id: whole(row, "id"),
            card_id: whole(row, "card_id"),
            ease: whole(row, "ease"),
            interval: 0,
            last_interval: 0,
            factor: 2500,
            taken_ms: whole(row, "taken_ms"),
            kind: whole(row, "kind"),
        })
        .collect()
}

/// The per-answer cap the predecessor applied, as the case records it.
fn cap_of(input: &Value) -> f64 {
    input["answer_time_cap_seconds"]
        .as_f64()
        .expect("the case's per-answer cap")
}

/// A session as the golden writes it.
fn bounds_and_effort(session: &Session) -> Value {
    json!({
        "start_ms": session.start.epoch_millis(),
        "end_ms": session.end.epoch_millis(),
        "reviews": session.effort.reviews,
        "distinct_cards": session.effort.distinct_cards,
        "minutes": session.effort.minutes,
    })
}

#[test]
fn sessions_and_their_effort_match_the_predecessors_goldens() {
    let sessions = golden::each_case("sessions_from_reviews", |case| {
        let ours = sessions_from_reviews(&reviews_of(&case.input), cap_of(&case.input));
        let ours: Vec<Value> = ours.iter().map(bounds_and_effort).collect();
        assert_eq!(json!(ours), case.output, "the sessions of {}", case.input);
    });
    let eligible = golden::each_case("eligible_sessions", |case| {
        let all = sessions_from_reviews(&reviews_of(&case.input), cap_of(&case.input));
        let ours: Vec<Value> = eligible_sessions(&all)
            .iter()
            .map(|session| {
                json!({
                    "start_ms": session.start.epoch_millis(),
                    "end_ms": session.end.epoch_millis(),
                })
            })
            .collect();
        assert_eq!(json!(ours), case.output, "the eligible of {}", case.input);
    });
    let floor = golden::each_case("meets_chest_floor", |case| {
        let effort = RealEffort {
            reviews: whole(&case.input, "reviews"),
            distinct_cards: whole(&case.input, "distinct_cards"),
            minutes: case.input["minutes"].as_f64().expect("minutes"),
        };
        assert_eq!(
            json!(meets_chest_floor(&effort)),
            case.output,
            "the floor of {}",
            case.input
        );
    });
    assert!(
        sessions.count > 0 && eligible.count > 0 && floor.count > 0,
        "a sessions golden examined nothing"
    );
}
