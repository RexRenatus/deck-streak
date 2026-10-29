//! The web engine decides over the companion's frames as the native build does (SPEC-511 C16, R10,
//! R7): for every decision vector of `formal/vectors/companion-decide.jsonl`, the web engine's
//! `decide`, given the hello, the `Preview` request and the reply as frames, answers the bytes the
//! interface crate's own `decide` gives, laid out here by hand, and that answer agrees with the
//! vector's outcome. The `Absent` frames, the committed `reply-refused` among them, are refused.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_companion::codec::{encode_reply, encode_request};
use deck_streak_companion::decide::{decide as native_decide, NewDue, Refusal, Session};
use deck_streak_companion::wire::{
    Adapter, HelloReply, Model, Preview, Reply, Request, ReviewEvent, Status,
};
use deck_streak_web_engine::companion::decide;
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/companion-decide.jsonl");
/// The committed frames (SPEC-501's codec vectors).
const FRAMES: &str = include_str!("../../../formal/vectors/companion-frames.jsonl");

fn number(vector: &Value, name: &str) -> i64 {
    vector[name]
        .as_i64()
        .unwrap_or_else(|| panic!("a number {name} in {vector}"))
}

fn small<T: TryFrom<i64>>(vector: &Value, name: &str) -> T {
    T::try_from(number(vector, name)).unwrap_or_else(|_| panic!("{name} in range in {vector}"))
}

fn text<'a>(vector: &'a Value, name: &str) -> &'a str {
    vector[name]
        .as_str()
        .unwrap_or_else(|| panic!("a string {name} in {vector}"))
}

fn status(name: &str) -> Status {
    match name {
        "ok" => Status::Ok,
        "absent" => Status::Absent,
        "fixture_refuses" => Status::FixtureRefuses,
        "busy" => Status::Busy,
        "error" => Status::Error,
        "conflict" => Status::Conflict,
        "unsupported" => Status::Unsupported,
        _ => panic!("a status name: {name}"),
    }
}

/// The status byte, written out here apart from the crate: the wire's declared values.
fn status_byte(status: Status) -> u8 {
    match status {
        Status::Ok => 0,
        Status::Absent => 1,
        Status::FixtureRefuses => 2,
        Status::Busy => 3,
        Status::Error => 4,
        Status::Conflict => 5,
        Status::Unsupported => 6,
    }
}

/// A refusal's code: its place in `Refusal`'s declared order, from 1, written out here.
fn refusal_code(refusal: Refusal) -> u8 {
    match refusal {
        Refusal::Malformed => 1,
        Refusal::NotNegotiated => 2,
        Refusal::Absent => 3,
        Refusal::Mismatch => 4,
        Refusal::Fixture => 5,
        Refusal::Status(_) => 6,
        Refusal::StaleEpoch => 7,
        Refusal::WrongCard => 8,
        Refusal::WrongStage => 9,
        Refusal::ModelChanged => 10,
        Refusal::IntervalOutOfRange => 11,
        Refusal::DueDayInconsistent => 12,
    }
}

/// A refusal's name, as the Lean writer spells it.
fn refusal_name(refusal: Refusal) -> String {
    let name = match refusal {
        Refusal::Malformed => "malformed",
        Refusal::NotNegotiated => "not_negotiated",
        Refusal::Absent => "absent",
        Refusal::Mismatch => "mismatch",
        Refusal::Fixture => "fixture",
        Refusal::Status(status) => {
            let word = match status {
                Status::Ok => "ok",
                Status::Absent => "absent",
                Status::FixtureRefuses => "fixture_refuses",
                Status::Busy => "busy",
                Status::Error => "error",
                Status::Conflict => "conflict",
                Status::Unsupported => "unsupported",
            };
            return format!("status:{word}");
        }
        Refusal::StaleEpoch => "stale_epoch",
        Refusal::WrongCard => "wrong_card",
        Refusal::WrongStage => "wrong_stage",
        Refusal::ModelChanged => "model_changed",
        Refusal::IntervalOutOfRange => "interval_out_of_range",
        Refusal::DueDayInconsistent => "due_day_inconsistent",
    };
    name.to_owned()
}

/// The answer's bytes, laid out here by hand (SPEC-511 R10): 17 for an admitted due date, `0`
/// then `card` i64, `interval_days` u32 and `due_day` i32, little-endian; 3 for a refusal, `1`,
/// the refusal's code and the status for `Status`, else `0xff`.
fn layout(outcome: Result<NewDue, Refusal>) -> Vec<u8> {
    match outcome {
        Ok(due) => {
            let mut bytes = vec![0];
            bytes.extend_from_slice(&due.card.to_le_bytes());
            bytes.extend_from_slice(&due.interval_days.to_le_bytes());
            bytes.extend_from_slice(&due.due_day.to_le_bytes());
            bytes
        }
        Err(refusal) => {
            let status = match refusal {
                Refusal::Status(status) => status_byte(status),
                _ => 0xff,
            };
            vec![1, refusal_code(refusal), status]
        }
    }
}

fn model_id(first: u64) -> [u64; 4] {
    [first, 0, 0, 0]
}

/// The outcome the Lean port printed for a decision.
fn lean_decision(vector: &Value) -> Result<NewDue, String> {
    match text(vector, "outcome") {
        "ok" => {
            let due = vector["due"].as_array().expect("an admitted due date");
            let part = |index: usize| due[index].as_i64().expect("a number");
            Ok(NewDue {
                card: part(0),
                interval_days: u32::try_from(part(1)).expect("an interval"),
                due_day: i32::try_from(part(2)).expect("a due day"),
            })
        }
        refusal => Err(refusal.to_owned()),
    }
}

