//! SPEC-345 A13 and A14 (R8; ADR-356 D5): an owner's gesture runs its one write on its one
//! target, and a request that names anything else is refused before the engine sees it.
//!
//! Each test opens a synthetic collection the engine builds, on a native dispatcher, and reads
//! each card's scheduling row through the core's fixed snapshot read, so what the engine wrote is
//! observed rather than inferred from a reply.

#![allow(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "a failed fixture should fail its test, and an enumerating test prints what it examined"
)]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::CollectionBuilder;
use anki_proto::cards::RemoveCardsRequest;
use anki_proto::deck_config::DeckConfigId;
use anki_proto::notes::RemoveNotesRequest;
use anki_proto::notetypes::ChangeNotetypeRequest;
use anki_proto::scheduler::card_answer::Rating;
use anki_proto::scheduler::{
    CardAnswer, GetQueuedCardsRequest, QueuedCards, ScheduleCardsAsNewRequest, SetDueDateRequest,
};
use deck_streak_engine_core::answer::{Grade, OwnerAnswer};
use deck_streak_engine_core::dispatch::{Dispatcher, Read};
use deck_streak_engine_core::gesture::{GestureRefusal, OwnerGesture, Target};
use deck_streak_engine_core::table::{EXEMPT, ExemptWrite, Transport};
use deck_streak_engine_core::undo_answer::Recorded;
use prost::Message;
use serde_json::Value;

const OPEN_COLLECTION: (u32, u32) = (3, 0);
const GET_QUEUED_CARDS: (u32, u32) = (13, 3);
/// The snapshot's columns: `[id, queue, type, due, ivl, reps, lapses]`.
const QUEUE: usize = 1;
const TYPE: usize = 2;
/// The default preset every collection has.
const DEFAULT_PRESET: i64 = 1;
/// A preset no synthetic collection holds.
const ANOTHER_PRESET: i64 = 2;
/// Bytes that are no message of any exempt write: a length-delimited field 1 whose length runs
/// past the end.
const NO_MESSAGE: [u8; 3] = [0x0a, 0x05, 0x01];

/// What the engine's own API reads from a synthetic collection before a dispatcher opens it: each
/// card's note, and a note type change from Basic to Basic (and reversed card) naming no note.
struct Fixture {
    cards: [i64; 2],
    notes: [i64; 2],
    change: ChangeNotetypeRequest,
}

fn fixture(synthetic: &support::Synthetic) -> Fixture {
    let mut col = CollectionBuilder::new(&synthetic.collection)
        .build()
        .expect("the engine opens the synthetic collection");
    let notes = synthetic.cards.map(|card| {
        col.storage
            .db()
            .query_row("select nid from cards where id = ?", [card], |row| {
                row.get(0)
            })
            .expect("each card has its note")
    });
    let basic = col
        .get_notetype_by_name("Basic")
        .expect("the note types are read")
        .expect("the engine creates its stock Basic note type");
    let reversed = col
        .get_notetype_by_name("Basic (and reversed card)")
        .expect("the note types are read")
        .expect("the engine creates its stock reversed note type");
    let mut input = col
        .notetype_change_info(basic.id, reversed.id)
        .expect("the engine maps Basic onto its reversed type")
        .input;
    input.note_ids = Vec::new();
    col.close(None).expect("the engine closes the collection");
    Fixture {
        cards: synthetic.cards,
        notes,
        change: input.into(),
    }
}

fn opened(synthetic: &support::Synthetic) -> Dispatcher {
    let dispatcher =
        Dispatcher::start(Transport::Native, &[]).expect("the engine starts from the default init");
    let (service, method) = OPEN_COLLECTION;
    dispatcher
        .run(service, method, &support::open_request(synthetic))
        .expect("the native dispatcher opens the collection");
    dispatcher
}

fn snapshot(dispatcher: &Dispatcher, card: i64) -> Value {
    let reply = dispatcher
        .read(Read::CardSnapshot(card))
        .expect("the snapshot reads");
    serde_json::from_slice::<Value>(&reply)
        .expect("the engine replies with JSON rows")
        .pointer("/0")
        .cloned()
        .unwrap_or(Value::Null)
}

fn note_count(dispatcher: &Dispatcher) -> Value {
    let reply = dispatcher
        .read(Read::NoteCount)
        .expect("the note count reads");
    serde_json::from_slice::<Value>(&reply)
        .expect("the engine replies with JSON rows")
        .pointer("/0/0")
        .cloned()
        .unwrap_or(Value::Null)
}

