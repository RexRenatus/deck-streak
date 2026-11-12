//! SPEC-345 A17 and A18 (R9; ADR-356 D5): the native adapter's one exempt entry runs an owner's
//! Forget tap on its one card, and each of its refusals reads as its own sentence.
//!
//! The test drives the adapter as a native client does: each request encoded as protobuf bytes by
//! the adapter's shared wire helpers, and the queue read back from the engine's own reply. Every
//! tap goes through one helper, whose one line is the containment census's held line for this
//! file (SPEC-345 section 8). The Forget tap test also runs on each side of a cutoff, and the
//! fixture's cutoff is held twelve hours after its clock's instant (SPEC-443 A3, A5, A6).

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

#[path = "support/clock.rs"]
mod clock;

use std::path::Path;
use std::sync::Arc;

use anki::collection::CollectionBuilder;
use clock::Clock;
use deck_streak_ffi::engine::{Engine, ExemptRefusal, ExemptTap, ExemptTarget, PressedGrade};
use support::{Synthetic, open_request, synthetic, wire};

/// `BackendCollectionService.OpenCollection`, as the engine's generated dispatch numbers it.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
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

/// Opens the synthetic collection through `engine` and answers its one card Good, through an
/// owner's press on the states the queue gave it (SPEC-365 R6), so the card leaves the new queue.
fn answered(synthetic: &Synthetic, engine: Arc<Engine>) -> Arc<Engine> {
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
    engine
        .answer(synthetic.card_id, PressedGrade::Good, states, 4_000)
        .expect("the owner's press reaches the engine");
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

/// An engine started from the default init message, whose every answer records the clock's
/// instant as its time (SPEC-443 R4).
fn on_the_clock(clock: Clock) -> Arc<Engine> {
    Engine::at_answer_time(&[], clock.at_millis())
        .expect("the engine starts from the default init message")
}

/// The synthetic collection for `test`, its cutoff first placed `start` after the clock's instant,
/// then pinned before the adapter opens it (SPEC-443 R5), with the pin's clock.
fn placed(test: &str, start: i64) -> (Synthetic, Clock) {
    let synthetic = synthetic(test);
    clock::place(&synthetic.collection, start);
    let clock = clock::pin(&synthetic.collection);
    (synthetic, clock)
}

/// The engine's own timing of today on a closed collection, read through the engine's own
/// interface: its day count and its next cutoff.
fn engine_day(collection: &Path) -> (u32, i64) {
    let mut col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the collection");
    let timing = col.timing_today().expect("the engine reads its timing");
    col.close(None).expect("the engine closes the collection");
    (timing.days_elapsed, timing.next_day_at.0)
}

/// The Forget tap test's own body (SPEC-345 A17): `engine` answers the synthetic collection's one
/// card, and the tap returns it to the new queue; three taps naming other than it are refused.
fn forget_tap_resets_its_one_card(synthetic: &Synthetic, engine: Arc<Engine>) {
    let engine = answered(synthetic, engine);
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
fn a_native_forget_tap_resets_its_one_card() {
    let synthetic = synthetic("exempt-forget");
    let clock = clock::pin(&synthetic.collection);
    forget_tap_resets_its_one_card(&synthetic, on_the_clock(clock));
}

/// SPEC-443 A3 (R4, R5): the Forget tap test's own body, on a collection first placed five minutes
/// before its cutoff, then on one first placed five minutes after it, each then pinned.
#[test]
fn a_native_forget_tap_resets_its_one_card_on_each_side_of_the_cutoff() {
    let (synthetic, clock) = placed("exempt-forget-just-before", clock::JUST_BEFORE_SECS);
    forget_tap_resets_its_one_card(&synthetic, on_the_clock(clock));
    let (synthetic, clock) = placed("exempt-forget-just-after", clock::JUST_AFTER_SECS);
    forget_tap_resets_its_one_card(&synthetic, on_the_clock(clock));
}

/// SPEC-443 A5 (R1, R2): the adapter's fixture, first placed five minutes before its cutoff and
/// then pinned, holds the engine's next cutoff more than 11 and at most 12 hours after its clock's
/// instant, at day 0, as the engine's own timing reads it.
#[test]
fn the_adapter_fixture_holds_the_engines_cutoff_twelve_hours_after_the_clock() {
    let (synthetic, clock) = placed("exempt-fixture-cutoff", clock::JUST_BEFORE_SECS);
    let (days, cutoff) = engine_day(&synthetic.collection);
    let at_secs = clock.at_secs();
    assert_eq!(
        (days, cutoff - at_secs > 39_600, cutoff - at_secs <= 43_200),
        (0, true, true),
        "the engine's day count and next cutoff on the adapter's fixture: day 0, the cutoff more \
         than 39 600 s and at most 43 200 s after the clock's instant; read day {days}, the cutoff \
         {cutoff}, {} s after the instant {at_secs}",
        cutoff - at_secs
    );
}

/// SPEC-443 A6 (R6): five minutes before a cutoff the fixture has not pinned, a Good press takes
/// the new card past its ten-minute step into the day-learning queue, due on day 1.
#[test]
fn an_adapter_good_press_just_before_the_cutoff_moves_the_card_to_tomorrow() {
    let synthetic = synthetic("exempt-good-press-just-before");
    let just_before = clock::place(&synthetic.collection, clock::JUST_BEFORE_SECS);
    drop(answered(&synthetic, on_the_clock(just_before)));
    let col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine opens the collection the adapter let go");
    let stored: (i64, i64) = col
        .storage
        .db()
        .query_row(
            "select queue, due from cards where id = ?",
            [synthetic.card_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the engine stores the card");
    col.close(None).expect("the engine closes the collection");
    assert_eq!(
        stored,
        (3, 1),
        "the Good press five minutes before the cutoff left the card in the day-learning queue, due \
         on day 1"
    );
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
        (
            ExemptRefusal::NeedsTheChoice,
            "the one-way sync runs only through the full-sync choice's write",
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
