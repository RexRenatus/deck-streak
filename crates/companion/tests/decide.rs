//! The decision and the handshake (SPEC-501 C4 and C5). `decide_frame` refuses each of its ten
//! reasons by name and admits the one valid case; `negotiate` is `Ok` only for a present
//! companion at major 1 holding a trained model, over every status, three majors and both
//! models. Each enumeration is run once more over a planted wrong function, which it must refuse.

// An integration test is test code: it prints the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_companion::codec::encode_reply;
use deck_streak_companion::decide::{NewDue, Refusal, Session, decide_frame, negotiate};
use deck_streak_companion::wire::{
    Adapter, HelloReply, MAJOR, MINOR, Model, Preview, Reply, ReviewEvent, Status,
};

const EPOCH: u32 = 7;
const MODEL: [u64; 4] = [5, 6, 7, 8];

const STATUSES: [Status; 7] = [
    Status::Ok,
    Status::Absent,
    Status::FixtureRefuses,
    Status::Busy,
    Status::Error,
    Status::Conflict,
    Status::Unsupported,
];

fn hello(status: Status, major: u16, model: Model) -> HelloReply {
    HelloReply {
        status,
        major,
        minor: MINOR,
        build_id: [1, 2, 3, 4],
        model_id: MODEL,
        model,
        adapter: Adapter::Off,
    }
}

fn negotiated() -> Session {
    Session::new(EPOCH, &hello(Status::Ok, MAJOR, Model::Trained))
}

fn asked() -> Preview {
    Preview {
        event: ReviewEvent {
            t_ms: 1_000,
            origin: [1, 2],
            seq: 3,
            card: 42,
            note: 43,
            home_deck: 44,
            preset: 1,
            grade: 3,
            kind: 1,
            duration_ms: 4_000,
            study_day: 100,
        },
        target_ppm: 900_000,
    }
}

/// The valid reply: card 42's press at 1,000, due in 3 days on study day 103.
fn due(status: Status, card: i64, staged: i64, interval: u32, day: i32, model: [u64; 4]) -> Reply {
    Reply::Due {
        status,
        card,
        staged_t_ms: staged,
        interval_days: interval,
        due_day: day,
        model_id: model,
    }
}

fn frame(epoch: u32, reply: &Reply) -> Vec<u8> {
    encode_reply(epoch, reply).expect("encodes")
}

type DecideFrame = fn(&Session, &Preview, &[u8]) -> Result<NewDue, Refusal>;

