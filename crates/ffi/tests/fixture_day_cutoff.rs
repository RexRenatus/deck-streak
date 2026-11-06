//! The review fixture's day (SPEC-615 A1 to A3 and A5).
//!
//! The review fixture is a version-2 collection, whose next day starts at its rollover hour on the
//! reading host's own clock. The first test sets the hour whose day ends soonest, the condition a
//! fixture with no hour of its own meets in the last minutes before the engine's default hour, and
//! holds the day pin to a day that ends about twelve hours after a fresh reading. The second holds
//! the pinned fixture's text card to the four interval words SPEC-348's A1 pins, through the
//! adapter, in the app's order of calls. The third reads the fixture's writer and holds it to one
//! pin, on the review collection, between the review and parity builds.
//!
//! The fourth plants a directory that an earlier run left under this process id and holds the
//! scratch helper to removing it before a test writes.

// A failed setup step fails the test, as clippy.toml allows inside a test function.
#![allow(clippy::expect_used)]

#[expect(
    dead_code,
    reason = "this test reads the wire helpers alone, never the synthetic collection"
)]
mod support;

#[path = "support/review.rs"]
mod review;

#[path = "support/day_cutoff.rs"]
mod day_cutoff;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anki::collection::{Collection, CollectionBuilder};
use deck_streak_ffi::engine::{Engine, EngineRefusal};
use support::wire;

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
/// The review fixture's card ids, in the order its builder adds the cards: text, image, sound and
/// speech. The engine seeds a review interval's fuzz from the card's id, so the text card's words
/// hold on every run only because the builder fixes these ids.
const FIXTURE_CARD_IDS: [i64; 4] = [1_000_007, 1_000_008, 1_000_009, 1_000_010];
/// SPEC-348's A1 words for a new card's Again, Hard, Good and Easy intervals on the default preset.
const NEW_CARD_INTERVALS: [&str; 4] = ["<1m", "<6m", "<10m", "4d"];
/// The review fixture's writer, as the census reads it.
const WRITER: &str = include_str!("../examples/review-fixture.rs");

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

/// A scratch directory for one test, named by the test and this process, and absent before the
/// test writes it.
fn scratch(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-fixture-day-cutoff")
        .join(format!("{test}-{}", std::process::id()));
    assert!(
        !dir.exists(),
        "the scratch directory {} is absent before the test writes it",
        dir.display()
    );
    dir
}

/// Opens the collection, sets `hour` as its rollover, and returns the seconds from the engine's
/// reading of now to its next day cutoff. The engine caches its day per open collection, so each
/// reading opens afresh and closes before the next.
fn lead_with_rollover(collection: &Path, hour: u32) -> i64 {
    let mut col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the review fixture");
    col.set_config_json("rollover", &hour, false)
        .expect("the engine accepts a rollover hour");
    let timing = col.timing_today().expect("the engine reads its day timing");
    col.close(None).expect("the engine closes the collection");
    timing.next_day_at.0 - timing.now.0
}

/// The number one config key stores, read by `sql` as the bytes of the engine's JSON text.
fn stored(col: &Collection, sql: &str) -> u32 {
    let bytes: Vec<u8> = col
        .storage
        .db()
        .query_row(sql, [], |row| row.get(0))
        .expect("the config holds the key");
    std::str::from_utf8(&bytes)
        .expect("the engine writes its config as UTF-8 text")
        .trim()
        .parse()
        .expect("the key holds a number")
}

#[test]
fn the_review_fixture_day_ends_twelve_hours_after_its_writer() {
    let dir = scratch("a1");
    let fixture = review::build(&dir).expect("the builder writes the review fixture");

    // (a) The premise: the builder's collection is a version-2 one.
    let col = CollectionBuilder::new(&fixture.collection)
        .build()
        .expect("the engine opens the review fixture");
    assert_eq!(
        stored(&col, "select val from config where key = 'schedVer'"),
        2,
        "the review fixture is a version-2 collection"
    );
    col.close(None).expect("the engine closes the collection");

    // (b) The condition of #779: the hour whose day ends soonest, set as the rollover.
    let (soonest, soonest_lead) = (0_u32..24)
        .map(|hour| (hour, lead_with_rollover(&fixture.collection, hour)))
        .min_by_key(|&(_, lead)| lead)
        .expect("the day has twenty-four hours");
    lead_with_rollover(&fixture.collection, soonest);
    assert!(
        soonest_lead <= 3_600,
        "the day set first ends within the hour (hour {soonest}, measured lead {soonest_lead})"
    );

    // (c) The pin, then the stored hour and scheduler version in a fresh open.
    let pinned = day_cutoff::pin(&fixture.collection).expect("the pin reads the fixture's day");
    let mut col = CollectionBuilder::new(&fixture.collection)
        .build()
        .expect("the engine opens the pinned review fixture");
    assert_eq!(
        stored(&col, "select val from config where key = 'rollover'"),
        pinned.hour,
        "the collection stores the pinned hour"
    );
    assert_eq!(
        stored(&col, "select val from config where key = 'schedVer'"),
        2,
        "the pin leaves the version-2 scheduler"
    );

    // (d) That open's own reading: the day ends more than eleven hours and at most twelve and a
    // half hours ahead.
    let timing = col.timing_today().expect("the engine reads its day timing");
    col.close(None).expect("the engine closes the collection");
    let lead = timing.next_day_at.0 - timing.now.0;
    assert!(
        lead > 39_600,
        "the fixture's day ends more than eleven hours ahead (hour {}, measured lead {lead})",
        pinned.hour
    );
    assert!(
        lead <= 45_000,
        "the fixture's day ends at most twelve and a half hours ahead (hour {}, measured lead {lead})",
        pinned.hour
    );

    // (e) That reading's cutoff is the one the pin read back.
    assert_eq!(
        timing.next_day_at.0, pinned.next_day_at,
        "the fresh reading meets the pin's cutoff"
    );
}

