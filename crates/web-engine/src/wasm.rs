//! The module's JavaScript boundary on `wasm32` (SPEC-338 R1, ADR-348): the study calls the
//! Worker makes, and `run_method` held to the study rule's table.
//!
//! The collection lives in the origin private file system through the `SyncAccessHandle` pool VFS,
//! which only a dedicated Worker can install. The page never reaches SQL: a card's snapshot is one
//! fixed read the engine core holds, and `run_method` refuses every call outside
//! [`crate::study::STUDY_CALLS`]. Every call reaches the engine through the core's dispatcher,
//! started on the web transport, whose web column equals the study calls (SPEC-345 R5; ADR-356 D1).

use std::cell::RefCell;

use anki_proto::backend::BackendError;
use anki_proto::backend::BackendInit;
use anki_proto::card_rendering::RenderCardResponse;
use anki_proto::card_rendering::RenderExistingCardRequest;
use anki_proto::card_rendering::RenderedTemplateNode;
use anki_proto::card_rendering::rendered_template_node::Value;
use anki_proto::cards::SetFlagRequest;
use anki_proto::collection::CloseCollectionRequest;
use anki_proto::collection::OpenCollectionRequest;
use anki_proto::collection::UndoStatus;
use anki_proto::decks::DeckId;
use anki_proto::decks::DeckTreeNode;
use anki_proto::decks::DeckTreeRequest;
use anki_proto::generic::String as Text;
use anki_proto::generic::StringList;
use anki_proto::notes::AddNoteRequest;
use anki_proto::notes::AddNotesRequest;
use anki_proto::notes::AddNotesResponse;
use anki_proto::notes::Note;
use anki_proto::notetypes::NotetypeId;
use anki_proto::notetypes::NotetypeNames;
use anki_proto::scheduler::BuryOrSuspendCardsRequest;
use anki_proto::scheduler::CardAnswer;
use anki_proto::scheduler::GetQueuedCardsRequest;
use anki_proto::scheduler::QueuedCards;
use anki_proto::scheduler::SchedulingStates;
use prost::Message;
use sqlite_wasm_vfs::sahpool::OpfsSAHPoolCfgBuilder;
use sqlite_wasm_vfs::sahpool::OpfsSAHPoolUtil;
use sqlite_wasm_vfs::sahpool::install;
use wasm_bindgen::prelude::*;

use deck_streak_engine_core::dispatch::{Dispatcher, Read, Refusal};
use deck_streak_engine_core::table::Transport;

use crate::study::{
    Answer, Shown, StudyError, admit, bury_of, engine_languages, service, shown_for, toggled_red,
};
use crate::synthetic::fields;

/// The pool's directory in OPFS, one per origin (SPEC-338 R4).
const DIRECTORY: &str = "deck-streak";
/// The collection's path inside the pool.
const COLLECTION_PATH: &str = "/deck-streak/collection.anki2";
/// The default deck of a new collection, where the synthetic notes go.
const DEFAULT_DECK: i64 = 1;

thread_local! {
    static DISPATCHER: RefCell<Option<Dispatcher>> = const { RefCell::new(None) };
    static POOL: RefCell<Option<OpfsSAHPoolUtil>> = const { RefCell::new(None) };
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
    /// The card the review last showed, with its states and its flag (SPEC-350 R2).
    static SHOWN: RefCell<Option<Shown<SchedulingStates>>> = const { RefCell::new(None) };
}

fn refuse(message: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&message.to_string())
}

fn dispatcher() -> Result<Dispatcher, JsValue> {
    DISPATCHER
        .with(|d| d.borrow().clone())
        .ok_or_else(|| refuse("the engine is not initialised"))
}

