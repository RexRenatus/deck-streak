//! The review screen's three pairs on the adapter's allow-list (SPEC-348 R1, A2).
//!
//! The test drives the adapter the way a native client does, over the synthetic collection the
//! round-trip tests open, each request encoded as protobuf bytes and each reply decoded from them.
//! The pairs are read from the engine's generated dispatch at the pinned rev, not from the
//! adapter's table, so a wrong entry fails here. It then holds the whole allow-list equal to a set
//! it writes out itself: the core's native column, which the core's parity test compares with it.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

#[expect(
    dead_code,
    reason = "this test reads the card's id alone, never the note's or the note's text"
)]
mod support;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use deck_streak_ffi::allow_list::ALLOW_LIST;
use deck_streak_ffi::engine::{Engine, EngineRefusal};
use support::synthetic::SECOND_DECK;
use support::{open_request, synthetic, wire};

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `DecksService.DeckTree`.
const DECK_TREE: (u32, u32) = (7, 4);
/// `DecksService.SetCurrentDeck`.
const SET_CURRENT_DECK: (u32, u32) = (7, 22);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// `SchedulerService.DescribeNextStates`.
const DESCRIBE_NEXT_STATES: (u32, u32) = (13, 24);
/// The engine's id for its own default deck.
const DEFAULT_DECK: i64 = 1;
/// The engine's words for a new card's Again, Hard and Good intervals on the default preset at the
/// pinned rev. Easy is fuzzed by a seed the card's id makes, so the core's test, which fixes the id,
/// pins all four.
const LEARNING_INTERVALS: [&str; 3] = ["<1m", "<6m", "<10m"];

/// The allow-list this delivery leaves: the six calls before the app shell, the shell's login and
/// the review screen's three pairs, each with the engine's name for it.
const EXPECTED: [(u32, u32, &str); 10] = [
    (1, 3, "BackendSyncService.SyncLogin"),
    (3, 0, "BackendCollectionService.OpenCollection"),
    (3, 8, "CollectionService.Undo"),
    (7, 4, "DecksService.DeckTree"),
    (7, 13, "DecksService.GetDeckNames"),
    (7, 22, "DecksService.SetCurrentDeck"),
    (13, 3, "SchedulerService.GetQueuedCards"),
    (13, 4, "SchedulerService.AnswerCard"),
    (13, 24, "SchedulerService.DescribeNextStates"),
    (27, 6, "CardRenderingService.RenderExistingCard"),
];

fn engine() -> Arc<Engine> {
    Engine::new(Vec::new()).expect("the engine starts from the default init message")
}

fn call(
    engine: &Engine,
    (service, method): (u32, u32),
    input: Vec<u8>,
) -> Result<Vec<u8>, EngineRefusal> {
    engine.run(service, method, input)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_secs()
}

/// `DeckId { did }`.
fn deck_id(did: i64) -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_varint_field(&mut out, 1, u64::from_ne_bytes(did.to_ne_bytes()));
    out
}

/// `GetQueuedCardsRequest { fetch_limit: 1 }`, then each queued card's bytes.
fn queued(engine: &Engine) -> Vec<Vec<u8>> {
    let mut request = Vec::new();
    wire::put_varint_field(&mut request, 1, 1);
    let reply = call(engine, GET_QUEUED_CARDS, request).expect("the queue is on the allow-list");
    wire::repeated(&reply, 1)
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("the engine writes UTF-8")
}

#[test]
fn the_allow_list_carries_the_review_pairs() {
    let synthetic = synthetic("review-pairs");
    let engine = engine();
    call(&engine, OPEN_COLLECTION, open_request(&synthetic))
        .expect("the adapter opens the synthetic collection");

    // (7,4): the root's children are the two decks, by name.
    let mut tree_request = Vec::new();
    wire::put_varint_field(&mut tree_request, 1, now_secs());
    let tree = call(&engine, DECK_TREE, tree_request)
        .expect("the adapter answers DecksService.DeckTree (7,4)");
    let children: Vec<(i64, String)> = wire::repeated(&tree, 3)
        .into_iter()
        .map(|child| (wire::signed(&child, 1), text(wire::bytes(&child, 2))))
        .collect();
    let names: Vec<&str> = children.iter().map(|(_, name)| name.as_str()).collect();
    assert_eq!(names, vec!["Default", SECOND_DECK], "the root's children");
    let second = children[1].0;

    // (7,22): the empty deck queues nothing, and the default deck queues the synthetic card.
    call(&engine, SET_CURRENT_DECK, deck_id(second))
        .expect("the adapter answers DecksService.SetCurrentDeck (7,22)");
    assert_eq!(queued(&engine).len(), 0, "the empty deck queues no card");
    call(&engine, SET_CURRENT_DECK, deck_id(DEFAULT_DECK))
        .expect("the adapter chooses the default deck again");
    let cards = queued(&engine);
    let head = cards.first().expect("the default deck queues its card");
    assert_eq!(
        wire::signed(&wire::bytes(head, 1), 1),
        synthetic.card_id,
        "the queued card is the synthetic card"
    );

    // (13,24): the queued card's four states, each described in the engine's words.
    let described = call(&engine, DESCRIBE_NEXT_STATES, wire::bytes(head, 3))
        .expect("the adapter answers SchedulerService.DescribeNextStates (13,24)");
    let described: Vec<String> = wire::repeated(&described, 1)
        .into_iter()
        .map(text)
        .collect();
    assert_eq!(described.len(), 4, "one interval per rating: {described:?}");
    assert_eq!(
        described[..3],
        LEARNING_INTERVALS,
        "a new card's learning intervals"
    );

    let listed: BTreeSet<(u32, u32, &str)> = ALLOW_LIST
        .iter()
        .map(|entry| (entry.service, entry.method, entry.name))
        .collect();
    assert_eq!(listed.len(), ALLOW_LIST.len(), "no pair is listed twice");
    assert_eq!(
        listed,
        EXPECTED.into_iter().collect::<BTreeSet<_>>(),
        "the allow-list"
    );
}
