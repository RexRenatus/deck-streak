//! A study day's metrics, its card snapshot and its per-course rows equal the predecessor's
//! (SPEC-071 A1 to A4; R6, R8, R11): every case of `goldens/daily_metrics.json`,
//! `goldens/card_snapshot.json` and `goldens/language_daily_metrics.json`, and no per-course row
//! for a review whose card is in no course. Every review, card, deck and course is synthetic.

// An integration test is test code: its helpers panic on a malformed golden, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_analytics::metrics::{LanguageDay, daily_metrics, language_metrics};
use deck_streak_analytics::score::ScoreState;
use deck_streak_analytics::settings::{AnalyticsSettings, LEECH_THRESHOLD, LeechThreshold};
use deck_streak_analytics::snapshot::{CardSnapshot, CardState, card_snapshot};
use deck_streak_ingest::reader::{Card, Review, course_of};
use deck_streak_ingest::settings::DECK_SEPARATOR;
use deck_streak_kernel::{
    CourseCode, Courses, Environment, Hour, StudyDay, StudyDayRule, Track, UtcOffset,
};
use serde_json::{Value, json};

fn integer(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

fn number(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("a number: {value}"))
}

fn rule(input: &Value) -> StudyDayRule {
    let hour = u8::try_from(integer(&input["rollover_hour"]))
        .ok()
        .and_then(Hour::new)
        .expect("an hour");
    let offset = i16::try_from(integer(&input["utc_offset_minutes"]))
        .ok()
        .and_then(UtcOffset::from_minutes)
        .expect("an offset");
    StudyDayRule::new(hour, offset)
}

/// A review from a golden's row: id, card, ease, interval, last interval, factor, time and type.
fn review(row: &Value) -> Review {
    let field = |index: usize| integer(&row[index]);
    Review {
        id: field(0),
        card_id: field(1),
        ease: field(2),
        interval: field(3),
        last_interval: field(4),
        factor: field(5),
        taken_ms: field(6),
        kind: field(7),
    }
}

fn reviews(input: &Value) -> Vec<Review> {
    input["reviews"]
        .as_array()
        .expect("the case's reviews")
        .iter()
        .map(review)
        .collect()
}

fn pairs(value: &Value) -> Vec<(i64, Value)> {
    value
        .as_array()
        .expect("a list of pairs")
        .iter()
        .map(|pair| (integer(&pair[0]), pair[1].clone()))
        .collect()
}

#[test]
fn the_daily_metrics_match_the_predecessors_golden() {
    let mut classes = BTreeSet::new();
    golden::each_case("daily_metrics", |case| {
        let input = &case.input;
        let card_decks: BTreeMap<i64, i64> = pairs(&input["card_decks"])
            .into_iter()
            .map(|(card, deck)| (card, integer(&deck)))
            .collect();
        let day = StudyDay::from_epoch_day(integer(&input["day"]));
        let metrics = daily_metrics(&reviews(input), rule(input), day, &card_decks);
        let port = json!({
            "day": metrics.day.epoch_day(),
            "reviews": metrics.reviews,
            "learn_count": metrics.learn_count,
            "review_count": metrics.review_count,
            "relearn_count": metrics.relearn_count,
            "filtered_count": metrics.filtered_count,
            "answered": metrics.answered,
            "passed": metrics.passed,
            "graduations": metrics.graduations,
            "decks_studied": metrics.decks_studied,
            "young_answered": metrics.young_answered,
            "young_passed": metrics.young_passed,
            "mature_answered": metrics.mature_answered,
            "mature_passed": metrics.mature_passed,
        });
        for (field, value) in port.as_object().expect("an object") {
            assert_eq!(&case.output[field], value, "{field} of {input}");
        }
        // Floats are compared bit for bit: the port sums in the predecessor's order.
        for (field, value) in [
            ("seconds", metrics.seconds),
            ("true_retention", metrics.true_retention),
            ("avg_answer_seconds", metrics.avg_answer_seconds),
        ] {
            assert_eq!(
                number(&case.output[field]).to_bits(),
                value.to_bits(),
                "{field} of {input}: the predecessor's {} against {value}",
                case.output[field]
            );
        }
        if let Some(class) = &case.class {
            classes.insert(class.clone());
        }
    });
    for class in ["empty", "tie", "mature-boundary", "rollover", "many"] {
        assert!(
            classes.contains(class),
            "the golden carries its {class} class"
        );
    }
}