fn now_millis() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock reads after the epoch")
        .as_millis();
    i64::try_from(millis).expect("the clock's milliseconds fit an i64")
}

/// Answers the card at the head of the queue Good, through an owner's answer (SPEC-365 R3), and
/// returns its id.
fn answer_next(dispatcher: &Dispatcher) -> i64 {
    let request = GetQueuedCardsRequest {
        fetch_limit: 1,
        intraday_learning_only: false,
    };
    let (service, method) = GET_QUEUED_CARDS;
    let queued = dispatcher
        .run(service, method, &request.encode_to_vec())
        .expect("the queue is admitted");
    let first = QueuedCards::decode(queued.as_slice())
        .expect("the queue decodes")
        .cards
        .into_iter()
        .next()
        .expect("a card is queued");
    let card = first.card.expect("a queued card carries its card").id;
    let states = first.states.expect("a queued card carries its states");
    let answer = CardAnswer {
        card_id: card,
        current_state: states.current,
        new_state: states.good,
        rating: Rating::Good as i32,
        answered_at_millis: now_millis(),
        milliseconds_taken: 1000,
    };
    dispatcher
        .run_answer(
            OwnerAnswer::from_press(card, Grade::Good),
            &answer.encode_to_vec(),
        )
        .expect("the owner's answer reaches the engine");
    card
}

fn tap(
    dispatcher: &Dispatcher,
    write: ExemptWrite,
    target: Target,
    request: &[u8],
) -> Result<Vec<u8>, GestureRefusal> {
    OwnerGesture::from_tap(write, target)
        .and_then(|gesture| dispatcher.run_exempt(gesture, request))
}

fn forget(cards: Vec<i64>) -> Vec<u8> {
    ScheduleCardsAsNewRequest {
        card_ids: cards,
        ..ScheduleCardsAsNewRequest::default()
    }
    .encode_to_vec()
}

fn set_due_date(cards: Vec<i64>) -> Vec<u8> {
    SetDueDateRequest {
        card_ids: cards,
        days: String::from("1"),
        config_key: None,
    }
    .encode_to_vec()
}

fn delete_preset(preset: i64) -> Vec<u8> {
    DeckConfigId { dcid: preset }.encode_to_vec()
}

fn change_note_type(fixture: &Fixture, notes: Vec<i64>) -> Vec<u8> {
    ChangeNotetypeRequest {
        note_ids: notes,
        ..fixture.change.clone()
    }
    .encode_to_vec()
}

fn delete_card(cards: Vec<i64>) -> Vec<u8> {
    RemoveCardsRequest { card_ids: cards }.encode_to_vec()
}

/// An undo's record naming the review row `review`. The fixture holds no answer, so no record names
/// a row it holds; `undo_answer.rs` holds the undo of a recorded answer (SPEC-371 A4 to A13).
fn undo(review: i64) -> Vec<u8> {
    Recorded {
        status: None,
        review,
        ..Recorded::default()
    }
    .encode_to_vec()
}

fn delete_note(notes: Vec<i64>, cards: Vec<i64>) -> Vec<u8> {
    RemoveNotesRequest {
        note_ids: notes,
        card_ids: cards,
    }
    .encode_to_vec()
}

/// The one target each write's gesture names in the fixture: the first card, its note, or the
/// default preset.
fn target(write: ExemptWrite, fixture: &Fixture) -> Target {
    match write {
        ExemptWrite::Forget
        | ExemptWrite::SetDueDate
        | ExemptWrite::DeleteCard
        | ExemptWrite::Undo => Target::Card(fixture.cards[0]),
        ExemptWrite::ChangeNoteType | ExemptWrite::DeleteNote => Target::Note(fixture.notes[0]),
        ExemptWrite::DeletePreset => Target::Preset(DEFAULT_PRESET),
        ExemptWrite::OneWaySync => Target::Collection,
    }
}

/// The request that names exactly the gesture's target.
fn only_the_target(write: ExemptWrite, fixture: &Fixture) -> Vec<u8> {
    let [card, _] = fixture.cards;
    let [note, _] = fixture.notes;
    match write {
        ExemptWrite::Forget => forget(vec![card]),
        ExemptWrite::SetDueDate => set_due_date(vec![card]),
        ExemptWrite::DeletePreset => delete_preset(DEFAULT_PRESET),
        ExemptWrite::ChangeNoteType => change_note_type(fixture, vec![note]),
        ExemptWrite::DeleteCard => delete_card(vec![card]),
        ExemptWrite::DeleteNote => delete_note(vec![note], Vec::new()),
        ExemptWrite::OneWaySync => Vec::new(),
        ExemptWrite::Undo => undo(0),
    }
}

