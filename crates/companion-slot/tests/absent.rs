//! The public slot is Absent (SPEC-501 C6): the companion `open` gives answers `Refused` with
//! status `Absent` to every request kind, whatever the model bytes, and a planted slot that
//! answers a due date is refused by the same enumeration.

// An integration test is test code: it prints the examined count on purpose.
#![allow(clippy::print_stdout)]

use deck_streak_companion::port::Companion;
use deck_streak_companion::wire::{
    Adapter, CheckpointReply, Preview, Reply, Request, Restore, ReviewEvent, Status,
};
use deck_streak_companion_slot::open;

fn event() -> ReviewEvent {
    ReviewEvent {
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
    }
}

/// One request of every kind schema v1 has.
fn every_kind() -> Vec<Request> {
    vec![
        Request::Hello {
            major: 1,
            minor: 0,
            release: [0, 3, 0],
        },
        Request::Fold {
            events: vec![event()],
        },
        Request::Preview(Preview {
            event: event(),
            target_ppm: 900_000,
        }),
        Request::Commit { event: event() },
        Request::Retract {
            t_ms: 1_000,
            origin: [1, 2],
            seq: 3,
        },
        Request::Checkpoint,
        Request::Restore(Restore::from(CheckpointReply {
            status: Status::Ok,
            model_id: [1, 2, 3, 4],
            adapter: Adapter::Off,
            bytes: vec![1, 2, 3],
        })),
        Request::Purge,
        Request::StateHash,
    ]
}

/// Every request kind `companion` answers with anything but `Refused { Absent }`.
fn findings(companion: &mut dyn Companion) -> (usize, Vec<String>) {
    let requests = every_kind();
    let mut found = Vec::new();
    for (epoch, request) in (1_u32..).zip(&requests) {
        let reply = companion.call(epoch, request);
        if reply
            != (Reply::Refused {
                status: Status::Absent,
            })
        {
            found.push(format!("{request:?}: {reply:?}"));
        }
    }
    (requests.len(), found)
}

/// A planted slot that answers every request with a due date.
struct AnswersDue;

impl Companion for AnswersDue {
    fn call(&mut self, _epoch: u32, _request: &Request) -> Reply {
        Reply::Due {
            status: Status::Ok,
            card: 42,
            staged_t_ms: 1_000,
            interval_days: 3,
            due_day: 103,
            model_id: [1, 2, 3, 4],
        }
    }
}

#[test]
fn the_public_slot_refuses_every_request_kind_as_absent() {
    for weights in [&[][..], &[1, 2, 3][..]] {
        let mut companion = open(weights);
        let (examined, found) = findings(companion.as_mut());
        println!(
            "examined {examined} of 9 request kind(s) over {} weight byte(s)",
            weights.len()
        );
        assert_eq!(examined, 9);
        assert_eq!(found, Vec::<String>::new());
    }
}

#[test]
fn a_slot_that_answers_a_due_date_is_refused() {
    let (examined, found) = findings(&mut AnswersDue);
    println!(
        "examined {examined} request kind(s) over the planted slot: {} finding(s)",
        found.len()
    );
    assert_eq!(found.len(), 9, "{found:?}");
}
