//! The review's bury and flag through the native adapter (SPEC-358 R3, A5 and A6), and the red
//! flag's number the app reads from it once (SPEC-358 R3).
//!
//! Each test builds the review fixture the app's review tests open, opens it through the adapter,
//! chooses its deck and reads the queue's first card, the text card, whose id the fixture's builder
//! fixes. The adapter's `bury` and `flag` act on that card, and the test reads the result back from
//! the engine: the queue and the card's flag through the adapter, and the card's stored queue
//! through the engine's own API once the adapter has let the collection go. The fixture's four new
//! cards are queued by the positions the builder gives them by adding them in order, so the card
//! after the text card is the image card on every run.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

#[expect(
    dead_code,
    reason = "this test reads the wire helpers alone, never the synthetic collection"
)]
mod support;

#[path = "support/review.rs"]
mod review;

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;
use deck_streak_ffi::engine::{Engine, red_flag};
use support::wire;

/// `BackendCollectionService.OpenCollection`.
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// `DecksService.DeckTree`.
const DECK_TREE: (u32, u32) = (7, 4);
/// `DecksService.SetCurrentDeck`.
const SET_CURRENT_DECK: (u32, u32) = (7, 22);
/// `SchedulerService.GetQueuedCards`.
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// The review fixture's text and image cards' ids, as its builder fixes them: the text card is
/// queued first and the image card next.
const TEXT_CARD: i64 = 1_000_007;
/// The image card's id.
const IMAGE_CARD: i64 = 1_000_008;
/// The engine's stored queue for a card the user buried, at the pinned rev
/// (`CardQueue::UserBuried`). The scheduler's own bury stores -2, which the next day undoes alone.
const USER_BURIED: i64 = -3;
/// The engine's stored queue for a new card (`CardQueue::New`).
const NEW: i64 = 0;
/// The engine's number for the red flag at the pinned rev, as the core's `RED` holds it.
const RED_FLAG: u32 = 1;

/// The review fixture, opened through a fresh adapter with its deck chosen.
struct Opened {
    engine: Arc<Engine>,
    fixture: review::Review,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_secs()
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("the engine's text is UTF-8")
}

/// Builds the review fixture in a scratch directory of the test's own, opens it through the
/// adapter and chooses its deck, as the app does.
fn opened(test: &str) -> Opened {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-review-actions")
        .join(format!("{test}-{}-{stamp}", std::process::id()));
    let fixture = review::build(&dir).expect("the builder writes the review fixture");
    assert_eq!(
        [fixture.cards.text, fixture.cards.image],
        [TEXT_CARD, IMAGE_CARD],
        "the review fixture's text and image cards carry their fixed ids"
    );
    let engine = Engine::new(Vec::new()).expect("the engine starts from the default init message");
    let mut open = Vec::new();
    for (field, path) in [
        (1, fixture.collection.clone()),
        (2, fixture.dir.join("collection.media")),
        (3, fixture.dir.join("collection.media.db")),
    ] {
        let path = path.to_str().expect("a scratch path is UTF-8");
        wire::put_bytes(&mut open, field, path.as_bytes());
    }
    let (service, method) = OPEN_COLLECTION;
    engine
        .run(service, method, open)
        .expect("the adapter opens the review fixture");
    let mut tree_request = Vec::new();
    wire::put_varint_field(&mut tree_request, 1, now_secs());
    let (service, method) = DECK_TREE;
    let tree = engine
        .run(service, method, tree_request)
        .expect("the adapter answers the deck tree");
    let deck = wire::repeated(&tree, 3)
        .into_iter()
        .find(|child| text(wire::bytes(child, 2)) == review::DECK)
        .map(|child| wire::signed(&child, 1))
        .expect("the review fixture's deck is a child of the root");
    let mut chosen = Vec::new();
    wire::put_varint_field(&mut chosen, 1, u64::from_ne_bytes(deck.to_ne_bytes()));
    let (service, method) = SET_CURRENT_DECK;
    engine
        .run(service, method, chosen)
        .expect("the adapter chooses the review fixture's deck");
    Opened { engine, fixture }
}

/// The queue's first card, as the engine answers it through the adapter: its id, and its flag,
/// `Card.flags`, field 17 at the pinned rev.
fn head(engine: &Engine) -> (i64, u64) {
    let mut request = Vec::new();
    wire::put_varint_field(&mut request, 1, 1);
    let (service, method) = GET_QUEUED_CARDS;
    let reply = engine
        .run(service, method, request)
        .expect("the queue is on the allow-list");
    let queued = wire::repeated(&reply, 1);
    let first = queued
        .first()
        .expect("the review fixture's deck queues a card");
    let card = wire::bytes(first, 1);
    (wire::signed(&card, 1), wire::varint(&card, 17))
}

/// The stored queue of each of `cards`, read through the engine's own API from the collection the
/// adapter has let go.
fn stored_queues(collection: &Path, cards: [i64; 2]) -> [i64; 2] {
    let col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the collection the adapter let go");
    let queues = cards.map(|card| {
        col.storage
            .db()
            .query_row("select queue from cards where id = ?", [card], |row| {
                row.get::<_, i64>(0)
            })
            .expect("the engine stores the card")
    });
    col.close(None).expect("the engine closes the collection");
    queues
}

#[test]
fn a_bury_buries_the_card_as_the_users_bury() {
    let Opened { engine, fixture } = opened("bury");
    assert_eq!(
        head(&engine).0,
        TEXT_CARD,
        "the queue shows the text card first"
    );

    engine
        .bury(TEXT_CARD)
        .expect("the adapter buries the shown card");

    assert_eq!(
        head(&engine).0,
        IMAGE_CARD,
        "after the bury the queue moves on to the next card"
    );
    drop(engine);
    assert_eq!(
        stored_queues(&fixture.collection, [TEXT_CARD, IMAGE_CARD]),
        [USER_BURIED, NEW],
        "the shown card alone is buried, as the user's bury"
    );
}

#[test]
fn a_flag_toggles_red_and_answers_the_new_flag() {
    let Opened { engine, .. } = opened("flag");
    assert_eq!(
        head(&engine),
        (TEXT_CARD, 0),
        "the queue shows the text card first, with no flag"
    );

    let answered = engine
        .flag(TEXT_CARD, 0)
        .expect("the adapter flags the shown card");
    assert_eq!(
        head(&engine),
        (TEXT_CARD, u64::from(RED_FLAG)),
        "a card with no flag turns red in the engine"
    );
    assert_eq!(answered, RED_FLAG, "the adapter answers the new flag, red");

    let answered = engine
        .flag(TEXT_CARD, RED_FLAG)
        .expect("the adapter flags the shown card again");
    assert_eq!(
        head(&engine),
        (TEXT_CARD, 0),
        "a red card turns to no flag in the engine"
    );
    assert_eq!(answered, 0, "the adapter answers the new flag, none");
}

#[test]
fn the_red_flag_is_the_engines_flag_one() {
    // The core's `RED` holds 1 at the cut; the app compares a card's flag with this one read.
    assert_eq!(red_flag(), 1, "the adapter answers the engine's red flag");
}
