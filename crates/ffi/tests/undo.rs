//! The review's undo through the native adapter (SPEC-515 A2, A3): the offer of the review's own
//! last answer, bury or flag, with the words the dialog shows; the undo the owner's confirmation
//! runs, after which the card is back as it was; and the refusals that say why.
//!
//! Each test builds the review fixture the app's review tests open, opens it through a fresh
//! adapter, chooses its deck and acts on the queue's first card, the text card, whose id the
//! fixture's builder fixes; the card after it is the image card. What an undo restored is read back
//! through the adapter's queue, and through the engine's own API once the adapter has let the
//! collection go. The dialog's words are written here from SPEC-371 R10 and SPEC-383 R11, never
//! read from the adapter.

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
use deck_streak_ffi::engine::{Engine, PressedGrade};
use deck_streak_ffi::review_undo::{UndoDone, UndoKind, UndoOffered};
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
/// The engine's stored queue for a new card (`CardQueue::New`).
const NEW: i64 = 0;
/// The engine's numbers for no flag, the red flag and the orange one at the pinned rev.
const NO_FLAG: u32 = 0;
const RED_FLAG: u32 = 1;
const ORANGE_FLAG: u32 = 2;
/// The time a test's press took, in milliseconds.
const TAKEN: u32 = 1_000;

/// The dialog's titles, its Keep button and its confirmations (SPEC-371 R10, SPEC-383 R10-R11).
const ANSWER_TITLE: &str = "Undo this answer?";
const BURY_TITLE: &str = "Undo this bury?";
const FLAG_TITLE: &str = "Undo this flag?";
const KEEP: &str = "Keep it";
const UNDO_ANSWER: &str = "Undo answer";
const UNDO_BURY: &str = "Undo bury";
const UNDO_FLAG: &str = "Undo flag";
/// The text card's line, its answer and the state it goes back to, as the dialog names them.
const TEXT_CARD_LINE: &str = "Card: a text card";
const GOOD_LINE: &str = "Your answer: Good";
const BACK_TO_NEW: &str = "Goes back to: New, in today's queue";
/// What each undo changes, as the dialog says it.
const ANSWER_UNSYNCED: &str = "This answer has not synced yet. Undoing it removes it from your \
                               review history; nothing else changes.";
const BURY_UNSYNCED: &str = "This bury has not synced yet. Undoing it puts the card back in \
                             today's queue; nothing else changes.";
const FLAG_ADDED: &str = "You added a red flag. Undoing removes it; nothing else changes.";
const FLAG_REMOVED: &str = "You removed the red flag. Undoing puts it back; nothing else changes.";
const FLAG_REPLACED: &str = "You replaced this card's flag with a red flag. Undoing puts its \
                             earlier flag back; nothing else changes.";
/// Why an answer can no longer be undone (SPEC-371 R10).
const ANSWER_GONE: &str = "This answer can no longer be undone: something changed after it.";

/// The review fixture, opened through a fresh adapter with its deck chosen.
struct Opened {
    engine: Arc<Engine>,
    fixture: review::Review,
}