/// One case: its name, the session, the reply frame and what the decision must answer.
type Case = (&'static str, Session, Vec<u8>, Result<NewDue, Refusal>);

/// The eleven cases: each of the ten refusals, then the one valid case.
fn cases() -> Vec<Case> {
    let good = due(Status::Ok, 42, 1_000, 3, 103, MODEL);
    let mut truncated = frame(EPOCH, &good);
    truncated.pop();
    let unnegotiated = Session::new(EPOCH, &hello(Status::Ok, MAJOR, Model::Fixture));
    vec![
        (
            "malformed",
            negotiated(),
            truncated,
            Err(Refusal::Malformed),
        ),
        (
            "not negotiated",
            unnegotiated,
            frame(EPOCH, &good),
            Err(Refusal::NotNegotiated),
        ),
        (
            "absent",
            negotiated(),
            frame(
                EPOCH,
                &Reply::Refused {
                    status: Status::Absent,
                },
            ),
            Err(Refusal::Absent),
        ),
        (
            "status",
            negotiated(),
            frame(EPOCH, &due(Status::Busy, 42, 1_000, 3, 103, MODEL)),
            Err(Refusal::Status(Status::Busy)),
        ),
        (
            "stale epoch",
            negotiated(),
            frame(EPOCH - 1, &good),
            Err(Refusal::StaleEpoch),
        ),
        (
            "wrong card",
            negotiated(),
            frame(EPOCH, &due(Status::Ok, 41, 1_000, 3, 103, MODEL)),
            Err(Refusal::WrongCard),
        ),
        (
            "wrong stage",
            negotiated(),
            frame(EPOCH, &due(Status::Ok, 42, 999, 3, 103, MODEL)),
            Err(Refusal::WrongStage),
        ),
        (
            "model changed",
            negotiated(),
            frame(EPOCH, &due(Status::Ok, 42, 1_000, 3, 103, [5, 6, 7, 9])),
            Err(Refusal::ModelChanged),
        ),
        (
            "interval out of range",
            negotiated(),
            frame(EPOCH, &due(Status::Ok, 42, 1_000, 36_501, 36_601, MODEL)),
            Err(Refusal::IntervalOutOfRange),
        ),
        (
            "due day inconsistent",
            negotiated(),
            frame(EPOCH, &due(Status::Ok, 42, 1_000, 3, 104, MODEL)),
            Err(Refusal::DueDayInconsistent),
        ),
        (
            "valid",
            negotiated(),
            frame(EPOCH, &good),
            Ok(NewDue {
                card: 42,
                interval_days: 3,
                due_day: 103,
            }),
        ),
    ]
}

/// Every case `decider` does not answer as the table says.
fn decide_findings(decider: DecideFrame) -> Vec<String> {
    cases()
        .into_iter()
        .filter_map(|(name, session, bytes, expected)| {
            let got = decider(&session, &asked(), &bytes);
            (got != expected).then(|| format!("{name}: {got:?}, not {expected:?}"))
        })
        .collect()
}

/// A planted decision that admits every reply.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a planted decision keeps the decision's signature"
)]
fn accept_all(_session: &Session, asked: &Preview, _frame: &[u8]) -> Result<NewDue, Refusal> {
    Ok(NewDue {
        card: asked.event.card,
        interval_days: 3,
        due_day: asked.event.study_day + 3,
    })
}

#[test]
fn decide_frame_refuses_each_of_its_ten_reasons_and_admits_the_one_valid_case() {
    let examined = cases();
    let refusals: Vec<_> = examined.iter().filter(|case| case.3.is_err()).collect();
    println!(
        "examined {} of 11 case(s), {} refusal(s)",
        examined.len(),
        refusals.len()
    );
    assert_eq!((examined.len(), refusals.len()), (11, 10));
    assert_eq!(decide_findings(decide_frame), Vec::<String>::new());
}

#[test]
fn an_accept_all_decide_is_refused_by_the_reason_table() {
    let findings = decide_findings(accept_all);
    println!(
        "examined the accept-all decide: {} finding(s)",
        findings.len()
    );
    assert_eq!(findings.len(), 10, "{findings:?}");
}

#[test]
fn the_interval_bounds_and_the_due_days_arithmetic_are_exact() {
    let at = |interval: u32, day: i32, study_day: i32| {
        let mut preview = asked();
        preview.event.study_day = study_day;
        decide_frame(
            &negotiated(),
            &preview,
            &frame(EPOCH, &due(Status::Ok, 42, 1_000, interval, day, MODEL)),
        )
    };
    assert_eq!(at(0, 100, 100), Err(Refusal::IntervalOutOfRange));
    assert!(at(1, 101, 100).is_ok());
    assert!(at(36_500, 36_600, 100).is_ok());
    assert_eq!(at(36_501, 36_601, 100), Err(Refusal::IntervalOutOfRange));
    // A study day near the top of i32 overflows; no wrapped due day is admitted.
    assert_eq!(
        at(100, i32::MAX, i32::MAX - 50),
        Err(Refusal::DueDayInconsistent)
    );
    assert_eq!(
        at(100, (i32::MAX - 50).wrapping_add(100), i32::MAX - 50),
        Err(Refusal::DueDayInconsistent)
    );
}

#[test]
fn a_reply_that_is_not_a_due_date_refuses_by_its_status() {
    let reply = |reply: Reply| decide_frame(&negotiated(), &asked(), &frame(EPOCH, &reply));
    let ack = |status| Reply::Ack {
        status,
        folded: 1,
        head_t_ms: 1_000,
    };
    assert_eq!(reply(ack(Status::Ok)), Err(Refusal::WrongStage));
    assert_eq!(reply(ack(Status::Absent)), Err(Refusal::Absent));
    assert_eq!(reply(ack(Status::Busy)), Err(Refusal::Status(Status::Busy)));
}

