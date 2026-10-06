//! SPEC-345 A17 and A18 (R9; ADR-356 D5): the native adapter's one exempt entry runs an owner's
//! Forget tap on its one card, and each of its refusals reads as its own sentence.
//!
//! The test drives the adapter as a native client does: each request encoded as protobuf bytes by
//! the adapter's shared wire helpers, and the queue read back from the engine's own reply. Every
//! tap goes through one helper, whose one line is the containment census's held line for this
//! file (SPEC-345 section 8).

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_ffi::engine::{Engine, ExemptRefusal, ExemptTap, ExemptTarget};
use support::{Synthetic, open_request, synthetic, wire};

/// `BackendCollectionService.OpenCollection`, as the engine's generated dispatch numbers it.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// `SchedulerService.AnswerCard`.
const ANSWER_CARD: (u32, u32) = (13, 4);
/// The engine's rating for Good (`CardAnswer.Rating.GOOD`).
const GOOD: u64 = 2;
/// The engine's queue for a new card (`QueuedCards.Queue.NEW`).
const NEW_QUEUE: u64 = 0;
/// The engine's queue for a card in learning (`QueuedCards.Queue.LEARNING`).
const LEARNING_QUEUE: u64 = 1;

/// Prints how many `what` a test examined and refuses none: a population that came back empty
/// judged nothing.
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Every tap of this file reaches the adapter's exempt entry here, and only here.
fn tap(
    engine: &Engine,
    write: ExemptTap,
    target: ExemptTarget,
    input: Vec<u8>,
) -> Result<Vec<u8>, ExemptRefusal> {
    engine.run_exempt(write, target, input)
}

/// The head of the queue and its counts, as a `QueuedCards` reply carries them.
#[derive(Debug, PartialEq, Eq)]
struct Head {
    card_id: i64,
    queue: u64,
    new: u64,
    learning: u64,
}

fn queue(engine: &Engine) -> Head {
    let mut request = Vec::new();
    wire::put_varint_field(&mut request, 1, 1);
    let (service, method) = GET_QUEUED_CARDS;
    let reply = engine
        .run(service, method, request)
        .expect("the queue is on the allow-list");
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

/// Opens the synthetic collection and answers its one card Good, through ordinary calls, so the
/// card leaves the new queue.
fn answered(synthetic: &Synthetic) -> std::sync::Arc<Engine> {
    let engine = Engine::new(Vec::new()).expect("the engine starts from the default init message");
    let (service, method) = OPEN_COLLECTION;
    engine
        .run(service, method, open_request(synthetic))
        .expect("the adapter opens the collection");
    let mut request = Vec::new();
    wire::put_varint_field(&mut request, 1, 1);
    let (service, method) = GET_QUEUED_CARDS;
    let queued = engine
        .run(service, method, request)
        .expect("the queue is on the allow-list");
    let states = wire::bytes(
        &wire::repeated(&queued, 1)
            .into_iter()
            .next()
            .expect("the new card is queued"),
        3,
    );
    let answered_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    let mut answer = Vec::new();
    wire::put_varint_field(
        &mut answer,
        1,
        u64::try_from(synthetic.card_id).expect("a card id is positive"),
    );
    wire::put_bytes(&mut answer, 2, &wire::bytes(&states, 1));
    wire::put_bytes(&mut answer, 3, &wire::bytes(&states, 4));
    wire::put_varint_field(&mut answer, 4, GOOD);
    wire::put_varint_field(
        &mut answer,
        5,
        u64::try_from(answered_at).expect("the time fits 64 bits"),
    );
    wire::put_varint_field(&mut answer, 6, 4_000);
    let (service, method) = ANSWER_CARD;
    engine
        .run(service, method, answer)
        .expect("the answer reaches the engine");
    engine
}

/// `ScheduleCardsAsNewRequest` naming `cards`, each in its own `card_ids` field.
fn forget(cards: &[i64]) -> Vec<u8> {
    let mut out = Vec::new();
    for card in cards {
        wire::put_varint_field(
            &mut out,
            1,
            u64::try_from(*card).expect("a card id is positive"),
        );
    }
    out
}

#[test]
fn a_native_forget_tap_resets_its_one_card() {
    let synthetic = synthetic("exempt-forget");
    let engine = answered(&synthetic);
    let learning = queue(&engine);
    let card = synthetic.card_id;
    let ran = tap(
        &engine,
        ExemptTap::Forget,
        ExemptTarget::Card { id: card },
        forget(&[card]),
    )
    .map(|_| ());
    assert_eq!(
        (ran, queue(&engine)),
        (
            Ok(()),
            Head {
                card_id: card,
                queue: NEW_QUEUE,
                new: 1,
                learning: 0,
            }
        ),
        "the Forget tap returned its one card to the new queue: before it, {learning:?}"
    );
    assert_eq!(
        learning,
        Head {
            card_id: card,
            queue: LEARNING_QUEUE,
            new: 0,
            learning: 1,
        },
        "the answer took the card out of the new queue before the tap"
    );
    let refused = vec![
        tap(
            &engine,
            ExemptTap::Forget,
            ExemptTarget::Note {
                id: synthetic.note_id,
            },
            forget(&[card]),
        ),
        tap(
            &engine,
            ExemptTap::Forget,
            ExemptTarget::Card { id: card },
            forget(&[card, card + 1]),
        ),
        tap(
            &engine,
            ExemptTap::Forget,
            ExemptTarget::Card { id: card },
            vec![0x0a, 0x05, 0x01],
        ),
    ];
    assert_eq!(
        refused,
        vec![
            Err(ExemptRefusal::WrongKind),
            Err(ExemptRefusal::NotTheTarget),
            Err(ExemptRefusal::Undecodable),
        ],
        "a tap on another kind, a request naming more, and bytes of no message are each refused"
    );
    examined("refused tap(s)", refused);
}

#[test]
fn each_exempt_refusal_reads_as_its_own_sentence() {
    let sentences: Vec<(ExemptRefusal, &str)> = vec![
        (
            ExemptRefusal::WrongKind,
            "the tap's target is not of the kind its write takes",
        ),
        (
            ExemptRefusal::NotTheTarget,
            "the request names other than the tap's one target",
        ),
        (
            ExemptRefusal::Undecodable,
            "the request is not the tap's own write",
        ),
        (
            ExemptRefusal::Engine { error: vec![0; 4] },
            "the engine refused the write (4 bytes)",
        ),
    ];
    let read: Vec<(String, &str)> = sentences
        .iter()
        .map(|(refusal, sentence)| (refusal.to_string(), *sentence))
        .collect();
    assert_eq!(
        read.iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>(),
        read.iter()
            .map(|(_, sentence)| *sentence)
            .collect::<Vec<_>>(),
        "each exempt refusal reads as its own sentence"
    );
    examined("exempt refusal(s)", read);
}
