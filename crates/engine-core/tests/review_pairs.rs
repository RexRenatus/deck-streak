//! The review screen's three pairs on a native dispatcher (SPEC-348 R1, A1).
//!
//! The test builds its own collection with the engine's own API: a Basic note in the default deck
//! and one in a second deck, `Review`. It then drives a native dispatcher the way the native adapter
//! does, each request encoded as protobuf bytes and each reply decoded from them. The pairs are the
//! engine's generated dispatch numbers, read from its protobuf definitions at the pinned rev, not
//! from the core's table: the decks service has no backend methods of its own, so the tree is its
//! method 4 and the current deck its method 22, and the scheduler's backend service answers three
//! methods before the collection service's, so the next states' description is its method 24.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;
use anki_proto::decks::{DeckId, DeckTreeNode, DeckTreeRequest};
use anki_proto::generic::StringList;
use anki_proto::scheduler::{GetQueuedCardsRequest, QueuedCards};
use deck_streak_engine_core::dispatch::{Dispatcher, Refusal};
use deck_streak_engine_core::table::Transport;
use prost::Message;

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

/// The native column with the review screen's three pairs (SPEC-345 M1, SPEC-347 R1, SPEC-348 R1),
/// less `AnswerCard`, which only an owner's press records (SPEC-365 R4), and less `Undo`, which
/// only the owner's gesture runs (SPEC-371 R2), and with the review's bury and flag (SPEC-358 R1).
const NATIVE: [(u32, u32); 10] = [
    (1, 3),
    (3, 0),
    (5, 4),
    (7, 4),
    (7, 13),
    (7, 22),
    (13, 3),
    (13, 14),
    (13, 24),
    (27, 6),
];
/// The eight exempt writes, held for a gesture on every transport (SPEC-345 M8; SPEC-364 R1;
/// SPEC-371 R2).
const HELD: [(u32, u32); 8] = [
    (1, 6),
    (3, 8),
    (5, 2),
    (11, 5),
    (13, 17),
    (13, 19),
    (23, 15),
    (25, 7),
];
/// `AnswerCard`, held for an owner's press on every transport (SPEC-365 R4).
const ANSWERED: [(u32, u32); 1] = [(13, 4)];
/// The highest service and method index the census sends: past every index the engine numbers.
const LAST: u32 = 64;

/// The second deck, whose one card the review chooses.
const REVIEW_DECK: &str = "Review";
/// The two cards' ids, fixed by the test: the engine seeds a review interval's fuzz from the
/// card's id, so a new card's Easy interval is the same on every run only for the same id.
const CARD_IDS: [i64; 2] = [1_000_001, 1_000_002];
/// The engine's intervals for a new card on the default preset, Again, Hard, Good and Easy, as
/// `DescribeNextStates` words them at the pinned rev for the second card's fixed id (measured
/// once, and pinned here).
const NEW_CARD_INTERVALS: [&str; 4] = ["<1m", "<6m", "<10m", "4d"];

/// Builds a Basic note in the default deck and one in `Review`, closed, and returns the collection
/// with the two cards, their ids fixed, in that order.
fn collection(test: &str) -> support::Synthetic {
    let dir = support::scratch("engine-core-review-pairs", test);
    std::fs::create_dir_all(dir.join("collection.media")).expect("a media directory");
    let path = dir.join("collection.anki2");
    let mut col = CollectionBuilder::new(&path)
        .build()
        .expect("the engine creates the collection");
    let review = col
        .get_or_create_normal_deck(REVIEW_DECK)
        .expect("the engine creates the second deck")
        .id;
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let mut cards = [0_i64; 2];
    for (index, (card, deck)) in cards
        .iter_mut()
        .zip([anki::decks::DeckId(1), review])
        .enumerate()
    {
        let mut note = basic.new_note();
        note.set_field(0, format!("review front {index}"))
            .expect("the front is set");
        note.set_field(1, format!("review back {index}"))
            .expect("the back is set");
        col.add_note(&mut note, deck)
            .expect("the engine adds the note");
        let fixed = col
            .storage
            .db()
            .execute(
                "update cards set id = ? where nid = ?",
                [CARD_IDS[index], note.id.0],
            )
            .expect("the card's id is fixed");
        assert_eq!(fixed, 1, "the note has one card");
        *card = CARD_IDS[index];
    }
    col.close(None).expect("the engine closes the collection");
    support::Synthetic {
        dir,
        collection: path,
        cards,
    }
}