type Negotiate = fn(&HelloReply) -> Result<[u64; 4], Refusal>;

/// What the handshake must answer for one hello.
fn expected(hello: &HelloReply) -> Result<[u64; 4], Refusal> {
    match (hello.status, hello.major, hello.model) {
        (Status::Absent, _, _) => Err(Refusal::Absent),
        (Status::Ok, 1, Model::Trained) => Ok(hello.model_id),
        (Status::Ok, 1, Model::Fixture) => Err(Refusal::Fixture),
        (Status::Ok, _, _) => Err(Refusal::Mismatch),
        (status, _, _) => Err(Refusal::Status(status)),
    }
}

/// Every hello of the cross product `negotiator` does not answer as [`expected`] does, and the
/// number examined.
fn negotiate_findings(negotiator: Negotiate) -> (usize, Vec<String>) {
    let mut examined = 0;
    let mut findings = Vec::new();
    for status in STATUSES {
        for major in [0, 1, 2] {
            for model in [Model::Trained, Model::Fixture] {
                examined += 1;
                let hello = hello(status, major, model);
                let got = negotiator(&hello);
                let want = expected(&hello);
                if got != want {
                    findings.push(format!(
                        "{status:?} {major} {model:?}: {got:?}, not {want:?}"
                    ));
                }
                if got.is_ok() != (status == Status::Ok && major == 1 && model == Model::Trained) {
                    findings.push(format!("{status:?} {major} {model:?}: admitted wrongly"));
                }
            }
        }
    }
    (examined, findings)
}

/// A planted handshake that compares the minor where the major belongs.
fn minor_only(hello: &HelloReply) -> Result<[u64; 4], Refusal> {
    match hello.status {
        Status::Ok => {}
        Status::Absent => return Err(Refusal::Absent),
        status => return Err(Refusal::Status(status)),
    }
    if hello.minor != MINOR {
        return Err(Refusal::Mismatch);
    }
    if hello.model != Model::Trained {
        return Err(Refusal::Fixture);
    }
    Ok(hello.model_id)
}

/// A planted handshake that never reads the model.
fn mode_blind(hello: &HelloReply) -> Result<[u64; 4], Refusal> {
    match hello.status {
        Status::Ok => {}
        Status::Absent => return Err(Refusal::Absent),
        status => return Err(Refusal::Status(status)),
    }
    if hello.major != MAJOR {
        return Err(Refusal::Mismatch);
    }
    Ok(hello.model_id)
}

#[test]
fn negotiate_admits_only_a_present_companion_at_major_one_with_a_trained_model() {
    let (examined, findings) = negotiate_findings(negotiate);
    println!("examined {examined} hello(s)");
    assert_eq!(examined, 42);
    assert_eq!(findings, Vec::<String>::new());
    // A minor difference is admitted.
    let newer = HelloReply {
        minor: MINOR + 3,
        ..hello(Status::Ok, MAJOR, Model::Trained)
    };
    assert_eq!(negotiate(&newer), Ok(MODEL));
    // A failed handshake leaves the session with no model, so every decision refuses.
    let failed = Session::new(EPOCH, &hello(Status::Ok, 2, Model::Trained));
    assert_eq!(failed.model(), None);
    assert_eq!(negotiated().model(), Some(MODEL));
}

#[test]
fn a_minor_only_and_a_mode_blind_negotiate_are_each_refused() {
    for (name, planted) in [
        ("minor-only", minor_only as Negotiate),
        ("mode-blind", mode_blind),
    ] {
        let (examined, findings) = negotiate_findings(planted);
        println!(
            "examined {examined} hello(s) over the {name} handshake: {} finding(s)",
            findings.len()
        );
        assert!(!findings.is_empty(), "the {name} handshake is refused");
    }
}