#[test]
fn the_pinned_review_fixture_keeps_the_text_cards_four_intervals() {
    let dir = scratch("a2");
    let fixture = review::build(&dir).expect("the builder writes the review fixture");
    let cards = &fixture.cards;
    assert_eq!(
        [cards.text, cards.image, cards.sound, cards.speech],
        FIXTURE_CARD_IDS,
        "the review fixture's text, image, sound and speech cards carry their fixed ids"
    );
    let pinned = day_cutoff::pin(&fixture.collection).expect("the pin reads the fixture's day");

    // The app's calls, in its order: open the fixture, find its deck in the tree, choose it, queue
    // one card, and describe that card's four states.
    let engine = engine();
    let media = fixture.dir.join("collection.media");
    let media_db = fixture.dir.join("collection.media.db");
    let mut open = Vec::new();
    for (field, path) in [(1, &fixture.collection), (2, &media), (3, &media_db)] {
        let path = path.to_str().expect("a scratch path is UTF-8");
        wire::put_bytes(&mut open, field, path.as_bytes());
    }
    call(&engine, OPEN_COLLECTION, open).expect("the adapter opens the pinned review fixture");
    let now = u64::try_from(pinned.t).expect("the engine's instant lies after the epoch");
    let mut tree_request = Vec::new();
    wire::put_varint_field(&mut tree_request, 1, now);
    let tree = call(&engine, DECK_TREE, tree_request).expect("the adapter answers the deck tree");
    let deck = wire::repeated(&tree, 3)
        .into_iter()
        .find(|child| text(wire::bytes(child, 2)) == review::DECK)
        .map(|child| wire::signed(&child, 1))
        .expect("the review fixture's deck is a child of the root");
    call(&engine, SET_CURRENT_DECK, deck_id(deck))
        .expect("the adapter chooses the review fixture's deck");
    let queue = queued(&engine);
    let head = queue
        .first()
        .expect("the review fixture's deck queues a card");
    assert_eq!(
        wire::signed(&wire::bytes(head, 1), 1),
        FIXTURE_CARD_IDS[0],
        "the review screen shows the text card first"
    );
    let described = call(&engine, DESCRIBE_NEXT_STATES, wire::bytes(head, 3))
        .expect("the adapter describes the first card's states");
    let described: Vec<String> = wire::repeated(&described, 1)
        .into_iter()
        .map(text)
        .collect();
    assert_eq!(
        described, NEW_CARD_INTERVALS,
        "the pinned fixture's first card shows the Again, Hard, Good and Easy intervals A1 pins"
    );
}

#[test]
fn the_fixture_writer_pins_the_review_collection_once() {
    let examined = WRITER.lines().count();
    println!("examined {examined} line(s)");
    assert!(examined > 0, "the census read none of the writer's lines");
    let at = |needle: &str| -> Vec<usize> {
        WRITER
            .match_indices(needle)
            .map(|(offset, _)| offset)
            .collect()
    };
    let pins = at("day_cutoff::pin(");
    let reviews = at("review::build(");
    let parities = at("parity::build(");
    assert_eq!(pins.len(), 1, "the writer calls the day pin once: {pins:?}");
    assert_eq!(
        reviews.len(),
        1,
        "the writer builds the review collection once"
    );
    assert_eq!(
        parities.len(),
        1,
        "the writer builds the parity collection once"
    );
    assert!(
        reviews[0] < pins[0] && pins[0] < parities[0],
        "the pin follows the review build and precedes the parity build"
    );
    assert_eq!(
        at("mod day_cutoff;").len(),
        1,
        "the writer includes the day pin once"
    );
}

#[test]
fn an_earlier_runs_scratch_directory_is_cleared_before_a_test_writes() {
    // An earlier run whose process had this one's id left its directory behind, with a file in it.
    let left = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-fixture-day-cutoff")
        .join(format!("a5-{}", std::process::id()));
    std::fs::create_dir_all(&left).expect("the earlier run's directory is planted");
    std::fs::write(left.join("collection.anki2"), b"left by an earlier run")
        .expect("the earlier run's file is planted");
    assert!(
        left.is_dir(),
        "the planted directory is there before the helper runs"
    );
    let dir = scratch("a5");
    assert_eq!(
        dir, left,
        "the helper names the directory by the test and this process"
    );
    assert!(
        !dir.exists(),
        "the helper clears an earlier run's directory before the test writes it"
    );
}