/// The queue's first card as the adapter shows it: its id, its flag and the states it is shown
/// with.
struct Shown {
    card: i64,
    flag: u32,
    states: Vec<u8>,
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

/// Builds the review fixture in a scratch directory of the test's own, sets the text card's flag
/// to `flag` through the engine's own API, opens it through the adapter and chooses its deck, as
/// the app does.
fn opened(test: &str, flag: u32) -> Opened {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_nanos();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ffi-undo")
        .join(format!("{test}-{}-{stamp}", std::process::id()));
    let fixture = review::build(&dir).expect("the builder writes the review fixture");
    assert_eq!(
        [fixture.cards.text, fixture.cards.image],
        [TEXT_CARD, IMAGE_CARD],
        "the review fixture's text and image cards carry their fixed ids"
    );
    let col = CollectionBuilder::new(&fixture.collection)
        .build()
        .expect("the engine opens the review fixture");
    col.storage
        .db()
        .execute(
            "update cards set flags = ? where id = ?",
            [i64::from(flag), TEXT_CARD],
        )
        .expect("the engine stores the text card's flag");
    col.close(None)
        .expect("the engine closes the review fixture");
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

/// The queue's first card, as the engine answers it through the adapter: `QueuedCard.card`'s id
/// (field 1) and flags (field 17), and `QueuedCard.states` (field 3).
fn head(engine: &Engine) -> Shown {
    let mut request = Vec::new();
    wire::put_varint_field(&mut request, 1, 1);
    let (service, method) = GET_QUEUED_CARDS;
    let reply = engine
        .run(service, method, request)
        .expect("the queue is on the allow-list");
    let first = wire::repeated(&reply, 1)
        .into_iter()
        .next()
        .expect("the review fixture's deck queues a card");
    let card = wire::bytes(&first, 1);
    Shown {
        card: wire::signed(&card, 1),
        flag: u32::try_from(wire::varint(&card, 17)).expect("a flag is a small number"),
        states: wire::bytes(&first, 3),
    }
}

/// The adapter's offer of the review's last step, or why there is none.
fn offer_of(engine: &Engine) -> UndoOffered {
    engine
        .undo_offer()
        .expect("the adapter answers the undo's offer")
}

/// The owner's confirmation of the offer of `card` at `step`.
fn confirm(engine: &Engine, card: i64, step: u32) -> UndoDone {
    engine
        .undo(card, step)
        .expect("the adapter answers the confirmation")
}

/// An offer as one comparable reading: its card, its kind, its title, its lines, its Keep and its
/// confirmation, and the sentence beside it.
type Reading = (i64, UndoKind, String, Vec<String>, String, String, String);

/// `offered`'s reading, or `None` when nothing was offered.
fn reading(offered: &UndoOffered) -> Option<Reading> {
    offered.offer.as_ref().map(|offer| {
        (
            offer.card,
            offer.kind,
            offer.title.clone(),
            offer.lines.clone(),
            offer.keep.clone(),
            offer.confirm.clone(),
            offered.said.clone(),
        )
    })
}

/// The reading an offer of `kind` on `card` is expected to give, from the words above.
fn expected(card: i64, kind: UndoKind, title: &str, lines: &[&str], confirm: &str) -> Reading {
    (
        card,
        kind,
        title.to_owned(),
        lines.iter().map(|line| (*line).to_owned()).collect(),
        KEEP.to_owned(),
        confirm.to_owned(),
        String::new(),
    )
}

/// The review-log rows `card` holds, read through the engine's own API from the collection the
/// adapter has let go.
fn reviews_of(collection: &Path, card: i64) -> i64 {
    stored(collection, "select count() from revlog where cid = ?", card)
}

/// The stored queue of `card`, read the same way.
fn queue_of(collection: &Path, card: i64) -> i64 {
    stored(collection, "select queue from cards where id = ?", card)
}

/// One integer `sql` selects for `card`, read through the engine's own API.
fn stored(collection: &Path, sql: &str, card: i64) -> i64 {
    let col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the collection the adapter let go");
    let value = col
        .storage
        .db()
        .query_row(sql, [card], |row| row.get::<_, i64>(0))
        .expect("the engine answers the read");
    col.close(None).expect("the engine closes the collection");
    value
}

#[test]
fn an_answer_is_offered_by_its_card_answer_and_state_then_undone() {
    let Opened { engine, fixture } = opened("answer", NO_FLAG);
    let shown = head(&engine);
    assert_eq!(shown.card, TEXT_CARD, "the queue shows the text card first");
    engine
        .answer(TEXT_CARD, PressedGrade::Good, shown.states, TAKEN)
        .expect("the adapter records the press");
    assert_eq!(
        head(&engine).card,
        IMAGE_CARD,
        "after the answer the queue shows the next card"
    );

    let offered = offer_of(&engine);
    assert_eq!(
        reading(&offered),
        Some(expected(
            TEXT_CARD,
            UndoKind::Answer,
            ANSWER_TITLE,
            &[TEXT_CARD_LINE, GOOD_LINE, BACK_TO_NEW, ANSWER_UNSYNCED],
            UNDO_ANSWER,
        )),
        "the offer names the card as text, the answer and the state it goes back to"
    );
    let offer = offered.offer.expect("the answer is offered");
    let done = confirm(&engine, offer.card, offer.step);
    assert_eq!(
        (done.undone, done.said.as_str()),
        (true, ""),
        "the confirmation undoes the answer"
    );
    assert_eq!(
        head(&engine).card,
        TEXT_CARD,
        "the undone card is queued first again"
    );
    let after = offer_of(&engine);
    assert_eq!(
        (reading(&after), after.said.as_str()),
        (None, ANSWER_GONE),
        "a successful undo empties the slot"
    );
    drop(engine);
    assert_eq!(
        reviews_of(&fixture.collection, TEXT_CARD),
        0,
        "the undo removed the answer's review"
    );
}

#[test]
fn a_bury_is_offered_with_its_line_then_undone() {
    let Opened { engine, fixture } = opened("bury", NO_FLAG);
    let shown = head(&engine);
    assert_eq!(shown.card, TEXT_CARD, "the queue shows the text card first");
    engine
        .bury(TEXT_CARD, shown.states)
        .expect("the adapter buries the shown card");
    assert_eq!(
        head(&engine).card,
        IMAGE_CARD,
        "after the bury the queue shows the next card"
    );

    let offered = offer_of(&engine);
    assert_eq!(
        reading(&offered),
        Some(expected(
            TEXT_CARD,
            UndoKind::Bury,
            BURY_TITLE,
            &[TEXT_CARD_LINE, BACK_TO_NEW, BURY_UNSYNCED],
            UNDO_BURY,
        )),
        "the offer names the card as text, the state it goes back to and the bury's line"
    );
    let offer = offered.offer.expect("the bury is offered");
    let done = confirm(&engine, offer.card, offer.step);
    assert_eq!(
        (done.undone, done.said.as_str()),
        (true, ""),
        "the confirmation undoes the bury"
    );
    assert_eq!(
        head(&engine).card,
        TEXT_CARD,
        "the card is queued first again"
    );
    drop(engine);
    assert_eq!(
        queue_of(&fixture.collection, TEXT_CARD),
        NEW,
        "the card is back in the new queue"
    );
}

#[test]
fn a_flag_is_offered_with_its_line_then_undone() {
    let mut seen = Vec::new();

    // A red flag added, then undone.
    let Opened { engine, .. } = opened("flag-added", NO_FLAG);
    let added = engine
        .flag(TEXT_CARD, NO_FLAG)
        .expect("the adapter flags the shown card");
    let offered = offer_of(&engine);
    let offer = offered.offer.clone().expect("the flag is offered");
    let done = confirm(&engine, offer.card, offer.step);
    seen.push((added, reading(&offered), done.undone, head(&engine).flag));

    // A red flag taken off, then undone.
    let red = engine
        .flag(TEXT_CARD, NO_FLAG)
        .expect("the adapter flags the card red");
    let removed = engine
        .flag(TEXT_CARD, red)
        .expect("the adapter takes the red flag off");
    let offered = offer_of(&engine);
    let offer = offered.offer.clone().expect("the removal is offered");
    let done = confirm(&engine, offer.card, offer.step);
    seen.push((removed, reading(&offered), done.undone, head(&engine).flag));

    // An orange flag replaced by red, then undone.
    let Opened { engine, .. } = opened("flag-replaced", ORANGE_FLAG);
    assert_eq!(
        head(&engine).flag,
        ORANGE_FLAG,
        "the text card carries the orange flag"
    );
    let replaced = engine
        .flag(TEXT_CARD, ORANGE_FLAG)
        .expect("the adapter sets red over the orange flag");
    let offered = offer_of(&engine);
    let offer = offered.offer.clone().expect("the replacement is offered");
    let done = confirm(&engine, offer.card, offer.step);
    seen.push((replaced, reading(&offered), done.undone, head(&engine).flag));

    let flagged = |line: &str| {
        Some(expected(
            TEXT_CARD,
            UndoKind::Flag,
            FLAG_TITLE,
            &[TEXT_CARD_LINE, line],
            UNDO_FLAG,
        ))
    };
    assert_eq!(
        seen,
        vec![
            (RED_FLAG, flagged(FLAG_ADDED), true, NO_FLAG),
            (NO_FLAG, flagged(FLAG_REMOVED), true, RED_FLAG),
            (RED_FLAG, flagged(FLAG_REPLACED), true, ORANGE_FLAG),
        ],
        "each flag's offer names the card and what its undo does, and the undo puts the earlier \
         flag back"
    );
}

#[test]
fn a_confirmation_after_a_later_change_is_refused_and_says_why() {
    let Opened { engine, fixture } = opened("later-change", NO_FLAG);
    let shown = head(&engine);
    engine
        .answer(TEXT_CARD, PressedGrade::Good, shown.states, TAKEN)
        .expect("the adapter records the press");
    let offer = offer_of(&engine).offer.expect("the answer is offered");

    // A later change: the image card, shown now, is flagged red before the confirmation.
    engine
        .flag(IMAGE_CARD, NO_FLAG)
        .expect("the adapter flags the image card");
    let refused = confirm(&engine, offer.card, offer.step);
    let now = head(&engine);
    let later = offer_of(&engine)
        .offer
        .map(|later| (later.card, later.kind));
    assert_eq!(
        (
            (refused.undone, refused.said.as_str()),
            later,
            (now.card, now.flag)
        ),
        (
            (false, ANSWER_GONE),
            Some((IMAGE_CARD, UndoKind::Flag)),
            (IMAGE_CARD, RED_FLAG)
        ),
        "a confirmation of the answer after a later flag is refused and says why, and the flag \
         is what the review offers now"
    );
    drop(engine);
    assert_eq!(
        reviews_of(&fixture.collection, TEXT_CARD),
        1,
        "the answer stands"
    );
}

#[test]
fn with_nothing_kept_no_undo_is_offered() {
    let Opened { engine, fixture } = opened("nothing-kept", NO_FLAG);
    let offered = offer_of(&engine);
    let refused = confirm(&engine, TEXT_CARD, 0);
    assert_eq!(
        (
            reading(&offered),
            offered.said.as_str(),
            refused.undone,
            refused.said.as_str()
        ),
        (None, ANSWER_GONE, false, ANSWER_GONE),
        "with nothing kept, nothing is offered or undone, and each says why"
    );
    assert_eq!(head(&engine).card, TEXT_CARD, "the queue is as it was");
    drop(engine);
    assert_eq!(
        reviews_of(&fixture.collection, TEXT_CARD),
        0,
        "nothing was written"
    );
}