/// Every request a write's gesture must refuse: none of the target, another, the target and
/// another, and for `RemoveNotes` any `card_ids`, each named.
fn naming_more(write: ExemptWrite, fixture: &Fixture) -> Vec<(&'static str, Vec<u8>)> {
    let [card, other] = fixture.cards;
    let [note, other_note] = fixture.notes;
    match write {
        ExemptWrite::Forget => vec![
            ("no card", forget(Vec::new())),
            ("another card", forget(vec![other])),
            ("the card and another", forget(vec![card, other])),
        ],
        ExemptWrite::SetDueDate => vec![
            ("no card", set_due_date(Vec::new())),
            ("another card", set_due_date(vec![other])),
            ("the card and another", set_due_date(vec![card, other])),
        ],
        ExemptWrite::DeletePreset => vec![
            ("no preset", delete_preset(0)),
            ("another preset", delete_preset(ANOTHER_PRESET)),
            (
                "the preset and another",
                [delete_preset(DEFAULT_PRESET), delete_preset(ANOTHER_PRESET)].concat(),
            ),
        ],
        ExemptWrite::ChangeNoteType => vec![
            ("no note", change_note_type(fixture, Vec::new())),
            ("another note", change_note_type(fixture, vec![other_note])),
            (
                "the note and another",
                change_note_type(fixture, vec![note, other_note]),
            ),
        ],
        ExemptWrite::DeleteCard => vec![
            ("no card", delete_card(Vec::new())),
            ("another card", delete_card(vec![other])),
            ("the card and another", delete_card(vec![card, other])),
        ],
        ExemptWrite::DeleteNote => vec![
            ("no note", delete_note(Vec::new(), Vec::new())),
            ("another note", delete_note(vec![other_note], Vec::new())),
            (
                "the note and another",
                delete_note(vec![note, other_note], Vec::new()),
            ),
            (
                "no note, and another card",
                delete_note(Vec::new(), vec![other]),
            ),
            (
                "the note, and its card",
                delete_note(vec![note], vec![card]),
            ),
        ],
        ExemptWrite::OneWaySync => Vec::new(),
        ExemptWrite::Undo => vec![("a review row the collection lacks", undo(1))],
    }
}

/// What became of a tap, as a test reads it: the engine ran it, the engine refused it, or the
/// core refused it before the engine saw it.
fn outcome(result: Result<Vec<u8>, GestureRefusal>) -> String {
    match result {
        Ok(_) => String::from("ran"),
        Err(GestureRefusal::Engine { .. }) => String::from("the engine refused it"),
        Err(refusal) => format!("refused before the engine: {refusal}"),
    }
}

#[test]
fn a_forget_gesture_resets_its_one_card_and_no_other() {
    let synthetic = support::synthetic("forget-one-card");
    let dispatcher = opened(&synthetic);
    let card = answer_next(&dispatcher);
    let other = answer_next(&dispatcher);
    assert_ne!(
        card, other,
        "the two answers took the collection's two cards"
    );
    let answered = (snapshot(&dispatcher, card), snapshot(&dispatcher, other));
    let ran = tap(
        &dispatcher,
        ExemptWrite::Forget,
        Target::Card(card),
        &forget(vec![card]),
    )
    .map(|_| ());
    let forgotten = snapshot(&dispatcher, card);
    assert_eq!(
        (
            ran,
            forgotten[QUEUE].as_i64(),
            forgotten[TYPE].as_i64(),
            snapshot(&dispatcher, other)
        ),
        (Ok(()), Some(0), Some(0), answered.1.clone()),
        "Forget returned its card to the new queue and left the other as answered: {answered:?}"
    );
    assert_eq!(
        (answered.0[QUEUE].as_i64(), answered.1[QUEUE].as_i64()),
        (Some(1), Some(1)),
        "both cards were in learning before the gesture"
    );
}