fn call(service: u32, method: u32, input: &[u8]) -> Result<Vec<u8>, JsValue> {
    dispatcher()?
        .run(service, method, input)
        .map_err(|refusal| match refusal {
            Refusal::Engine { error } => match BackendError::decode(error.as_slice()) {
                Ok(err) => refuse(format!("engine error {}: {}", err.kind, err.message)),
                Err(_) => refuse("engine error (undecodable)"),
            },
            Refusal::NotAllowed { service, method } | Refusal::NeedsGesture { service, method } => {
                refuse(StudyError::CallRefused { service, method })
            }
        })
}

fn decode<M: Message + Default>(bytes: &[u8]) -> Result<M, JsValue> {
    M::decode(bytes).map_err(refuse)
}

fn query(read: Read) -> Result<serde_json::Value, JsValue> {
    let reply = dispatcher()?
        .read(read)
        .map_err(|_| refuse("the read failed"))?;
    serde_json::from_slice(&reply).map_err(refuse)
}

/// Installs the `SyncAccessHandle` pool in OPFS directory `deck-streak` as `SQLite`'s default VFS.
/// The Worker calls it only while it holds the collection's Web Lock. Returns the pool's file
/// count.
#[wasm_bindgen]
pub async fn install_storage() -> Result<u32, JsValue> {
    let cfg = OpfsSAHPoolCfgBuilder::new().directory(DIRECTORY).build();
    let pool = install::<sqlite_wasm_rs::WasmOsCallback>(&cfg, true)
        .await
        .map_err(|e| refuse(format!("storage-refused: {e}")))?;
    let count = pool.count();
    POOL.with(|p| *p.borrow_mut() = Some(pool));
    Ok(count)
}

/// The message of the last panic. A panic on `wasm32` aborts as a bare `unreachable` trap, so the
/// hook keeps its message for the Worker's `engine-failed` reply.
#[wasm_bindgen]
pub fn last_panic() -> Option<String> {
    LAST_PANIC.with(|p| p.borrow().clone())
}

/// Creates the engine's backend, as the desktop's bridge does, behind the core's dispatcher on the
/// web transport, speaking the page's languages, or English when it sends none (SPEC-350 R4).
#[wasm_bindgen(js_name = "init")]
pub fn create_backend(languages: Vec<String>) -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(|info| {
        let message = info.to_string();
        LAST_PANIC.with(|p| {
            if let Ok(mut slot) = p.try_borrow_mut() {
                *slot = Some(message);
            }
        });
    }));
    let msg = BackendInit {
        preferred_langs: engine_languages(languages),
        locale_folder_path: String::new(),
        server: false,
    };
    let dispatcher = Dispatcher::start(Transport::Web, &msg.encode_to_vec()).map_err(refuse)?;
    DISPATCHER.with(|d| *d.borrow_mut() = Some(dispatcher));
    Ok(())
}

/// Opens the collection, creating it when the pool holds none. Returns JSON:
/// `{"existed": bool, "notes": number}`, so a collection lost to eviction says so.
#[wasm_bindgen]
pub fn open() -> Result<String, JsValue> {
    let existed = POOL.with(|p| {
        p.borrow()
            .as_ref()
            .is_some_and(|pool| pool.list().iter().any(|file| file == COLLECTION_PATH))
    });
    let request = OpenCollectionRequest {
        collection_path: COLLECTION_PATH.into(),
        media_folder_path: String::new(),
        media_db_path: String::new(),
    };
    call(service::COLLECTION, 0, &request.encode_to_vec())?;
    let notes = query(Read::NoteCount)?;
    let notes = notes
        .pointer("/0/0")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(serde_json::json!({ "existed": existed, "notes": notes }).to_string())
}

/// Closes the collection.
#[wasm_bindgen]
pub fn close() -> Result<(), JsValue> {
    let request = CloseCollectionRequest {
        downgrade_to_schema11: false,
    };
    call(service::COLLECTION, 1, &request.encode_to_vec()).map(|_| ())
}

