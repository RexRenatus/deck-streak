//! The evaluator equals the engine's search golden, its rollover class passes, and the
//! engine's day equals the engine's timing goldens (SPEC-514 A3).
//!
//! The golden's driver builds each case's collection in the engine with its clock moved by whole
//! days to the host's day, and the engine reads the host's clock and zone: so each case ran at the
//! first instant at or after the case's own instant whose time of day, in UTC, is the run's, and
//! in the creation offset the case gives, with the host's zone (UTC) as the current offset. The
//! card ids are the driver's: one second apart, ending one second before the case's instant.
//! [`RUN_SECOND_OF_DAY`] is the run's one time of day; the census below prints every second of the
//! day at which the evaluator equals all the golden's cases, and holds this one among them.

#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_search::{
    Card, Collection, Deck, EngineDay, Note, Notetype, Profile, Review, eval,
};
use deck_streak_study_day::{Hour, StudyDayRule, UtcOffset};
use serde_json::Value;

/// Seconds in a day.
const DAY: i64 = 86_400;
/// The second of the UTC day the golden's run read from the host's clock.
const RUN_SECOND_OF_DAY: i64 = 36_000;

fn int(value: &Value, key: &str) -> i64 {
    value[key]
        .as_i64()
        .unwrap_or_else(|| panic!("{key} is a whole number in {value}"))
}

fn text(value: &Value) -> String {
    value
        .as_str()
        .unwrap_or_else(|| panic!("{value} is text"))
        .to_owned()
}

fn index(value: &Value) -> usize {
    usize::try_from(
        value
            .as_u64()
            .unwrap_or_else(|| panic!("{value} is an index")),
    )
    .unwrap_or_else(|_| panic!("{value} fits an index"))
}

