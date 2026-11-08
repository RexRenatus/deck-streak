//! SPEC-638 A1 (R1, R2), ADR-661 D1 and D2: a native press on the states a card was shown with,
//! after another answer changed the card, reaches its caller through the adapter's own entry as
//! the engine's refusal, and records nothing.
//!
//! The test drives the adapter as a native client does: it opens the synthetic collection, reads
//! the head card's states from the queue's own reply, and presses Good on them twice through
//! `Engine::answer`, with the same grade and the same milliseconds. The first press is recorded
//! and changes the card, so the second, on the states shown before it, is stale. The test asserts
//! the second press's outcome as its caller reads it, the engine's error kind and its message
//! head, and the queue after it as the first press left it.
//!
//! MUTATION COVERAGE, not red first: the handover is on dev; rows S63801 and S63802 are its reds.

#![allow(clippy::expect_used, reason = "a failed fixture should fail its test")]

#[expect(dead_code, reason = "this test never reads the synthetic note id")]
mod support;

use deck_streak_ffi::engine::{Engine, PressRefusal, PressedGrade};
use support::{open_request, synthetic, wire};

/// `BackendCollectionService.OpenCollection`, as the engine's generated dispatch numbers it.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// The engine's error kind for invalid input, `BackendError.Kind.INVALID_INPUT`; as the enum's
/// default, the wire omits it and the field reads 0.
const INVALID_INPUT: u64 = 0;
/// The head of the engine's refusal of a stale current state, its message up to its first colon,
/// spelt here and never read from the engine.
const STALE: &str = "card was modified";
/// The milliseconds each press says it took.
const MILLISECONDS: u32 = 4_000;

/// The head of the queue and its counts, as a `QueuedCards` reply carries them.
#[derive(Debug, PartialEq, Eq)]
struct Head {
    card_id: i64,
    queue: u64,
    new: u64,
    learning: u64,
}

/// A press's outcome as the adapter's caller reads it.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// The press was recorded, and the adapter returned the engine's reply of this many bytes.
    Recorded(usize),
    /// The engine refused the press: its error's kind and its message up to the first colon.
    EngineRefused(u64, String),
    /// The adapter refused the press for another reason, named.
    Other(PressRefusal),
}

/// The queue's reply for its head card alone.
fn queued(engine: &Engine) -> Vec<u8> {
    let mut request = Vec::new();
    wire::put_varint_field(&mut request, 1, 1);
    let (service, method) = GET_QUEUED_CARDS;
    engine
        .run(service, method, request)
        .expect("the queue is on the allow-list")
}

fn head(engine: &Engine) -> Head {
    let reply = queued(engine);
    let head = wire::repeated(&reply, 1)
        .into_iter()
        .next()
        .unwrap_or_default();
    Head {
        card_id: wire::signed(&wire::bytes(&head, 1), 1),
        queue: wire::varint(&head, 2),
        new: wire::varint(&reply, 2),
        learning: wire::varint(&reply, 3),
    }
}

/// A press's result read as its caller reads it, the engine's error decoded from its own bytes.
fn outcome(press: Result<Vec<u8>, PressRefusal>) -> Outcome {
    match press {
        Ok(recorded) => Outcome::Recorded(recorded.len()),
        Err(PressRefusal::Engine { error }) => {
            let message = String::from_utf8(wire::bytes(&error, 1))
                .expect("the engine's error message is UTF-8");
            let head = match message.split_once(':') {
                Some((head, _)) => head.to_owned(),
                None => message,
            };
            Outcome::EngineRefused(wire::varint(&error, 2), head)
        }
        Err(other) => Outcome::Other(other),
    }
}

/// SPEC-638 A1 (R1, R2): the head card's Good press is recorded, and a second Good press on the
/// states the queue showed before the first is the engine's refusal, handed to the caller as
/// `PressRefusal::Engine` with no bytes recorded, and the queue reads as the first press left it.
#[test]
fn a_native_press_on_a_card_changed_since_it_was_shown_is_the_engines_refusal_and_records_nothing()
{
    let synthetic = synthetic("stale-press");
    let engine = Engine::new(Vec::new()).expect("the engine starts from the default init message");
    let (service, method) = OPEN_COLLECTION;
    engine
        .run(service, method, open_request(&synthetic))
        .expect("the adapter opens the collection");
    let shown = wire::bytes(
        &wire::repeated(&queued(&engine), 1)
            .into_iter()
            .next()
            .expect("the new card is queued"),
        3,
    );
    let first = engine.answer(
        synthetic.card_id,
        PressedGrade::Good,
        shown.clone(),
        MILLISECONDS,
    );
    let after_first = head(&engine);
    let second = engine.answer(synthetic.card_id, PressedGrade::Good, shown, MILLISECONDS);
    let after_second = head(&engine);
    assert_eq!(
        (first.is_ok(), outcome(second), after_second),
        (
            true,
            Outcome::EngineRefused(INVALID_INPUT, STALE.to_owned()),
            after_first
        ),
        "the first press is recorded; the second, on the states shown before it, reaches its \
         caller as the engine's refusal (kind {INVALID_INPUT}, head `{STALE}`); and the queue after \
         the refusal reads as the first press left it"
    );
}