/// Adds `count` synthetic Basic notes to the default deck, into an empty collection only, for the
/// tests and the measurements, each with two fields of 200 characters (ADR-022's shape). Returns
/// the number of notes added.
#[wasm_bindgen]
pub fn seed(count: u32) -> Result<u32, JsValue> {
    let held = query(Read::NoteCount)?;
    if held.pointer("/0/0").and_then(serde_json::Value::as_i64) != Some(0) {
        return Err(refuse("seed refuses a collection that holds notes"));
    }
    let names: NotetypeNames = decode(&call(service::NOTETYPES, 8, &[])?)?;
    let basic = names
        .entries
        .iter()
        .find(|entry| entry.name == "Basic")
        .ok_or_else(|| refuse("no Basic notetype"))?
        .id;
    let template: Note = decode(&call(
        service::NOTES,
        0,
        &NotetypeId { ntid: basic }.encode_to_vec(),
    )?)?;
    let requests = (0..count)
        .map(|i| {
            let mut note = template.clone();
            note.fields = Vec::from(fields(i));
            AddNoteRequest {
                note: Some(note),
                deck_id: DEFAULT_DECK,
            }
        })
        .collect();
    let reply: AddNotesResponse = decode(&call(
        service::NOTES,
        2,
        &AddNotesRequest { requests }.encode_to_vec(),
    )?)?;
    u32::try_from(reply.nids.len()).map_err(refuse)
}

/// The wall clock in whole milliseconds since the Unix epoch, from the JS `Date` (the engine's own
/// clock reads it the same way on `wasm32`, the fork's `js-date-clock` patch).
#[expect(
    clippy::cast_possible_truncation,
    reason = "Date.now() is a whole number of milliseconds, far inside i64"
)]
fn now_millis() -> i64 {
    js_sys::Date::now() as i64
}

fn first_queued() -> Result<Option<anki_proto::scheduler::queued_cards::QueuedCard>, JsValue> {
    let request = GetQueuedCardsRequest {
        fetch_limit: 1,
        intraday_learning_only: false,
    };
    let queued: QueuedCards = decode(&call(service::SCHEDULER, 3, &request.encode_to_vec())?)?;
    Ok(queued.cards.into_iter().next())
}

/// The next card's id, or `None` when nothing is due.
#[wasm_bindgen]
pub fn next_card() -> Result<Option<i64>, JsValue> {
    Ok(first_queued()?
        .and_then(|queued| queued.card)
        .map(|card| card.id))
}

/// Answers the next card with a wire rating, 1 to 4, as a reviewer does. Returns its card id.
#[wasm_bindgen]
pub fn answer(rating: u32, milliseconds_taken: u32) -> Result<i64, JsValue> {
    let answer = Answer::from_wire(rating).map_err(refuse)?;
    let queued = first_queued()?.ok_or_else(|| refuse("no card is queued"))?;
    let card = queued
        .card
        .ok_or_else(|| refuse("a queued card without its card"))?;
    let states = queued
        .states
        .ok_or_else(|| refuse("a queued card without its states"))?;
    let request = CardAnswer {
        card_id: card.id,
        current_state: states.current,
        new_state: answer.pick(states.again, states.hard, states.good, states.easy),
        rating: answer.rating(),
        answered_at_millis: now_millis(),
        milliseconds_taken,
    };
    call(service::SCHEDULER, 4, &request.encode_to_vec())?;
    Ok(card.id)
}

/// Undoes the last operation, as the reviewer's undo does, and forgets the kept card: the undone
/// card is shown again by the next card view (SPEC-350 R2).
#[wasm_bindgen]
pub fn undo() -> Result<(), JsValue> {
    call(service::COLLECTION, 8, &[])?;
    SHOWN.with(|kept| *kept.borrow_mut() = None);
    Ok(())
}

/// One card's scheduling fields as JSON (`[id, queue, type, due, ivl, reps, lapses]`), or `null`.
#[wasm_bindgen]
pub fn snapshot(card_id: i64) -> Result<String, JsValue> {
    let row = query(Read::CardSnapshot(card_id))?;
    Ok(row
        .pointer("/0")
        .cloned()
        .unwrap_or(serde_json::Value::Null)
        .to_string())
}