/// A card from a golden's row: id, queue, type, due, interval and lapses.
fn card(row: &Value) -> Card {
    let field = |index: usize| integer(&row[index]);
    Card {
        id: field(0),
        note_id: field(0),
        deck_id: 1,
        original_deck_id: 0,
        queue: field(1),
        kind: field(2),
        due: field(3),
        interval: field(4),
        factor: 2500,
        reps: 0,
        lapses: field(5),
        track: Track::Language,
        course: None,
        tier: None,
    }
}

#[test]
fn the_card_snapshot_matches_the_predecessors_golden() {
    let mut defaults = 0;
    golden::each_case("card_snapshot", |case| {
        let input = &case.input;
        let cards: Vec<Card> = input["cards"]
            .as_array()
            .expect("the case's cards")
            .iter()
            .map(card)
            .collect();
        // A case with no threshold takes the predecessor's default, which must be the port's.
        let threshold = if input["leech_threshold"].is_null() {
            defaults += 1;
            LeechThreshold::default().get()
        } else {
            integer(&input["leech_threshold"])
        };
        let snapshot = card_snapshot(&cards, threshold, integer(&input["day_number"]));
        let port = json!({
            "total_cards": snapshot.total_cards,
            "mature_count": snapshot.mature_count,
            "young_count": snapshot.young_count,
            "learning_count": snapshot.learning_count,
            "suspended_count": snapshot.suspended_count,
            "leech_active": snapshot.leech_active,
            "backlog": snapshot.backlog,
            "due_today": snapshot.due_today,
        });
        assert_eq!(port, case.output, "the snapshot of {input}");
    });
    assert!(
        defaults > 0,
        "the golden carries cases at the default threshold"
    );
}

/// The case's synthetic courses, as a courses file holds them.
fn courses_of(input: &Value) -> Courses {
    let courses: Vec<Value> = input["courses"]
        .as_array()
        .expect("the case's courses")
        .iter()
        .enumerate()
        .map(|(index, course)| {
            let alias = char::from(b'a' + u8::try_from(index).expect("few courses"));
            json!({
                "code": course[1], "name": course[2], "flag": course[3], "deck_root": course[0],
                "alias": alias.to_string(), "writing": false, "unit_bands": {},
            })
        })
        .collect();
    let file = json!({
        "schema": "deckstreak.courses.v1",
        "courses": courses,
        "focus_subjects": [],
    });
    Courses::parse(&file.to_string()).expect("the case's courses parse")
}

/// Each card's course: its deck's name, through ingest's course rule.
fn card_courses(input: &Value, courses: &Courses) -> BTreeMap<i64, CourseCode> {
    let names: BTreeMap<i64, String> = pairs(&input["deck_names"])
        .into_iter()
        .map(|(deck, name)| (deck, name.as_str().expect("a name").to_owned()))
        .collect();
    pairs(&input["card_decks"])
        .into_iter()
        .filter_map(|(card, deck)| {
            let name = names.get(&integer(&deck))?;
            course_of(courses, name).map(|course| (card, course))
        })
        .collect()
}

fn rows(languages: &[LanguageDay]) -> Vec<Value> {
    languages
        .iter()
        .map(|row| {
            json!([
                row.day.epoch_day(),
                row.course.as_str(),
                row.reviews,
                row.seconds,
                row.answered,
                row.passed
            ])
        })
        .collect()
}

#[test]
fn the_per_language_metrics_match_the_predecessors_golden() {
    let mut rows_proved = 0;
    golden::each_case("language_daily_metrics", |case| {
        let input = &case.input;
        let courses = courses_of(input);
        let days: BTreeSet<StudyDay> = input["days"]
            .as_array()
            .expect("the case's days")
            .iter()
            .map(|day| StudyDay::from_epoch_day(integer(day)))
            .collect();
        let port = language_metrics(
            &reviews(input),
            rule(input),
            &card_courses(input, &courses),
            &days,
        );
        let expected = case.output.as_array().expect("the golden's rows");
        assert_eq!(rows(&port).len(), expected.len(), "the rows of {input}");
        for (port_row, golden_row) in rows(&port).iter().zip(expected) {
            // The seconds are compared bit for bit, the rest exactly.
            assert_eq!(port_row[0], golden_row[0]);
            assert_eq!(port_row[1], golden_row[1]);
            assert_eq!(port_row[2], golden_row[2]);
            assert_eq!(
                number(&port_row[3]).to_bits(),
                number(&golden_row[3]).to_bits(),
                "the seconds of {golden_row}"
            );
            assert_eq!(port_row[4], golden_row[4]);
            assert_eq!(port_row[5], golden_row[5]);
            rows_proved += 1;
        }
    });
    assert!(rows_proved > 0, "the golden proved at least one row");
}

