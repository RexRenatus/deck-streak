//! Road to C2's band rules answer every vector the Lean port writes (#85): for each input in
//! `formal/vectors/road-to-c2.jsonl`, written by `formal/lean/Formal/RoadToC2.lean`'s port,
//! `course_progress` reaches the same current band, `band_step` takes the same step,
//! `mastery_pillar` gives the same pillar and `parse_unit` reads the same unit. A cross-check of
//! the proof's port against the code: MUTATION COVERAGE, never a red-first line. Every course, deck
//! and card here is synthetic.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;

use deck_streak_curriculum::law::mastery_pillar;
use deck_streak_curriculum::progress::{BandStep, band_step, course_progress, parse_unit};
use deck_streak_ingest::reader::Card;
use deck_streak_kernel::{CourseCode, Courses, Track};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/road-to-c2.jsonl");

/// One synthetic course whose six bands each hold one unit, in the order of `CEFR_BANDS`.
const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[{"code":"ga","name":"Gamma",
"flag":"g","deck_root":"Gamma Course","alias":"g","writing":false,
"unit_bands":{"A1":[1,1],"A2":[2,2],"B1":[3,3],"B2":[4,4],"C1":[5,5],"C2":[6,6]}}]}"#;

/// The current bands the vectors name: each band, and one with no place in the order.
const CURRENT_BANDS: [&str; 7] = ["A1", "A2", "B1", "B2", "C1", "C2", "Z9"];

/// A card of course `ga` in deck `deck`: mature (a review card at 30 days, mastery 1) when
/// `achieved`, else young (a review card at 1 day, mastery 0).
fn card(deck: i64, achieved: bool) -> Card {
    Card {
        id: deck,
        note_id: deck,
        deck_id: deck,
        original_deck_id: 0,
        queue: 2,
        kind: 2,
        due: 0,
        interval: if achieved { 30 } else { 1 },
        factor: 2500,
        reps: 0,
        lapses: 0,
        track: Track::Language,
        course: CourseCode::new("ga"),
        tier: None,
        memory: None,
    }
}

/// The current band `course_progress` reaches when each band, in order, holds one card that is
/// mature exactly when its flag is set: a band with a mature card is achieved, one with a young
/// card is not.
fn current_band(courses: &Courses, achieved: &[bool]) -> &'static str {
    let (mut cards, mut deck_names) = (Vec::new(), BTreeMap::new());
    for (deck, flag) in (1_i64..).zip(achieved) {
        cards.push(card(deck, *flag));
        deck_names.insert(deck, format!("Gamma Course\u{1f}Unit {deck}"));
    }
    let progress = course_progress(&cards, &deck_names, courses, 0);
    assert_eq!(progress.len(), 1, "the one course is counted");
    progress[0].current_band
}

/// A vector field as text.
fn text<'a>(vector: &'a Value, name: &str) -> &'a str {
    vector[name]
        .as_str()
        .unwrap_or_else(|| panic!("a string {name} in {vector}"))
}

/// The step a vector records.
fn recorded_step(vector: &Value) -> BandStep {
    let band = || {
        let band = text(vector, "band");
        CURRENT_BANDS
            .into_iter()
            .find(|candidate| *candidate == band)
            .unwrap_or_else(|| panic!("a band the vectors name: {vector}"))
    };
    match text(vector, "step") {
        "first_sighting" => BandStep::FirstSighting(band()),
        "band_up" => BandStep::BandUp(band()),
        "unchanged" => BandStep::Unchanged,
        _ => panic!("a step of a known kind: {vector}"),
    }
}

#[test]
fn the_road_to_c2_rules_answer_every_lean_vector() {
    let courses = Courses::parse(COURSES).expect("the synthetic courses parse");
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "RoadToC2", "{header}");
    assert_eq!(
        header["covers"], "crates/curriculum/src/progress.rs",
        "{header}"
    );
    assert_eq!(header["anchor"], "band_step", "{header}");
    let (mut currents, mut steps, mut pillars, mut units) = (0_u64, 0_u64, 0_u64, 0_u64);
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        match vector["rule"].as_str() {
            Some("course_progress") => {
                let achieved: Vec<bool> = vector["achieved"]
                    .as_array()
                    .expect("the achieved flags")
                    .iter()
                    .map(|flag| flag.as_bool().expect("a flag"))
                    .collect();
                assert_eq!(
                    current_band(&courses, &achieved),
                    text(&vector, "current_band"),
                    "{vector}"
                );
                currents += 1;
            }
            Some("band_step") => {
                let stored = vector["stored"].as_str();
                let current = text(&vector, "current");
                let current = CURRENT_BANDS
                    .into_iter()
                    .find(|candidate| *candidate == current)
                    .unwrap_or_else(|| panic!("a current band the vectors name: {vector}"));
                assert_eq!(
                    band_step(stored, current),
                    recorded_step(&vector),
                    "{vector}"
                );
                steps += 1;
            }
            Some("mastery_pillar") => {
                let leeches = vector["law_leech_active"]
                    .as_i64()
                    .unwrap_or_else(|| panic!("a leech count in {vector}"));
                let pillar = vector["pillar"]
                    .as_i64()
                    .and_then(|pillar| i32::try_from(pillar).ok())
                    .unwrap_or_else(|| panic!("a whole pillar in {vector}"));
                assert_eq!(
                    mastery_pillar(leeches).to_bits(),
                    f64::from(pillar).to_bits(),
                    "{vector}"
                );
                pillars += 1;
            }
            Some("parse_unit") => {
                let unit = match &vector["unit"] {
                    Value::Null => None,
                    unit => Some(
                        unit.as_u64()
                            .and_then(|unit| u32::try_from(unit).ok())
                            .unwrap_or_else(|| panic!("a u32 unit in {vector}")),
                    ),
                };
                assert_eq!(parse_unit(text(&vector, "deck_name")), unit, "{vector}");
                units += 1;
            }
            _ => panic!("a vector of a known rule: {vector}"),
        }
    }
    println!(
        "examined {currents} current band, {steps} band step, {pillars} pillar and {units} unit \
         vector(s)"
    );
    assert_eq!(
        (currents, steps, pillars, units),
        (64, 56, 21, 25),
        "every input the writer prints"
    );
    assert_eq!(
        header["vectors"].as_u64(),
        Some(currents + steps + pillars + units),
        "{header}"
    );
}

#[test]
fn the_recorded_counterexamples_answer_as_proved() {
    let courses = Courses::parse(COURSES).expect("the synthetic courses parse");
    // `a_current_band_one_past_the_run_misses_it`: A1 alone achieved is a current band of A1, not
    // the A2 one past the run.
    assert_eq!(
        current_band(&courses, &[true, false, false, false, false, false]),
        "A1"
    );
    // `the_last_achieved_band_counts_after_a_gap`: A1 and B1 achieved around a gap leave A1.
    assert_eq!(
        current_band(&courses, &[true, false, true, false, false, false]),
        "A1"
    );
    // `a_band_reached_again_reads_as_a_band_up`: A2 stored and reached again is no band-up.
    assert_eq!(band_step(Some("A2"), "A2"), BandStep::Unchanged);
    // `a_pillar_with_no_penalty_cap_leaves_the_range`: 20 leeches leave a pillar of 70, not 40.
    assert_eq!(mastery_pillar(20).to_bits(), 70.0_f64.to_bits());
}