/// One deck of the tree as JSON: its id as a decimal string, its name, level, new, learning and
/// review counts, and its children.
fn deck_json(node: &DeckTreeNode) -> serde_json::Value {
    serde_json::json!({
        "id": node.deck_id.to_string(),
        "name": node.name,
        "level": node.level,
        "new": node.new_count,
        "learning": node.learn_count,
        "review": node.review_count,
        "children": node.children.iter().map(deck_json).collect::<Vec<_>>(),
    })
}

/// The decks under the tree's root, as JSON, with today's counts. Asking for today's counts also
/// unburies on a day rollover, as the desktop's deck list does (SPEC-350 R6, M11).
#[wasm_bindgen]
pub fn deck_tree() -> Result<String, JsValue> {
    let request = DeckTreeRequest {
        now: now_millis() / 1000,
    };
    let root: DeckTreeNode = decode(&call(service::DECKS, 4, &request.encode_to_vec())?)?;
    let decks: Vec<serde_json::Value> = root.children.iter().map(deck_json).collect();
    Ok(serde_json::Value::Array(decks).to_string())
}

/// Makes `deck` the current deck, whose cards the queue then holds (SPEC-350 R6).
#[wasm_bindgen]
pub fn set_current_deck(deck: i64) -> Result<(), JsValue> {
    call(service::DECKS, 22, &DeckId { did: deck }.encode_to_vec()).map(|_| ())
}

/// A rendered side's text: each text node as it is, and each replacement node's current text.
fn joined(nodes: &[RenderedTemplateNode]) -> String {
    nodes
        .iter()
        .filter_map(|node| match &node.value {
            Some(Value::Text(text)) => Some(text.as_str()),
            Some(Value::Replacement(replacement)) => Some(replacement.current_text.as_str()),
            None => None,
        })
        .collect()
}

/// The queue's head as the review shows it, as JSON: the queue's new, learning and review counts,
/// and the card, or `null` when the deck is done. The card carries its id as a decimal string, its
/// ordinal and flag, both sides rendered in full with sound and speech tags stripped, the note
/// type's CSS, the four interval labels for the states read with it, and the engine's undo label.
/// The card is kept with those states and its flag for `rate`, `bury` and `flag` (SPEC-350 R2, R3).
#[wasm_bindgen]
pub fn current_card() -> Result<String, JsValue> {
    SHOWN.with(|kept| *kept.borrow_mut() = None);
    let request = GetQueuedCardsRequest {
        fetch_limit: 1,
        intraday_learning_only: false,
    };
    let queued: QueuedCards = decode(&call(service::SCHEDULER, 3, &request.encode_to_vec())?)?;
    let counts = serde_json::json!({
        "new": queued.new_count,
        "learning": queued.learning_count,
        "review": queued.review_count,
    });
    let Some(head) = queued.cards.into_iter().next() else {
        return Ok(serde_json::json!({ "counts": counts, "card": null }).to_string());
    };
    let card = head
        .card
        .ok_or_else(|| refuse("a queued card without its card"))?;
    let states = head
        .states
        .ok_or_else(|| refuse("a queued card without its states"))?;
    let render = RenderExistingCardRequest {
        card_id: card.id,
        browser: false,
        partial_render: false,
    };
    let rendered: RenderCardResponse =
        decode(&call(service::CARD_RENDERING, 6, &render.encode_to_vec())?)?;
    let question = Text {
        val: joined(&rendered.question_nodes),
    };
    let question: Text = decode(&call(
        service::CARD_RENDERING,
        9,
        &question.encode_to_vec(),
    )?)?;
    let answer = Text {
        val: joined(&rendered.answer_nodes),
    };
    let answer: Text = decode(&call(service::CARD_RENDERING, 9, &answer.encode_to_vec())?)?;
    let labels: StringList = decode(&call(service::SCHEDULER, 24, &states.encode_to_vec())?)?;
    let undo: UndoStatus = decode(&call(service::COLLECTION, 7, &[])?)?;
    let view = serde_json::json!({
        "counts": counts,
        "card": {
            "id": card.id.to_string(),
            "ordinal": card.template_idx,
            "flag": card.flags,
            "question": question.val,
            "answer": answer.val,
            "css": rendered.css,
            "labels": labels.vals,
            "undo": undo.undo,
        },
    });
    SHOWN.with(|kept| {
        *kept.borrow_mut() = Some(Shown {
            card: card.id,
            states,
            flag: card.flags,
        });
    });
    Ok(view.to_string())
}