fn now_secs() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_secs();
    i64::try_from(secs).expect("the clock's seconds fit an i64")
}

fn run(
    dispatcher: &Dispatcher,
    (service, method): (u32, u32),
    input: &[u8],
) -> Result<Vec<u8>, Refusal> {
    dispatcher.run(service, method, input)
}

#[test]
fn the_review_pairs_run_natively_and_no_other_pair_joins() {
    let synthetic = collection("review-pairs");
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    run(
        &dispatcher,
        OPEN_COLLECTION,
        &support::open_request(&synthetic),
    )
    .expect("the native dispatcher opens the collection");

    // (7,4): the tree's root holds the two decks, and names the one the review chooses.
    let tree = run(
        &dispatcher,
        DECK_TREE,
        &DeckTreeRequest { now: now_secs() }.encode_to_vec(),
    )
    .expect("the native dispatcher answers DecksService.DeckTree (7,4)");
    let tree = DeckTreeNode::decode(tree.as_slice()).expect("the tree decodes");
    let names: Vec<&str> = tree
        .children
        .iter()
        .map(|child| child.name.as_str())
        .collect();
    assert_eq!(names, vec!["Default", REVIEW_DECK], "the root's children");
    let review = tree.children[1].deck_id;

    // (7,22) then (13,3): the queue after choosing the deck is that deck's card.
    run(
        &dispatcher,
        SET_CURRENT_DECK,
        &DeckId { did: review }.encode_to_vec(),
    )
    .expect("the native dispatcher answers DecksService.SetCurrentDeck (7,22)");
    let queued = run(
        &dispatcher,
        GET_QUEUED_CARDS,
        &GetQueuedCardsRequest {
            fetch_limit: 1,
            intraday_learning_only: false,
        }
        .encode_to_vec(),
    )
    .expect("the queue is admitted");
    let queued = QueuedCards::decode(queued.as_slice()).expect("the queue decodes");
    let head = queued.cards.first().expect("the chosen deck queues a card");
    assert_eq!(
        head.card.as_ref().map(|card| card.id),
        Some(synthetic.cards[1]),
        "the queued card is the chosen deck's"
    );

    // (13,24): a new card's four states, described in the engine's words.
    let states = head
        .states
        .clone()
        .expect("a queued card carries its states");
    let described = run(&dispatcher, DESCRIBE_NEXT_STATES, &states.encode_to_vec())
        .expect("the native dispatcher answers SchedulerService.DescribeNextStates (13,24)");
    let described = StringList::decode(described.as_slice()).expect("the intervals decode");
    assert_eq!(described.vals, NEW_CARD_INTERVALS, "a new card's intervals");

    // Every pair outside the native column stays refused on the native dispatcher: held for a
    // gesture when it is an exempt write, held for a press when it records a grade, not allowed
    // otherwise.
    let others: Vec<(u32, u32)> = (0..=LAST)
        .flat_map(|service| (0..=LAST).map(move |method| (service, method)))
        .filter(|pair| !NATIVE.contains(pair))
        .collect();
    let others = support::examined("pair(s) outside the native column", others);
    let (mut for_gesture, mut for_answer, mut refused, mut answered) =
        (Vec::new(), Vec::new(), 0_usize, Vec::new());
    for &(service, method) in &others {
        match dispatcher.run(service, method, &[]) {
            Err(Refusal::NeedsGesture { .. }) => for_gesture.push((service, method)),
            Err(Refusal::NeedsAnswer { .. }) => for_answer.push((service, method)),
            Err(Refusal::NotAllowed { .. }) => refused += 1,
            Ok(_) | Err(Refusal::Engine { .. }) => answered.push((service, method)),
        }
    }
    assert_eq!(
        (for_gesture, for_answer, refused, answered),
        (
            HELD.to_vec(),
            ANSWERED.to_vec(),
            4225 - NATIVE.len() - HELD.len() - ANSWERED.len(),
            Vec::new()
        ),
        "the pairs outside the native column: held for a gesture, held for a press, refused, and \
         reaching the engine"
    );
}
