//! SPEC-413 A5 (R9): the engine core's FSRS-7 schedule gives every vector the Lean port writes.
//! For each line of `formal/vectors/fsrs7-schedule.jsonl`, written by
//! `formal/lean/Formal/Fsrs7ScheduleVectors.lean` from the port in
//! `formal/lean/Formal/Fsrs7Schedule.lean`, the replay's rounding step gives the vector's whole
//! days, its day count gives the vector's engine day of the last review, and its schedule step gives
//! the vector's schedule; for each write line, `CardReplay::stock_fields` leaves the vector's card
//! after one write and after two. The private steps are reached through the module's hidden pure
//! seams (ADR-427). An `f32` is written `[0]` NaN, `[1]` and `[2]` the infinities, `[3,n,d]` the
//! value `n / d` with a power-of-two `d`, so each is exact in an `f32`.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "an integration test panics on a malformed vector, and prints its examined count on \
              purpose"
)]

use anki_proto::cards::{Card, FsrsMemoryState};
use deck_streak_engine_core::late::EngineDay;
use deck_streak_engine_core::replay::{
    CardReplay, Schedule, engine_day_of, schedule_of, whole_days,
};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/fsrs7-schedule.jsonl");

/// An integer the vector holds.
fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer, not {value}"))
}

/// An unsigned integer the vector holds, as a `u32`.
fn unsigned(value: &Value) -> u32 {
    u32::try_from(int(value)).unwrap_or_else(|_| panic!("a u32, not {value}"))
}

/// An `f32` as the writer tags it.
fn float(value: &Value) -> f32 {
    let tagged = value.as_array().expect("an f32 is a tagged array");
    match (tagged.len(), tagged.first().map(int)) {
        (1, Some(0)) => f32::NAN,
        (1, Some(1)) => f32::NEG_INFINITY,
        (1, Some(2)) => f32::INFINITY,
        (3, Some(3)) => {
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                reason = "the writer's numerators and power-of-two denominators are exact in an f64, \
                          and their quotient is exact in an f32"
            )]
            let quotient = (int(&tagged[1]) as f64 / int(&tagged[2]) as f64) as f32;
            quotient
        }
        _ => panic!("an unknown f32 tag: {value}"),
    }
}

/// Two `f32` values the vectors call equal: the same bits, or both NaN.
fn same_float(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

/// A schedule, or `None` for the writer's `[]`.
fn schedule(value: &Value) -> Option<Schedule> {
    let pair = value.as_array().expect("a schedule is an array");
    match pair.as_slice() {
        [] => None,
        [ivl, due] => Some(Schedule {
            ivl: unsigned(ivl),
            due: i32::try_from(int(due)).expect("a due fits an i32"),
        }),
        _ => panic!("a schedule has 0 or 2 fields: {value}"),
    }
}

/// The engine day a vector names: today's count and the next rollover.
fn engine_day(value: &Value) -> EngineDay {
    EngineDay {
        days_elapsed: unsigned(&value[0]),
        next_day_at: int(&value[1]),
    }
}

/// A card as the writer writes it: its memory state, interval and due; every other field is the
/// default.
fn card(value: &Value) -> Card {
    let memory = value[0].as_array().expect("a memory state is an array");
    Card {
        memory_state: match memory.as_slice() {
            [] => None,
            [stability, difficulty] => Some(FsrsMemoryState {
                stability: float(stability),
                difficulty: float(difficulty),
            }),
            _ => panic!("a memory state has 0 or 2 fields: {value}"),
        },
        interval: unsigned(&value[1]),
        due: i32::try_from(int(&value[2])).expect("a due fits an i32"),
        ..Card::default()
    }
}

/// Whether two cards hold the same memory state, interval and due.
fn same_card(a: &Card, b: &Card) -> bool {
    let memory = match (a.memory_state, b.memory_state) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            same_float(x.stability, y.stability) && same_float(x.difficulty, y.difficulty)
        }
        _ => false,
    };
    memory && a.interval == b.interval && a.due == b.due
}

/// The replay a write line names.
fn replay(value: &Value) -> CardReplay {
    CardReplay {
        card: 1,
        last_review: 0,
        stability: float(&value[0]),
        difficulty: float(&value[1]),
        schedule: schedule(&value[2]),
    }
}

