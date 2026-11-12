//! The historical landmarks (SPEC-102 A1 to A4): the landmarks, the days due, the texts and the
//! constants equal the goldens of the predecessor's `landmarks.py`.

#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_kernel::StudyDay;
use deck_streak_notifications::landmarks::{
    ANNIVERSARY_EVENT_TYPE, ANNIVERSARY_GAP_TEMPLATE, ANNIVERSARY_TEMPLATE, LANDMARK_DAY_STEP,
    LANDMARK_HIGH_WATER_KEY, Landmark, STUDY_DAY_EVENT_TYPE, STUDY_DAY_TEMPLATE, compute_landmarks,
    due_today, high_water_mark, render_landmark,
};
use serde_json::Value;

fn integer(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

fn day(value: &Value) -> StudyDay {
    StudyDay::from_epoch_day(integer(value))
}

fn days(value: &Value) -> Vec<StudyDay> {
    value
        .as_array()
        .expect("a list of days")
        .iter()
        .map(day)
        .collect()
}

/// A golden text with each `{day:N}` token written as the ISO date of epoch day `N`.
fn expand(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("{day:") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "{day:".len()..];
        let end = after.find('}').expect("a closed token");
        let number: i64 = after[..end].parse().expect("a day number");
        out.push_str(&StudyDay::from_epoch_day(number).to_string());
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// The landmarks of a golden case's output, as the port's own type.
fn expected_landmarks(output: &Value) -> Vec<(String, String, i64, StudyDay)> {
    output["landmarks"]
        .as_array()
        .expect("the landmarks")
        .iter()
        .map(|item| {
            (
                item["key"].as_str().expect("a key").to_owned(),
                item["event"].as_str().expect("an event").to_owned(),
                integer(&item["ordinal"]),
                day(&item["day"]),
            )
        })
        .collect()
}

fn seen(found: &[Landmark]) -> Vec<(String, String, i64, StudyDay)> {
    found
        .iter()
        .map(|item| {
            (
                item.key.clone(),
                item.event.to_owned(),
                i64::from(item.ordinal),
                item.day,
            )
        })
        .collect()
}

#[test]
fn the_landmarks_match_the_parity_golden() {
    let examined = golden::each_case("landmarks", |case| {
        let found = compute_landmarks(&days(&case.input["study_days"]), day(&case.input["today"]));
        assert_eq!(
            seen(&found),
            expected_landmarks(&case.output),
            "{}",
            case.input
        );
    });
    println!("{examined}");
}

#[test]
fn only_the_landmarks_dated_the_day_are_due() {
    let examined = golden::each_case("landmarks", |case| {
        let today = day(&case.input["today"]);
        let found = compute_landmarks(&days(&case.input["study_days"]), today);
        let due: Vec<&str> = due_today(&found, today)
            .iter()
            .map(|item| item.key.as_str())
            .collect();
        let expected: Vec<&str> = case.output["due"]
            .as_array()
            .expect("the due keys")
            .iter()
            .map(|key| key.as_str().expect("a key"))
            .collect();
        assert_eq!(due, expected, "{}", case.input);
        assert!(
            found.iter().filter(|item| item.day == today).count() == due.len(),
            "{}",
            case.input
        );
    });
    println!("{examined}");
}

#[test]
fn the_landmark_text_matches_the_parity_golden() {
    let examined = golden::each_case("landmark_text", |case| {
        let ordinal = u32::try_from(integer(&case.input["ordinal"])).expect("an ordinal");
        let event = case.input["event"].as_str().expect("an event");
        let event: &'static str = if event == "landmark_anniversary" {
            ANNIVERSARY_EVENT_TYPE
        } else {
            STUDY_DAY_EVENT_TYPE
        };
        let item = Landmark {
            key: String::new(),
            event,
            ordinal,
            day: day(&case.input["day"]),
        };
        assert_eq!(
            render_landmark(&item, false),
            expand(case.output["plain"].as_str().expect("the plain text")),
            "{}",
            case.input
        );
        assert_eq!(
            render_landmark(&item, true),
            expand(case.output["gap_honest"].as_str().expect("the gap text")),
            "{}",
            case.input
        );
    });
    println!("{examined}");
}

#[test]
fn the_landmark_constants_equal_the_predecessors() {
    let examined = golden::each_case("landmarks.constants", |case| {
        let name = case.input["name"].as_str().expect("a name");
        match name {
            "landmarks.LANDMARK_DAY_STEP" => {
                assert_eq!(
                    i64::try_from(LANDMARK_DAY_STEP).unwrap(),
                    integer(&case.output)
                );
            }
            "landmarks.ANNIVERSARY_EVENT_TYPE" => {
                assert_eq!(ANNIVERSARY_EVENT_TYPE, case.output.as_str().unwrap());
            }
            "landmarks.STUDY_DAY_EVENT_TYPE" => {
                assert_eq!(STUDY_DAY_EVENT_TYPE, case.output.as_str().unwrap());
            }
            "constants.__LANDMARK_ANNIVERSARY_TEMPLATE" => {
                assert_eq!(ANNIVERSARY_TEMPLATE, case.output.as_str().unwrap());
            }
            "constants.__LANDMARK_ANNIVERSARY_GAP_TEMPLATE" => {
                assert_eq!(ANNIVERSARY_GAP_TEMPLATE, case.output.as_str().unwrap());
            }
            "constants.__LANDMARK_STUDY_DAY_TEMPLATE" => {
                assert_eq!(STUDY_DAY_TEMPLATE, case.output.as_str().unwrap());
            }
            "landmarks.LANDMARK_HIGH_WATER_KEY" => {
                assert_eq!(LANDMARK_HIGH_WATER_KEY, case.output.as_str().unwrap());
            }
            other => panic!("a constant this test does not know: {other}"),
        }
    });
    println!("{examined}");
}

/// The mark the first run stores is the predecessor's bytes (SPEC-102 A38): every golden run that
/// wrote a mark wrote the highest anniversary and study-day ordinals of the landmarks up to today.
#[test]
fn the_mark_is_the_predecessors_json_for_every_golden_run() {
    let cases = golden::read(&golden::committed("landmarks_run")).expect("the run golden");
    let mut examined = 0;
    for case in &cases.cases {
        let landmarks =
            compute_landmarks(&days(&case.input["study_days"]), day(&case.input["today"]));
        match case.output["mark_written"].as_str() {
            Some(written) => assert_eq!(
                high_water_mark(&landmarks),
                written,
                "the mark of {}",
                case.input
            ),
            // A run that found the mark stored writes none, so it has no bytes to compare.
            None => assert!(
                case.input["mark"].is_string(),
                "a run writes no mark only when one is stored: {}",
                case.input
            ),
        }
        examined += 1;
    }
    assert_eq!(examined, 10, "every landmarks_run case is examined");
    println!("examined {examined} landmarks_run marks");
}

#[test]
fn the_anniversary_walk_stops_at_its_cap() {
    let today: StudyDay = "+11975-01-01".parse().expect("a far study day");
    let found = compute_landmarks(&[StudyDay::from_epoch_day(0)], today);
    let anniversaries: Vec<&Landmark> = found
        .iter()
        .filter(|item| item.event == ANNIVERSARY_EVENT_TYPE)
        .collect();
    assert_eq!(anniversaries.len(), 10_000);
    let last = anniversaries.last().expect("an anniversary");
    assert_eq!(last.key, "landmark:anniv:10000");
    assert_eq!(
        last.day,
        "+11970-01-01".parse::<StudyDay>().expect("the cap's day")
    );
}
