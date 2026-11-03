//! The decision and the handshake answer every vector the Lean port writes (SPEC-501 C8): for each
//! input in `formal/vectors/companion-decide.jsonl`, written by
//! `formal/lean/Formal/CompanionDecide.lean`'s port, `decide` gives the same outcome and the same
//! due date, and `negotiate` gives the same outcome and model id. A model id is compared only, so
//! the port's one number is the id's first word.

// An integration test is test code: its helpers panic on a malformed vector, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_companion::decide::{NewDue, Refusal, Session, decide, negotiate};
use deck_streak_companion::wire::{
    Adapter, HelloReply, Model, Preview, Reply, ReviewEvent, Status,
};
use serde_json::Value;

/// The vectors, as the Lean writer printed them.
const VECTORS: &str = include_str!("../../../formal/vectors/companion-decide.jsonl");

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

/// A refusal's name, as the Lean writer spells it.
fn refusal_name(refusal: Refusal) -> String {
    let name = match refusal {
        Refusal::Malformed => "malformed",
        Refusal::NotNegotiated => "not_negotiated",
        Refusal::Absent => "absent",
        Refusal::Mismatch => "mismatch",
        Refusal::Fixture => "fixture",
        Refusal::Status(status) => {
            return format!(
                "status:{}",
                match status {
                    Status::Ok => "ok",
                    Status::Absent => "absent",
                    Status::FixtureRefuses => "fixture_refuses",
                    Status::Busy => "busy",
                    Status::Error => "error",
                    Status::Conflict => "conflict",
                    Status::Unsupported => "unsupported",
                }
            );
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

fn model_id(first: u64) -> [u64; 4] {
    [first, 0, 0, 0]
}

/// The outcome the Lean port printed for a decision, as `decide`'s own type.
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

/// `decide` over one vector, with the non-due reply read once as an `Ack` and once as a
/// `Refused`; both must agree.
fn our_decision(vector: &Value) -> Result<NewDue, String> {
    let session_epoch = small::<u32>(vector, "session_epoch");
    let hello = |model| HelloReply {
        status: Status::Ok,
        major: 1,
        minor: 0,
        build_id: [0; 4],
        model_id: model_id(vector["session_model"].as_u64().unwrap_or(0)),
        model,
        adapter: Adapter::Off,
    };
    let session = if vector["session_model"].is_null() {
        Session::new(session_epoch, &hello(Model::Fixture))
    } else {
        Session::new(session_epoch, &hello(Model::Trained))
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
    let replied = small::<u32>(vector, "replied_epoch");
    let status = status(text(vector, "status"));
    let candidates = match text(vector, "reply") {
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
    let outcomes: Vec<_> = candidates
        .iter()
        .map(|reply| decide(&session, &asked, replied, reply).map_err(refusal_name))
        .collect();
    assert!(
        outcomes.windows(2).all(|pair| pair[0] == pair[1]),
        "{vector}"
    );
    outcomes.into_iter().next().expect("one outcome")
}

fn our_handshake(vector: &Value) -> Result<u64, String> {
    let hello = HelloReply {
        status: status(text(vector, "status")),
        major: small(vector, "major"),
        minor: small(vector, "minor"),
        build_id: [0; 4],
        model_id: model_id(vector["hello_model_id"].as_u64().expect("a model id")),
        model: match text(vector, "model") {
            "trained" => Model::Trained,
            "fixture" => Model::Fixture,
            model => panic!("a model: {model}"),
        },
        adapter: Adapter::Off,
    };
    negotiate(&hello)
        .map(|model| model[0])
        .map_err(refusal_name)
}

fn lean_handshake(vector: &Value) -> Result<u64, String> {
    match text(vector, "outcome") {
        "ok" => Ok(vector["model_id"].as_u64().expect("a model id")),
        refusal => Err(refusal.to_owned()),
    }
}

#[test]
fn decide_and_negotiate_answer_every_lean_vector() {
    let mut lines = VECTORS.lines();
    let header: Value =
        serde_json::from_str(lines.next().expect("a header line")).expect("a JSON header");
    assert_eq!(header["schema"], "phx.formal.vectors.v1", "{header}");
    assert_eq!(header["entry"], "CompanionDecide", "{header}");
    assert_eq!(
        header["covers"], "crates/companion/src/decide.rs",
        "{header}"
    );
    let (mut decisions, mut handshakes, mut admitted) = (0_u64, 0_u64, 0_u64);
    let mut reasons = std::collections::BTreeSet::new();
    for line in lines {
        let vector: Value = serde_json::from_str(line).expect("a JSON vector");
        match text(&vector, "rule") {
            "decide" => {
                let lean = lean_decision(&vector);
                assert_eq!(our_decision(&vector), lean, "{vector}");
                match lean {
                    Ok(_) => admitted += 1,
                    Err(reason) => {
                        reasons.insert(reason.split(':').next().unwrap_or("").to_owned());
                    }
                }
                decisions += 1;
            }
            "negotiate" => {
                assert_eq!(our_handshake(&vector), lean_handshake(&vector), "{vector}");
                handshakes += 1;
            }
            rule => panic!("a vector of a known rule: {rule}"),
        }
    }
    println!(
        "examined {decisions} decision and {handshakes} handshake vector(s); {admitted} admitted; \
         refusals {reasons:?}"
    );
    assert_eq!(
        (decisions, handshakes),
        (191, 84),
        "every input the writer prints"
    );
    assert_eq!(
        header["vectors"].as_u64(),
        Some(decisions + handshakes),
        "{header}"
    );
    assert_eq!(
        reasons.len(),
        9,
        "the vectors reach each of decide's nine refusals: {reasons:?}"
    );
}