/// One decision vector's inputs: the hello, the session's epoch, the press, the reply's epoch and
/// each reply it is read as (a non-due reply once as an `Ack` and once as a `Refused`).
struct Inputs {
    hello: HelloReply,
    epoch: u32,
    asked: Preview,
    replied: u32,
    replies: Vec<Reply>,
}

fn inputs(vector: &Value) -> Inputs {
    let model = if vector["session_model"].is_null() {
        Model::Fixture
    } else {
        Model::Trained
    };
    let hello = HelloReply {
        status: Status::Ok,
        major: 1,
        minor: 0,
        build_id: [0; 4],
        model_id: model_id(vector["session_model"].as_u64().unwrap_or(0)),
        model,
        adapter: Adapter::Off,
    };
    let asked = Preview {
        event: ReviewEvent {
            t_ms: number(vector, "t_ms"),
            origin: [1, 2],
            seq: 3,
            card: number(vector, "card"),
            note: 4,
            home_deck: 5,
            preset: 6,
            grade: 3,
            kind: 1,
            duration_ms: 1_000,
            study_day: small(vector, "study_day"),
        },
        target_ppm: 900_000,
    };
    let status = status(text(vector, "status"));
    let replies = match text(vector, "reply") {
        "due" => vec![Reply::Due {
            status,
            card: number(vector, "reply_card"),
            staged_t_ms: number(vector, "staged_t_ms"),
            interval_days: small(vector, "interval_days"),
            due_day: small(vector, "due_day"),
            model_id: model_id(vector["model"].as_u64().expect("a model")),
        }],
        "other" => vec![
            Reply::Ack {
                status,
                folded: 0,
                head_t_ms: 0,
            },
            Reply::Refused { status },
        ],
        kind => panic!("a reply kind: {kind}"),
    };
    Inputs {
        hello,
        epoch: small(vector, "session_epoch"),
        asked,
        replied: small(vector, "replied_epoch"),
        replies,
    }
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("a hex byte"))
        .collect()
}

/// A committed frame by its name.
fn committed(name: &str) -> Vec<u8> {
    FRAMES
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("a JSON frame line"))
        .find(|line| line["name"] == name)
        .map(|line| hex(line["hex"].as_str().expect("a hex frame")))
        .unwrap_or_else(|| panic!("a committed frame {name}"))
}

#[test]
fn the_web_engines_decide_answers_every_vector_byte_equal_with_the_native_decision() {
    let vectors: Vec<Value> = VECTORS
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str(line).expect("a JSON vector"))
        .filter(|vector: &Value| vector["rule"] == "decide")
        .collect();
    let vectors = examined("decision vector(s)", vectors);
    let (mut answers, mut admitted) = (0_usize, 0_usize);
    let mut differing = Vec::new();
    for vector in &vectors {
        let given = inputs(vector);
        let hello = encode_reply(0, &Reply::Hello(given.hello)).expect("a hello frame");
        let preview =
            encode_request(given.epoch, &Request::Preview(given.asked)).expect("a preview frame");
        let session = Session::new(given.epoch, &given.hello);
        for reply in &given.replies {
            let frame = encode_reply(given.replied, reply).expect("a reply frame");
            let native = native_decide(&session, &given.asked, given.replied, reply);
            assert_eq!(
                native.map_err(refusal_name),
                lean_decision(vector),
                "the native decision agrees with the vector: {vector}"
            );
            let web = decide(&hello, given.epoch, &preview, &frame);
            if web != layout(native) {
                differing.push(format!(
                    "{vector}: web {web:?}, native {:?}",
                    layout(native)
                ));
            }
            admitted += usize::from(native.is_ok());
            answers += 1;
        }
    }
    println!("examined {answers} answer(s); {admitted} admitted");
    assert_eq!(
        vectors.len(),
        191,
        "every decision vector the writer prints"
    );
    assert!(admitted > 0, "the vectors admit a due date");
    assert_eq!(differing, Vec::<String>::new());
}

#[test]
fn the_absent_frames_are_refused_by_the_web_engines_decide() {
    let refused = committed("reply-refused");
    let preview = committed("request-preview");
    // The committed refusal is `Refused { Absent }` at epoch 7, and the press's frame is at 7.
    assert_eq!(
        refused,
        encode_reply(
            7,
            &Reply::Refused {
                status: Status::Absent
            }
        )
        .expect("a refusal frame")
    );
    let absent_hello = encode_reply(
        0,
        &Reply::Refused {
            status: Status::Absent,
        },
    )
    .expect("the absent hello");
    let trained = encode_reply(
        0,
        &Reply::Hello(HelloReply {
            status: Status::Ok,
            major: 1,
            minor: 0,
            build_id: [0; 4],
            model_id: model_id(5),
            model: Model::Trained,
            adapter: Adapter::Off,
        }),
    )
    .expect("a trained hello");
    let cases = examined(
        "absent frame case(s)",
        vec![
            // An absent companion's hello negotiates nothing.
            (
                "the absent hello",
                absent_hello.clone(),
                refused.clone(),
                vec![1, 2, 0xff],
            ),
            // A negotiated session whose reply is the absent refusal refuses as absent.
            (
                "the absent reply",
                trained.clone(),
                refused.clone(),
                vec![1, 3, 0xff],
            ),
            // A reply that is no frame is malformed, before the session is asked.
            ("no reply", absent_hello, Vec::new(), vec![1, 1, 0xff]),
            (
                "a cut reply",
                trained,
                refused[..refused.len() - 1].to_vec(),
                vec![1, 1, 0xff],
            ),
        ],
    );
    for (what, hello, reply, expected) in cases {
        assert_eq!(decide(&hello, 7, &preview, &reply), expected, "{what}");
    }
}

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}