/// Rates the kept card, and no other, with a wire rating, 1 to 4: the engine answers it with the
/// states kept when it was shown and the next state the rating picks, then the kept card is
/// forgotten (SPEC-350 R2).
#[wasm_bindgen]
pub fn rate(card: i64, rating: u32, milliseconds: u32) -> Result<(), JsValue> {
    let answer = Answer::from_wire(rating).map_err(refuse)?;
    let shown = SHOWN
        .with(|kept| shown_for(kept.borrow().as_ref(), card).cloned())
        .map_err(refuse)?;
    let states = shown.states;
    let request = CardAnswer {
        card_id: shown.card,
        current_state: states.current,
        new_state: answer.pick(states.again, states.hard, states.good, states.easy),
        rating: answer.rating(),
        answered_at_millis: now_millis(),
        milliseconds_taken: milliseconds,
    };
    call(service::SCHEDULER, 4, &request.encode_to_vec())?;
    SHOWN.with(|kept| *kept.borrow_mut() = None);
    Ok(())
}

/// Buries the kept card, and no other, as the user's bury of that card alone, then forgets it
/// (SPEC-350 R2).
#[wasm_bindgen]
pub fn bury(card: i64) -> Result<(), JsValue> {
    SHOWN
        .with(|kept| shown_for(kept.borrow().as_ref(), card).map(|_| ()))
        .map_err(refuse)?;
    let bury = bury_of(card);
    let request = BuryOrSuspendCardsRequest {
        card_ids: bury.card_ids,
        note_ids: bury.note_ids,
        mode: bury.mode,
    };
    call(service::SCHEDULER, 14, &request.encode_to_vec())?;
    SHOWN.with(|kept| *kept.borrow_mut() = None);
    Ok(())
}

/// Toggles red on the kept card, and no other, and answers the flag it now carries, which the kept
/// card keeps for the next toggle (SPEC-350 R7).
#[wasm_bindgen]
pub fn flag(card: i64) -> Result<u32, JsValue> {
    let flag = SHOWN
        .with(|kept| shown_for(kept.borrow().as_ref(), card).map(|kept| toggled_red(kept.flag)))
        .map_err(refuse)?;
    let request = SetFlagRequest {
        card_ids: vec![card],
        flag,
    };
    call(service::CARDS, 4, &request.encode_to_vec())?;
    SHOWN.with(|kept| {
        if let Some(kept) = kept.borrow_mut().as_mut() {
            kept.flag = flag;
        }
    });
    Ok(flag)
}

/// The module's linear memory in pages of 65536 bytes. Linear memory never shrinks, so a reading
/// taken after each step is the Worker's high-water so far (SPEC-338 R3, ADR-348).
#[wasm_bindgen]
pub fn memory_pages() -> u32 {
    u32::try_from(core::arch::wasm32::memory_size::<0>()).unwrap_or(u32::MAX)
}

/// The backend's protocol, held to the study calls: a service and method index, a protobuf
/// request, a protobuf reply. Any other pair is refused by name before the engine sees it.
#[wasm_bindgen]
pub fn run_method(service: u32, method: u32, input: &[u8]) -> Result<Vec<u8>, JsValue> {
    admit(service, method).map_err(refuse)?;
    call(service, method, input)
}