/// One schedule vector: `None` when the engine core gives every recorded answer, else the first
/// difference.
fn schedule_differs(vector: &Value) -> Option<String> {
    let day = engine_day(&vector["day"]);
    let last = int(&vector["last"]);
    let interval = float(&vector["interval"]);
    let max_ivl = unsigned(&vector["max"]);
    let whole = whole_days(interval);
    if whole != unsigned(&vector["whole"]) {
        return Some(format!("the whole days are {whole}"));
    }
    let last_day = engine_day_of(day, last);
    if last_day != int(&vector["last_day"]) {
        return Some(format!("the last review's engine day is {last_day}"));
    }
    let given = schedule_of(whole, last, day, max_ivl);
    (given != schedule(&vector["schedule"])).then(|| format!("the schedule is {given:?}"))
}

/// One write vector: `None` when one write and two leave the recorded cards, else the first
/// difference.
fn write_differs(vector: &Value) -> Option<String> {
    let replayed = replay(&vector["replay"]);
    let mut written = card(&vector["card"]);
    replayed.stock_fields(&mut written);
    if !same_card(&written, &card(&vector["written"])) {
        return Some(format!("one write leaves {written:?}"));
    }
    replayed.stock_fields(&mut written);
    (!same_card(&written, &card(&vector["twice"]))).then(|| format!("two writes leave {written:?}"))
}

#[test]
fn the_engine_cores_schedule_gives_every_lean_vectors_output() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("the header parses");
    assert_eq!(header["schema"], "phx.formal.vectors.v1");
    assert_eq!(header["entry"], "Fsrs7Schedule");
    assert_eq!(
        header["covers"], "crates/engine-core/src/replay.rs",
        "the vectors were written from the entry that covers the replay's schedule"
    );
    assert_eq!(header["anchor"], "scheduled");
    let declared = header["vectors"].as_u64().expect("a declared count");

    let mut examined = 0_u64;
    let mut schedules = 0_u64;
    let mut writes = 0_u64;
    let mut differing = Vec::new();
    for (at, line) in lines.enumerate() {
        let vector: Value = serde_json::from_str(line).expect("a vector parses");
        let differs = if vector.get("schedule").is_some() {
            schedules += 1;
            schedule_differs(&vector)
        } else {
            writes += 1;
            write_differs(&vector)
        };
        if let Some(why) = differs {
            differing.push(format!("line {}: {line}: {why}", at + 2));
        }
        examined += 1;
    }
    println!(
        "examined {examined} vector(s): {schedules} schedule(s), {writes} write(s), {} differing",
        differing.len()
    );
    assert!(examined > 0, "examined 0 vectors: nothing was judged");
    assert!(
        schedules > 0 && writes > 0,
        "both halves are judged: {schedules} schedule(s), {writes} write(s)"
    );
    assert_eq!(examined, declared, "the header declares {declared} vectors");
    assert!(
        differing.is_empty(),
        "{} of {examined} vectors differ; the first: {}",
        differing.len(),
        differing.first().map_or("", String::as_str)
    );
}

/// The recorded counterexamples, as literal cases: a first Again's interval at 0.9 rounds to 0
/// whole days and is scheduled one day out; a review one second before today began counts in
/// yesterday; a replay written twice leaves the due it wrote once.
#[test]
fn the_recorded_counterexamples_hold_as_literal_cases() {
    assert_eq!(whole_days(3.827_562_6e-5), 0);
    let today = EngineDay {
        days_elapsed: 10,
        next_day_at: 172_800,
    };
    assert_eq!(engine_day_of(today, 86_399_000), 9);
    assert_eq!(
        schedule_of(0, 86_399_000, today, 36_500),
        Some(Schedule { ivl: 1, due: 10 })
    );
    let replayed = CardReplay {
        card: 1,
        last_review: 0,
        stability: 4.0,
        difficulty: 5.0,
        schedule: Some(Schedule { ivl: 5, due: 5 }),
    };
    let mut written = Card::default();
    replayed.stock_fields(&mut written);
    replayed.stock_fields(&mut written);
    assert_eq!((written.interval, written.due), (5, 5));
}