#[test]
fn a_request_naming_more_than_its_gestures_target_is_refused() {
    let writes = EXEMPT.map(|row| row.write);
    let ran: Vec<(ExemptWrite, String)> = writes
        .iter()
        .map(|&write| {
            let synthetic = support::synthetic(&format!("only-the-target-{write:?}"));
            let fixture = fixture(&synthetic);
            let dispatcher = opened(&synthetic);
            let target = target(write, &fixture);
            let result = tap(
                &dispatcher,
                write,
                target,
                &only_the_target(write, &fixture),
            );
            // each collection numbers its own cards, so the target is named by its role
            (
                write,
                outcome(result).replace(&format!("{target:?}"), "its target"),
            )
        })
        .collect();
    let engine_refuses_its_default_preset = String::from("the engine refused it");
    assert_eq!(
        ran,
        vec![
            (ExemptWrite::Forget, String::from("ran")),
            (ExemptWrite::SetDueDate, String::from("ran")),
            (ExemptWrite::DeletePreset, engine_refuses_its_default_preset),
            (ExemptWrite::ChangeNoteType, String::from("ran")),
            (ExemptWrite::DeleteCard, String::from("ran")),
            (ExemptWrite::DeleteNote, String::from("ran")),
            (
                ExemptWrite::OneWaySync,
                String::from(
                    "refused before the engine: the one-way sync runs only through the full-sync \
                     choice's write",
                ),
            ),
            // the fixture holds no answer, so an undo's record names no row it holds (SPEC-371 R5)
            (
                ExemptWrite::Undo,
                String::from(
                    "refused before the engine: the Undo request names other than its target"
                ),
            ),
        ],
        "a request naming exactly its gesture's target passes the check and reaches the engine"
    );

    let synthetic = support::synthetic("naming-more");
    let fixture = fixture(&synthetic);
    let dispatcher = opened(&synthetic);
    let before = (
        fixture.cards.map(|card| snapshot(&dispatcher, card)),
        note_count(&dispatcher),
    );
    let mut refused = Vec::new();
    let mut expected = Vec::new();
    for write in writes {
        let target = target(write, &fixture);
        let mut requests = naming_more(write, &fixture);
        requests.push(("no message", NO_MESSAGE.to_vec()));
        for (named, request) in requests {
            refused.push((write, named, tap(&dispatcher, write, target, &request)));
            expected.push((
                write,
                named,
                Err(if write == ExemptWrite::OneWaySync {
                    GestureRefusal::NeedsTheChoice
                } else if named == "no message" {
                    GestureRefusal::Undecodable { write }
                } else {
                    GestureRefusal::NotTheTarget { write, target }
                }),
            ));
        }
    }
    let after = (
        fixture.cards.map(|card| snapshot(&dispatcher, card)),
        note_count(&dispatcher),
    );
    assert_eq!(
        refused, expected,
        "each request naming other than its gesture's target is refused before the engine"
    );
    assert_eq!(
        after, before,
        "no refused request changed a card's row or the note count"
    );
    support::examined("refused request(s)", refused);
}

#[test]
fn run_exempt_refuses_the_one_way_sync() {
    let synthetic = support::synthetic("one-way-exempt");
    let fixture = fixture(&synthetic);
    let dispatcher = opened(&synthetic);
    let before = (
        fixture.cards.map(|card| snapshot(&dispatcher, card)),
        note_count(&dispatcher),
    );
    // Whatever the bytes, the one-way gesture never reaches its request through `run_exempt`: the
    // last is the engine's own one-way request, which the core alone may build (SPEC-364 R3).
    let requests = [
        ("no message", NO_MESSAGE.to_vec()),
        ("an empty request", Vec::new()),
        (
            "an upload request naming a loopback endpoint",
            anki_proto::sync::FullUploadOrDownloadRequest {
                auth: Some(anki_proto::sync::SyncAuth {
                    hkey: String::from("planted-host-key"),
                    endpoint: Some(String::from("http://127.0.0.1:1/")),
                    io_timeout_secs: None,
                }),
                upload: true,
                server_usn: None,
            }
            .encode_to_vec(),
        ),
    ];
    let refused: Vec<(&str, Result<Vec<u8>, GestureRefusal>)> = requests
        .iter()
        .map(|(named, request)| {
            (
                *named,
                tap(
                    &dispatcher,
                    ExemptWrite::OneWaySync,
                    Target::Collection,
                    request,
                ),
            )
        })
        .collect();
    let expected: Vec<(&str, Result<Vec<u8>, GestureRefusal>)> = requests
        .iter()
        .map(|(named, _)| (*named, Err(GestureRefusal::NeedsTheChoice)))
        .collect();
    assert_eq!(
        refused, expected,
        "run_exempt refuses the one-way gesture before it decodes its request"
    );
    let after = (
        fixture.cards.map(|card| snapshot(&dispatcher, card)),
        note_count(&dispatcher),
    );
    assert_eq!(
        after, before,
        "no refused request changed a card or the note count"
    );
    support::examined("one-way request(s)", refused);
}