/// The case's collection as the driver builds it: decks, note types and notes added in order,
/// so their ids rise with their index, and the cards' ids one second apart before the instant.
fn collection(source: &Value) -> Collection {
    let now = int(source, "now");
    let decks = source["decks"].as_array().expect("decks");
    let deck_id = |i: usize| i64::try_from(i).expect("a deck index") + 2;
    let notetypes = source["notetypes"]
        .as_array()
        .expect("note types")
        .iter()
        .enumerate()
        .map(|(i, nt)| Notetype {
            id: i64::try_from(i).expect("an index") + 1,
            name: text(&nt["name"]),
            fields: nt["fields"]
                .as_array()
                .expect("fields")
                .iter()
                .map(text)
                .collect(),
            templates: nt["templates"]
                .as_array()
                .expect("templates")
                .iter()
                .map(|t| text(&t["name"]))
                .collect(),
            sort_field: index(&nt["sort_field"]),
        })
        .collect();
    let notes = source["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .enumerate()
        .map(|(i, note)| Note {
            id: i64::try_from(i).expect("an index") + 1,
            notetype: index(&note["notetype"]),
            fields: note["fields"]
                .as_array()
                .expect("fields")
                .iter()
                .map(text)
                .collect(),
            tags: note["tags"]
                .as_array()
                .expect("tags")
                .iter()
                .map(text)
                .collect(),
        })
        .collect();
    let rows = source["cards"].as_array().expect("cards");
    let count = i64::try_from(rows.len()).expect("a card count");
    let cards = rows
        .iter()
        .enumerate()
        .map(|(i, card)| Card {
            id: (now - (count - i64::try_from(i).expect("an index"))) * 1000,
            note: index(&card["note"]),
            deck_id: deck_id(index(&card["deck"])),
            original_deck_id: if card["odeck"].is_null() {
                0
            } else {
                deck_id(index(&card["odeck"]))
            },
            ord: u16::try_from(int(card, "ord")).expect("an ordinal"),
            kind: int(card, "type"),
            queue: int(card, "queue"),
            due: int(card, "due"),
            original_due: int(card, "odue"),
            interval: int(card, "ivl"),
            reps: int(card, "reps"),
            lapses: int(card, "lapses"),
        })
        .collect();
    let revlog = source["revlog"]
        .as_array()
        .expect("review log")
        .iter()
        .map(|review| Review {
            id: int(review, "at_ms"),
            card: index(&review["card"]),
            ease: int(review, "ease"),
        })
        .collect();
    Collection {
        decks: decks
            .iter()
            .enumerate()
            .map(|(i, deck)| Deck {
                id: deck_id(i),
                name: text(&deck["name"]),
            })
            .collect(),
        notetypes,
        notes,
        cards,
        revlog,
    }
}

/// The engine's day for the case at `instant`: created in the case's offset, read in UTC.
fn engine_day(source: &Value, instant: i64) -> EngineDay {
    let offset = i16::try_from(int(source, "utc_offset_minutes")).expect("an offset");
    let creation = UtcOffset::from_minutes(offset).expect("an offset in bounds");
    let hour = Hour::new(u8::try_from(int(source, "rollover_hour")).expect("an hour"))
        .expect("an hour in bounds");
    EngineDay::at(
        int(source, "crt"),
        creation,
        instant,
        StudyDayRule::new(hour, UtcOffset::UTC),
    )
}

/// The instant the run's engine read for a case whose own instant is `now`.
fn run_instant(now: i64, second_of_day: i64) -> i64 {
    now + (second_of_day - now).rem_euclid(DAY)
}

/// The cards the evaluator selects for one golden case at the run's second of the day.
fn answer(input: &Value, second_of_day: i64) -> Value {
    let source = &input["collection"];
    let search = input["search"].as_str().expect("a search");
    let query = Profile::OPERATOR
        .parse(search)
        .unwrap_or_else(|refusal| panic!("{search:?} parses: {refusal}"));
    let day = engine_day(source, run_instant(int(source, "now"), second_of_day));
    let cards = eval(&query, &collection(source), day);
    serde_json::json!({ "cards": cards })
}

#[test]
fn eval_equals_the_engines_search_golden() {
    let examined = golden::assert_parity(&golden::committed("anki.search.operator"), |input| {
        answer(input, RUN_SECOND_OF_DAY)
    });
    assert_eq!(examined.count, 88, "the golden's cases");
}

#[test]
fn the_rollover_class_passes() {
    let mut rollover = 0;
    golden::each_case("anki.search.operator", |case| {
        if case.class.as_deref() == Some("rollover") {
            rollover += 1;
            assert_eq!(
                answer(&case.input, RUN_SECOND_OF_DAY),
                case.output,
                "the rollover case {}",
                case.input["search"]
            );
        }
    });
    println!("examined {rollover} rollover case(s)");
    assert_eq!(rollover, 8, "the golden's rollover class");
}

#[test]
fn the_engine_day_matches_the_engines_timing_golden() {
    for name in ["anki.sched.timing_today", "anki.timing.day"] {
        golden::each_case(name, |case| {
            let input = &case.input;
            let offset = i16::try_from(int(input, "utc_offset_minutes")).expect("an offset");
            let zone = UtcOffset::from_minutes(offset).expect("an offset in bounds");
            let hour = Hour::new(u8::try_from(int(input, "rollover_hour")).expect("an hour"))
                .expect("an hour in bounds");
            let day = EngineDay::at(
                int(input, "crt"),
                zone,
                int(input, "now"),
                StudyDayRule::new(hour, zone),
            );
            assert_eq!(
                serde_json::json!({"day": day.days_elapsed, "next_rollover": day.next_day_at}),
                case.output,
                "{name}: {input}"
            );
        });
    }
}

#[test]
fn the_run_second_is_one_of_the_seconds_the_golden_admits() {
    let golden = golden::read(&golden::committed("anki.search.operator")).expect("the golden");
    let admitted: Vec<i64> = (0..DAY)
        .step_by(60)
        .filter(|second| {
            golden
                .cases
                .iter()
                .all(|case| answer(&case.input, *second) == case.output)
        })
        .collect();
    let first = admitted.first().copied().unwrap_or(-1);
    let last = admitted.last().copied().unwrap_or(-1);
    println!(
        "examined {} minute(s) of the day: {} admit every case, from second {first} to {last}",
        DAY / 60,
        admitted.len()
    );
    assert!(
        admitted.contains(&RUN_SECOND_OF_DAY),
        "the run's second is not one the golden admits"
    );
    assert_eq!(
        i64::try_from(admitted.len()).expect("a count"),
        (last - first) / 60 + 1,
        "the admitted seconds are not one band"
    );
    assert!(
        admitted.len() < usize::try_from(DAY / 60).expect("a count"),
        "every minute admits every case, so the golden pins no clock"
    );
}