#[test]
fn a_review_with_no_course_adds_no_language_row() {
    let courses = Courses::parse(
        &json!({
            "schema": "deckstreak.courses.v1",
            "courses": [{"code": "qaa", "name": "Course Qaa", "flag": "F", "deck_root": "Qaa",
                         "alias": "a", "writing": false, "unit_bands": {}}],
            "focus_subjects": [],
        })
        .to_string(),
    )
    .expect("the courses parse");
    let rule = StudyDayRule::default();
    let day = StudyDay::from_epoch_day(20_000);
    // 20000 days after the epoch, at 05:00 UTC: inside the study day under the default rule.
    let at = 20_000 * 86_400_000 + 5 * 3_600_000;
    let unit = format!("Qaa{DECK_SEPARATOR}Unit 01");
    let decks = [
        ("Qaa", Some("qaa")),
        ("Qaa Arts", None),
        (unit.as_str(), Some("qaa")),
        ("Qa", None),
        ("Other", None),
    ];
    let mut card_courses = BTreeMap::new();
    let mut reviews = Vec::new();
    for (index, (deck, expected)) in (1_i64..).zip(decks) {
        let course = course_of(&courses, deck);
        assert_eq!(
            course.map(|code| code.as_str().to_owned()).as_deref(),
            expected,
            "{deck:?}"
        );
        if let Some(course) = course {
            card_courses.insert(index, course);
        }
        reviews.push(Review {
            id: at + index,
            card_id: index,
            ease: 3,
            interval: 30,
            last_interval: 25,
            factor: 2500,
            taken_ms: 10_000,
            kind: 1,
        });
    }
    // A card the courses never heard of.
    reviews.push(Review {
        id: at + 100,
        card_id: 99,
        ease: 1,
        interval: 1,
        last_interval: 25,
        factor: 2500,
        taken_ms: 10_000,
        kind: 1,
    });
    let languages = language_metrics(&reviews, rule, &card_courses, &BTreeSet::from([day]));
    assert_eq!(
        rows(&languages),
        [json!([20_000, "qaa", 2, 20.0, 2, 2])],
        "only the course's two reviews make a row"
    );
}

/// SPEC-071 §10: a snapshot's counts reach the stored card state and the score's inputs unchanged.
#[test]
fn a_snapshots_counts_reach_the_card_state_and_the_scores_inputs() {
    let snapshot = CardSnapshot {
        total_cards: 90,
        mature_count: 40,
        young_count: 30,
        learning_count: 11,
        suspended_count: 9,
        leech_active: 3,
        backlog: 7,
        due_today: 12,
    };
    let state = CardState::from(&snapshot);
    assert_eq!(
        state,
        CardState {
            mature_count: 40,
            young_count: 30,
            leech_active: 3,
            backlog: 7,
            due_today: 12,
        }
    );
    let inputs = ScoreState {
        due_today: 12,
        backlog: 7,
        leech_active: 3,
    };
    assert_eq!(ScoreState::from(&snapshot), inputs);
    assert_eq!(ScoreState::from(&state), inputs);
}

/// SPEC-071 R8: the leech threshold is read from its setting, and defaults when it is unset.
#[test]
fn the_leech_threshold_is_read_from_its_setting() {
    let set = AnalyticsSettings::from_env(&Environment::from_vars([(LEECH_THRESHOLD, "12")]))
        .expect("a threshold of 12 lapses");
    assert_eq!(set.leech_threshold.get(), 12);
    let unset = AnalyticsSettings::from_env(&Environment::default()).expect("no setting");
    assert_eq!(unset.leech_threshold, LeechThreshold::default());
    assert_ne!(
        LeechThreshold::default().get(),
        12,
        "the set value differs from the default"
    );
}
