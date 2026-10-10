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
use anki_proto::card_rendering::HtmlToTextLineRequest;
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
use anki_proto::scheduler::SchedulingState;
use anki_proto::scheduler::SchedulingStates;
use anki_proto::sync::SyncAuth;
use anki_proto::sync::SyncCollectionRequest;
use anki_proto::sync::SyncCollectionResponse;
use anki_proto::sync::SyncLoginRequest;
use prost::Message;
use sqlite_wasm_vfs::sahpool::OpfsSAHPoolCfgBuilder;
use sqlite_wasm_vfs::sahpool::OpfsSAHPoolUtil;
use sqlite_wasm_vfs::sahpool::install;
use wasm_bindgen::prelude::*;

use deck_streak_engine_core::answer::{self, OwnerAnswer};
use deck_streak_engine_core::credential::{self, Generation, Kept, Outcome};
use deck_streak_engine_core::dispatch::{Dispatcher, Read, Refusal};
use deck_streak_engine_core::face::{Clip, Face, Side};
use deck_streak_engine_core::gesture::{GestureRefusal, OwnerGesture, Target};
use deck_streak_engine_core::late;
use deck_streak_engine_core::media::{Reader, TYPES};
use deck_streak_engine_core::review::{bury_of, toggled_red};
use deck_streak_engine_core::table::{ExemptWrite, Transport};
use deck_streak_engine_core::undo_answer::{self, Review};
use js_sys::{Array, Object, Reflect, Uint8Array};

use crate::study::{
    Files, Grade, LastAnswer, Recorded, Returns, Shown, StudyError, UndoRefusal, Wanted, admit,
    engine_languages, grade, last_answer_for, media_type, service, shown_for, undo_view,
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
    static POOL: RefCell<Option<std::rc::Rc<OpfsSAHPoolUtil>>> = const { RefCell::new(None) };
    static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
    /// The card the review last showed, with its states and its flag (SPEC-350 R2).
    static SHOWN: RefCell<Option<Shown<SchedulingStates>>> = const { RefCell::new(None) };
    /// The review's own last answer, the one answer its undo may revert (SPEC-371 R6; ADR-382 D4).
    static LAST_ANSWER: RefCell<Option<LastAnswer>> = const { RefCell::new(None) };
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
            Refusal::NotAllowed { service, method }
            | Refusal::NeedsGesture { service, method }
            | Refusal::NeedsAnswer { service, method } => {
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
    POOL.with(|p| *p.borrow_mut() = Some(std::rc::Rc::new(pool)));
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
    // the pool is the core's files port on this target (SPEC-377 R4; ADR-388 D7, D8)
    let port: Arc<dyn CoreFiles> = Arc::new(PoolPort);
    dispatcher.install_files(port);
    DISPATCHER.with(|d| *d.borrow_mut() = Some(dispatcher));
    Ok(())
}

/// Opens the collection, creating it when the pool holds none. Returns JSON:
/// `{"existed": bool, "notes": number}`, so a collection lost to eviction says so. No answer of
/// another opening is kept for an undo (SPEC-371 R6).
#[wasm_bindgen]
pub fn open() -> Result<String, JsValue> {
    LAST_ANSWER.with(|kept| *kept.borrow_mut() = None);
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

/// Closes the collection, and forgets the review's own last answer, which no later opening may
/// undo (SPEC-371 R6).
#[wasm_bindgen]
pub fn close() -> Result<(), JsValue> {
    LAST_ANSWER.with(|kept| *kept.borrow_mut() = None);
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

/// The undo the review offers for its own last answer (SPEC-371 R7, R8), as JSON:
/// `{"offer": {card, step, text, grade, returns}}` with the card's id as a decimal string, the step
/// the record holds, the card's question as one line of text, the grade the answer recorded and the
/// kind of state the undo returns the card to; or `{"offer": null, "why": "synced" | "none"}`. The
/// core's rule judges the record against the engine now. It reads, and writes nothing.
#[wasm_bindgen]
pub fn undo_offer() -> Result<String, JsValue> {
    let Some(last) = LAST_ANSWER.with(|kept| kept.borrow().clone()) else {
        return Ok(no_offer(UndoRefusal::Gone));
    };
    let recorded = engine_record(&last);
    let now: UndoStatus = decode(&call(service::COLLECTION, 7, &[])?)?;
    if let Err(refusal) =
        undo_answer::judge(&recorded, &now, review_of(recorded.review)?, last.card)
    {
        return Ok(no_offer(mirrored(refusal)));
    }
    let render = RenderExistingCardRequest {
        card_id: last.card,
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
    let line = HtmlToTextLineRequest {
        text: question.val,
        preserve_media_filenames: true,
    };
    let text: Text = decode(&call(service::CARD_RENDERING, 14, &line.encode_to_vec())?)?;
    Ok(serde_json::json!({
        "offer": {
            "card": last.card.to_string(),
            "step": last.recorded.step,
            "text": text.val,
            "grade": last.grade.word(),
            "returns": last.returns.word(),
        },
    })
    .to_string())
}

/// No offer, with the reason the page reads: `synced` for a synced answer, `none` otherwise.
fn no_offer(refusal: UndoRefusal) -> String {
    serde_json::json!({ "offer": null, "why": refusal.why() }).to_string()
}

/// Undoes the review's own last answer, and only the one its offer named (SPEC-371 R7): the
/// confirmation's card and step must be the kept answer's, the core's rule judges the record again
/// against the engine now, and the write runs only through the owner's gesture on that card, which
/// the core judges once more at the write. Then the record and the kept card are forgotten, and the
/// next card view shows the undone card again. A refusal reads as its own sentence, `undo-synced`
/// for a synced answer and `not-undoable` for every other.
#[wasm_bindgen]
pub fn undo(card: i64, step: u32) -> Result<(), JsValue> {
    let last = LAST_ANSWER
        .with(|kept| last_answer_for(kept.borrow().as_ref(), card, step).cloned())
        .map_err(refuse)?;
    let recorded = engine_record(&last);
    let now: UndoStatus = decode(&call(service::COLLECTION, 7, &[])?)?;
    undo_answer::judge(&recorded, &now, review_of(recorded.review)?, card)
        .map_err(|refusal| refuse(StudyError::NotUndoable(mirrored(refusal))))?;
    let gesture = OwnerGesture::from_tap(ExemptWrite::Undo, Target::Card(card)).map_err(refuse)?;
    dispatcher()?
        .run_exempt(gesture, &recorded.encode_to_vec())
        .map_err(|refusal| match refusal {
            GestureRefusal::NotTheTarget { .. } => {
                refuse(StudyError::NotUndoable(UndoRefusal::Changed))
            }
            other => refuse(other),
        })?;
    LAST_ANSWER.with(|kept| *kept.borrow_mut() = None);
    SHOWN.with(|kept| *kept.borrow_mut() = None);
    Ok(())
}

/// The core's record of the kept answer, which its rule judges and its exempt door decodes
/// (SPEC-371 R5).
fn engine_record(last: &LastAnswer) -> undo_answer::Recorded {
    undo_answer::Recorded {
        status: Some(UndoStatus {
            undo: last.recorded.label.clone(),
            redo: String::new(),
            last_step: last.recorded.step,
        }),
        review: last.recorded.review,
    }
}

/// The review-log row `id`, by the core's fixed read (SPEC-371 R4), or `None` when the collection
/// lacks it.
fn review_of(id: i64) -> Result<Option<Review>, JsValue> {
    let rows = query(Read::Review(id))?;
    let Some(row) = rows.pointer("/0") else {
        return Ok(None);
    };
    let cid = row.pointer("/0").and_then(serde_json::Value::as_i64);
    let usn = row
        .pointer("/1")
        .and_then(serde_json::Value::as_i64)
        .and_then(|usn| i32::try_from(usn).ok());
    match (cid, usn) {
        (Some(cid), Some(usn)) => Ok(Some(Review { cid, usn })),
        _ => Err(refuse(
            "the review read answered other than a card and a sync mark",
        )),
    }
}

/// The study rule's word for the core's refusal, one for one: the study rule holds no engine type.
fn mirrored(refusal: undo_answer::UndoRefusal) -> UndoRefusal {
    match refusal {
        undo_answer::UndoRefusal::Gone => UndoRefusal::Gone,
        undo_answer::UndoRefusal::NotTheCard => UndoRefusal::NotTheCard,
        undo_answer::UndoRefusal::Synced => UndoRefusal::Synced,
        undo_answer::UndoRefusal::Changed => UndoRefusal::Changed,
    }
}

/// The kind of state an undo returns a card to, by the core's rule, from the state the card was in
/// when it was answered, as the study rule names it.
fn returned(state: Option<&SchedulingState>) -> Returns {
    let Some(state) = state else {
        return Returns::New;
    };
    match undo_answer::returns_to(state) {
        undo_answer::Returns::New => Returns::New,
        undo_answer::Returns::Learning => Returns::Learning,
        undo_answer::Returns::Review => Returns::Review,
        undo_answer::Returns::Relearning => Returns::Relearning,
        undo_answer::Returns::Preview => Returns::Preview,
    }
}

/// The newest review-log row's id and card, by the core's fixed read (SPEC-371 R4), or `None` when
/// the collection holds no review.
fn newest_review(rows: &serde_json::Value) -> Result<Option<Newest>, JsValue> {
    let Some(row) = rows.pointer("/0") else {
        return Ok(None);
    };
    let id = row.pointer("/0").and_then(serde_json::Value::as_i64);
    let card = row.pointer("/1").and_then(serde_json::Value::as_i64);
    match (id, card) {
        (Some(id), Some(card)) => Ok(Some(Newest { id, card })),
        _ => Err(refuse(
            "the newest review read answered other than a row and a card",
        )),
    }
}

/// The newest review-log row: its id and the card it reviewed.
struct Newest {
    id: i64,
    card: i64,
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

/// Runs one owner's tap on an exempt write (SPEC-345 R9): `write` names the write by its place in
/// the core's exempt table (0 `Forget`, 1 `SetDueDate`, 2 `DeletePreset`, 3 `ChangeNoteType`,
/// 4 `DeleteCard`, 5 `DeleteNote`), `target` its one target's id, and `input` the write's request,
/// which runs only when it names exactly that target. A refusal reads as its own sentence.
#[wasm_bindgen]
pub fn run_exempt(write: u32, target: i64, input: &[u8]) -> Result<Vec<u8>, JsValue> {
    let (write, target) = match write {
        0 => (ExemptWrite::Forget, Target::Card(target)),
        1 => (ExemptWrite::SetDueDate, Target::Card(target)),
        2 => (ExemptWrite::DeletePreset, Target::Preset(target)),
        3 => (ExemptWrite::ChangeNoteType, Target::Note(target)),
        4 => (ExemptWrite::DeleteCard, Target::Card(target)),
        5 => (ExemptWrite::DeleteNote, Target::Note(target)),
        _ => return Err(refuse(format!("tap {write} names no exempt write"))),
    };
    let gesture = OwnerGesture::from_tap(write, target).map_err(refuse)?;
    dispatcher()?.run_exempt(gesture, input).map_err(refuse)
}

/// Whether the Worker keeps a sync login it started at generation `started` while the store reads
/// `current` (SPEC-363 R4): the generation to store it at, or nothing when a removal or another
/// kept login came first. The core's rule decides; this only carries it across the boundary.
#[wasm_bindgen]
#[must_use]
pub fn credential_on_obtained(started: u64, current: u64) -> Option<u64> {
    match credential::on_obtained(Generation::from(started), Generation::from(current)) {
        Kept::Store { at } => Some(u64::from(at)),
        Kept::Discard => None,
    }
}

/// Whether the Worker may send the sync key it holds at generation `held` while the store reads
/// `current` and a sealed record is or is not stored (SPEC-363 R4). The core's rule decides.
#[wasm_bindgen]
#[must_use]
pub fn credential_may_send(held: u64, current: u64, sealed: bool) -> bool {
    credential::may_send(Generation::from(held), Generation::from(current), sealed)
}

/// A sync's answer as the core's rule reads it (SPEC-363 R4): 0 accepted when there is no error,
/// 1 refused, 2 failed. `error` is the engine error's bytes, as the dispatcher's refusal carries
/// them.
#[wasm_bindgen]
#[must_use]
pub fn credential_classify(error: Option<Vec<u8>>) -> u8 {
    match credential::classify(error.as_deref()) {
        Outcome::Accepted => 0,
        Outcome::Refused => 1,
        Outcome::Failed => 2,
    }
}

/// Whether an answer to a send at generation `sent` drops the key while the store reads `current`
/// (SPEC-363 R4). `outcome` is [`credential_classify`]'s number; any other number reads as failed,
/// which never drops the key. The core's rule decides.
#[wasm_bindgen]
#[must_use]
pub fn credential_on_outcome(sent: u64, current: u64, outcome: u8) -> bool {
    let outcome = match outcome {
        0 => Outcome::Accepted,
        1 => Outcome::Refused,
        _ => Outcome::Failed,
    };
    credential::on_outcome(Generation::from(sent), Generation::from(current), outcome)
}

/// The generation a removal moves the store to from `current` (SPEC-363 R4), or nothing at the
/// maximum, where the Worker deletes the record and stores no key again. The core's rule decides.
#[wasm_bindgen]
#[must_use]
pub fn credential_on_removed(current: u64) -> Option<u64> {
    credential::on_removed(Generation::from(current)).map(u64::from)
}

/// A sync call's refusal as the Worker reads it (SPEC-364 R17, ADR-375 D16): the engine's own
/// error keeps its bytes, which the credential module's classifier reads to tell a refused key
/// from a lost network; any other refusal is the boundary's.
fn sync_refusal(refusal: Refusal) -> JsValue {
    match refusal {
        Refusal::Engine { error } => Uint8Array::from(error.as_slice()).into(),
        Refusal::NotAllowed { service, method }
        | Refusal::NeedsGesture { service, method }
        | Refusal::NeedsAnswer { service, method } => {
            refuse(StudyError::CallRefused { service, method })
        }
    }
}

/// Logs in to the sync server at `endpoint` and answers its host key (SPEC-364 R17, ADR-375 D16).
/// The login reaches the engine through the dispatcher, whose web column admits it, never through
/// the study allow-list; an engine refusal is thrown as the engine's error bytes.
#[wasm_bindgen]
pub fn sync_login(endpoint: String, user: String, password: String) -> Result<String, JsValue> {
    let request = SyncLoginRequest {
        username: user,
        password,
        endpoint: Some(endpoint),
    };
    let reply = dispatcher()?
        .run(service::SYNC, 3, &request.encode_to_vec())
        .map_err(sync_refusal)?;
    let auth: SyncAuth = decode(&reply)?;
    Ok(auth.hkey)
}

/// Runs a normal sync with the host key `key` against `endpoint`, with no media and the engine's
/// own timeout, and answers what the server requires next as the engine's number (SPEC-364 R18,
/// ADR-375 D16). An engine refusal is thrown as the engine's error bytes.
#[wasm_bindgen]
pub fn sync_collection(key: String, endpoint: String) -> Result<u32, JsValue> {
    let request = SyncCollectionRequest {
        auth: Some(SyncAuth {
            hkey: key,
            endpoint: Some(endpoint),
            io_timeout_secs: None,
        }),
        sync_media: false,
    };
    let reply = dispatcher()?
        .run(service::SYNC, 5, &request.encode_to_vec())
        .map_err(sync_refusal)?;
    let response: SyncCollectionResponse = decode(&reply)?;
    u32::try_from(response.required).map_err(refuse)
}

/// Hands the core the sync service's statement of its minimum client level: the body the Worker
/// read at its own origin, or nothing when no answer was read (SPEC-374 R23). Answers nothing when
/// the statement admits this client, and otherwise the sentence the core now refuses every sync
/// with. The core decides; this export keeps no rule of its own.
#[wasm_bindgen]
pub fn handshake(statement: Option<Vec<u8>>) -> Result<Option<String>, JsValue> {
    dispatcher()?.handshake(statement.as_deref());
    let outcome = deck_streak_engine_core::handshake::decide(statement.as_deref());
    match deck_streak_engine_core::handshake::admits(outcome) {
        Ok(()) => Ok(None),
        Err(refusal) => {
            let error: BackendError = decode(&refusal)?;
            Ok(Some(error.message))
        }
    }
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
/// It carries `late`, the core's answer whether the card is past its due day in the engine's day,
/// so its review cannot count toward the streak for that day (SPEC-376 R4).
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
    let judged = match LAST_ANSWER.with(|kept| kept.borrow().clone()) {
        Some(last) => {
            let recorded = engine_record(&last);
            let review = review_of(recorded.review)?;
            Some(undo_answer::judge(&recorded, &undo, review, last.card).map_err(mirrored))
        }
        None => None,
    };
    let day = dispatcher()?
        .engine_day()
        .map_err(|_| refuse("the engine's day was not read"))?;
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
            "undo": undo_view(judged),
            "late": late::past_due_day(&card, day),
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

/// Rates the kept card, and no other, with a wire rating, 1 for Again or 3 for Good: the core
/// records it as the owner's answer to that card, with the states kept when it was shown and the
/// next state its grade picks, then the kept card is forgotten (SPEC-350 R2, SPEC-365 R7). Hard
/// and Easy are refused by name before anything reaches the engine. Once the answer is recorded,
/// it is kept as the review's own last answer, with the engine's undo status and the review row it
/// wrote, only when the newest review is of the rated card (SPEC-371 R6).
#[wasm_bindgen]
pub fn rate(card: i64, rating: u32, milliseconds: u32) -> Result<(), JsValue> {
    let grade = grade(rating).map_err(refuse)?;
    let shown = SHOWN
        .with(|kept| shown_for(kept.borrow().as_ref(), card).cloned())
        .map_err(refuse)?;
    let states = shown.states;
    let returns = returned(states.current.as_ref());
    let request = CardAnswer {
        card_id: shown.card,
        current_state: states.current,
        new_state: grade.pick(states.again, states.good),
        rating: grade.rating(),
        answered_at_millis: now_millis(),
        milliseconds_taken: milliseconds,
    };
    let answer = OwnerAnswer::from_press(shown.card, pressed(grade));
    dispatcher()?
        .run_answer(answer, &request.encode_to_vec())
        .map_err(refuse)?;
    SHOWN.with(|kept| *kept.borrow_mut() = None);
    LAST_ANSWER.with(|kept| *kept.borrow_mut() = None);
    let queue: UndoStatus = decode(&call(service::COLLECTION, 7, &[])?)?;
    let newest = newest_review(&query(Read::NewestReview)?)?;
    let recorded = newest
        .filter(|newest| newest.card == shown.card)
        .map(|newest| LastAnswer {
            card: shown.card,
            grade,
            returns,
            recorded: Recorded {
                step: queue.last_step,
                label: queue.undo,
                review: newest.id,
            },
        });
    LAST_ANSWER.with(|kept| *kept.borrow_mut() = recorded);
    Ok(())
}

/// The core's grade for the grade the wire named, one for one. The study rule keeps its own grade
/// because it holds no engine type, so the native tests judge the rule this module runs; this is
/// where the press becomes the core's (SPEC-365 R7).
fn pressed(grade: Grade) -> answer::Grade {
    match grade {
        Grade::Again => answer::Grade::Again,
        Grade::Good => answer::Grade::Good,
    }
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

/// The core reads a face's media through the files the Worker gave, and each name it asks for
/// that they lack is recorded for the Worker to read (SPEC-350 R14, ADR-361 D12).
impl Reader for Wanted<'_> {
    fn read(&self, name: &str, limit: u64) -> Option<Vec<u8>> {
        self.ask(name, limit)
    }
}

/// Both faces of the kept card, and no other, as the core completes them with the media files
/// `names` and `contents` carry: `{question, answer, wanted}`. Each face is `{text, css, autoplay,
/// replay, omitted}`, its text holding the core's `data:` URLs; `wanted` names each file the core
/// asked for that was not given, with the limit it asked for. The core reads synchronously and the
/// media directory does not, so the Worker asks once with no files and again with the files the
/// first answer wanted. The client always wishes autoplay; the card's preset decides (SPEC-350 R14,
/// R15, ADR-361 D12).
#[wasm_bindgen]
pub fn faces(card: i64, names: Vec<String>, contents: Vec<Uint8Array>) -> Result<JsValue, JsValue> {
    SHOWN
        .with(|kept| shown_for(kept.borrow().as_ref(), card).map(|_| ()))
        .map_err(refuse)?;
    let files = Files::new(
        names
            .into_iter()
            .zip(contents.into_iter().map(|bytes| bytes.to_vec())),
    );
    let wanted = Wanted::new(&files);
    let engine = dispatcher()?;
    let question = engine
        .face(card, Side::Question, true, &wanted)
        .map_err(|_| refuse("the engine could not complete the question"))?;
    let answer = engine
        .face(card, Side::Answer, true, &wanted)
        .map_err(|_| refuse("the engine could not complete the answer"))?;
    let reply = Object::new();
    set(&reply, "question", &face_value(question)?)?;
    set(&reply, "answer", &face_value(answer)?)?;
    set(&reply, "wanted", &wanted_value(wanted.into_names())?)?;
    Ok(reply.into())
}

/// Writes `value` to `target` under `key`.
fn set(target: &Object, key: &str, value: &JsValue) -> Result<(), JsValue> {
    Reflect::set(target, &JsValue::from_str(key), value).map(|_| ())
}

/// A face as the page reads it.
fn face_value(face: Face) -> Result<JsValue, JsValue> {
    let value = Object::new();
    set(&value, "text", &face.text.into())?;
    set(&value, "css", &face.css.into())?;
    set(&value, "autoplay", &clips_value(face.autoplay)?)?;
    set(&value, "replay", &clips_value(face.replay)?)?;
    let omitted: Array = face.omitted.into_iter().map(JsValue::from).collect();
    set(&value, "omitted", &omitted)?;
    Ok(value.into())
}

/// The clips, in the core's order.
fn clips_value(clips: Vec<Clip>) -> Result<JsValue, JsValue> {
    let array = Array::new();
    for clip in clips {
        array.push(&clip_value(clip)?);
    }
    Ok(array.into())
}

/// A sound as its name, its bytes and the type the core's table gives its name; speech as its
/// text, language and the native platform's rate.
fn clip_value(clip: Clip) -> Result<JsValue, JsValue> {
    let value = Object::new();
    match clip {
        Clip::Sound { name, bytes } => {
            set(&value, "kind", &"sound".into())?;
            let media = media_type(&name, &TYPES).map_or(JsValue::NULL, JsValue::from_str);
            set(&value, "type", &media)?;
            set(&value, "name", &name.into())?;
            set(&value, "bytes", &Uint8Array::from(bytes.as_slice()).into())?;
        }
        Clip::Speech {
            text,
            language,
            rate,
            ..
        } => {
            set(&value, "kind", &"speech".into())?;
            set(&value, "text", &text.into())?;
            set(&value, "language", &language.into())?;
            set(&value, "rate", &rate.into())?;
        }
    }
    Ok(value.into())
}

/// Each name the core asked for and was not given, with the limit it asked for.
fn wanted_value(names: Vec<(String, u64)>) -> Result<JsValue, JsValue> {
    let array = Array::new();
    for (name, limit) in names {
        let ask = Object::new();
        set(&ask, "name", &name.into())?;
        set(&ask, "limit", &limit.into())?;
        array.push(&ask);
    }
    Ok(array.into())
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

// The full sync's choice (SPEC-377 R4, R5; ADR-388 D7 to D9): the pool as the core's files port,
// one choice's stage between the owner's taps, and the four exports the Worker's choice calls. They
// reach the engine only through `one_way` and the dispatcher's unsynced read; every file a choice
// makes is a new pool name the web engine mints after it reserves the pool for it.

/// The pool as the core's `Files` port (SPEC-377 R4; ADR-388 D8): a path holds a file only when
/// the pool lists its one name, and two paths name one file only when they are one pool name.
struct PoolPort;

impl CoreFiles for PoolPort {
    fn holds(&self, path: &Path) -> bool {
        let name = path.to_string_lossy();
        POOL.with(|p| {
            p.borrow()
                .as_ref()
                .is_some_and(|pool| files::holds(&pool.list(), &name))
        })
    }

    fn same(&self, open: &Path, path: &Path) -> bool {
        files::same(&open.to_string_lossy(), &path.to_string_lossy())
    }
}

/// The pool's refusal to grow, as the Worker reads a storage refusal.
fn storage(error: impl std::fmt::Display) -> JsValue {
    refuse(format!("storage-refused: {error}"))
}

/// A new pool name for a choice file of `kind`: the pool is reserved for it first, three files
/// beyond those it holds (the file, and room for the engine's journal beside it), so the engine
/// can open the file the name names (SPEC-377 R5; ADR-388 D8). The pool is held by a counted
/// handle, so no borrow of its cell is held across the reserve's wait. The reserve stays on one
/// line, where the boundary census plants over it, so rustfmt leaves this function as written.
#[rustfmt::skip]
async fn choice_file(kind: Kind) -> Result<String, JsValue> {
    let pool = POOL
        .with(|p| p.borrow().clone())
        .ok_or_else(|| refuse("storage-refused: the pool is not installed"))?;
    pool.reserve_minimum_capacity(pool.count() + 3).await.map_err(storage)?;
    files::choice_name(&pool.list(), COLLECTION_PATH, kind)
        .ok_or_else(|| refuse("storage-refused: no choice file name is free"))
}

/// One choice's stage between the owner's taps (SPEC-377 R5; ADR-388 D9): the counts the owner
/// saw, and the server copy they were taken from. A Worker that ends drops it, which is the model's
/// `Cancel`.
#[derive(Clone)]
struct Stage {
    counted: Counted,
    copy: String,
}

thread_local! {
    /// The choice's stage, held between the count and the owner's tap.
    static STAGE: RefCell<Option<Stage>> = const { RefCell::new(None) };
}

/// What a choice export answers: its JSON, or the refusal it throws to the Worker. Named short so
/// each export's parameters stay on one line, where the boundary census reads them.
type Json = Result<String, JsValue>;

/// The host key `key` and `endpoint` as the engine's sync auth, with the engine's own timeout.
fn sync_auth(key: String, endpoint: String) -> SyncAuth {
    SyncAuth {
        hkey: key,
        endpoint: Some(endpoint),
        io_timeout_secs: None,
    }
}

/// What one direction loses, as JSON; a direction the engine did not offer is null.
fn losses_json(losses: Option<Losses>) -> serde_json::Value {
    losses.map_or(serde_json::Value::Null, |losses| {
        serde_json::json!({
            "reviews": losses.reviews,
            "cards": losses.cards,
            "notes": losses.notes,
        })
    })
}

/// A confirm that wrote nothing, and why, as JSON.
fn refused(why: &str) -> String {
    serde_json::json!({ "outcome": "refused", "why": why }).to_string()
}

/// A confirm that found `side` changed since the counts, with the new counts, as JSON.
fn changed(side: &str, counts: Counts) -> String {
    serde_json::json!({
        "outcome": "changed",
        "why": side,
        "upload": losses_json(counts.upload),
        "download": losses_json(counts.download),
    })
    .to_string()
}

/// Why a step of the choice refused, as the Worker reads it: a word for the core's own reasons, and
/// the engine's error bytes thrown, so the Worker settles the send with them (ADR-375 D17).
fn why(reason: Reason) -> Result<&'static str, JsValue> {
    match reason {
        Reason::Engine(refusal) => Err(sync_refusal(refusal)),
        Reason::Gesture(GestureRefusal::Engine { error }) => {
            Err(Uint8Array::from(error.as_slice()).into())
        }
        Reason::Gesture(_) => Ok("gesture"),
        Reason::OpenCollection => Ok("open-collection"),
        Reason::HoldsRows => Ok("holds-rows"),
        Reason::Unheld => Ok("unheld"),
    }
}

/// The count's refusal: the engine's error bytes, or the web engine's sentence naming why.
fn uncounted(reason: Reason) -> JsValue {
    match why(reason) {
        Ok(word) => refuse(format!("choice-refused: {word}")),
        Err(bytes) => bytes,
    }
}

/// The choice's counts, from the normal sync's `required` as the Worker heard it, with the host key
/// `key` against `endpoint` (SPEC-377 R5): the server's collection is fetched by the core into a
/// new pool file, and the counts and that file are held as the stage. Answers JSON, what each
/// offered direction loses. An engine refusal is thrown as its bytes.
#[wasm_bindgen]
pub async fn full_sync_count(key: String, endpoint: String, required: u32) -> Json {
    let answer = SyncCollectionResponse {
        required: i32::try_from(required).map_err(refuse)?,
        ..SyncCollectionResponse::default()
    };
    let auth = sync_auth(key, endpoint);
    let copy = choice_file(Kind::Server).await?;
    let counted =
        one_way::count(&dispatcher()?, &answer, &auth, Path::new(&copy)).map_err(uncounted)?;
    let counts = counted.counts();
    STAGE.with(|stage| *stage.borrow_mut() = Some(Stage { counted, copy }));
    Ok(serde_json::json!({
        "upload": losses_json(counts.upload),
        "download": losses_json(counts.download),
    })
    .to_string())
}

/// The owner's tap on `direction` (0 the upload, 1 the download), with the snapshot answer the
/// Worker read, `found` (SPEC-377 R5, R6; ADR-388 D9, D10). The tap builds the one-way gesture;
/// the core backs up the side the write replaces and, for an upload, judges the Worker's answer
/// and checks a fresh server copy, before the write. Answers JSON: written; changed, with the new
/// counts and which side changed; or refused, and why. The stage is kept on every refusal,
/// replaced on a change and dropped on the write. An engine refusal is thrown as its bytes.
#[wasm_bindgen]
pub async fn full_sync_confirm(direction: u32, key: String, endpoint: String, found: bool) -> Json {
    let Some(Stage { counted, copy }) = STAGE.with_borrow(Clone::clone) else {
        return Ok(refused("no-stage"));
    };
    let direction = match direction {
        0 => Direction::Upload,
        1 => Direction::Download,
        _ => return Ok(refused("not-offered")),
    };
    let Ok(gesture) = OwnerGesture::from_tap(ExemptWrite::OneWaySync, Target::Collection) else {
        return Ok(refused("gesture"));
    };
    let Ok(confirmed) = counted.confirm(direction) else {
        return Ok(refused("not-offered"));
    };
    let auth = sync_auth(key, endpoint);
    let dispatcher = dispatcher()?;
    let backup = choice_file(Kind::Backup).await?;
    let made = one_way::back_up(&dispatcher, confirmed, Path::new(&backup), Path::new(&copy));
    let backed_up = match made {
        Ok(backed_up) => backed_up,
        Err(refusal) => return why(refusal.reason).map(refused),
    };
    let ready = match backed_up.download_ready() {
        Ok(ready) => ready,
        Err(backed_up) => {
            let Ok(checked) = backed_up.snapshot_found(&SnapshotAnswer { found }) else {
                return Ok(refused("no-snapshot"));
            };
            let fresh = choice_file(Kind::Server).await?;
            match one_way::recheck(&dispatcher, checked, &auth, Path::new(&fresh)) {
                Ok(Ok(ready)) => ready,
                Ok(Err(counted)) => {
                    let counts = counted.counts();
                    STAGE.set(Some(Stage {
                        counted,
                        copy: fresh,
                    }));
                    return Ok(changed("server", counts));
                }
                Err(refusal) => return why(refusal.reason).map(refused),
            }
        }
    };
    match one_way::write(&dispatcher, ready, gesture, &auth) {
        Ok(()) => {
            STAGE.set(None);
            Ok(serde_json::json!({ "outcome": "written" }).to_string())
        }
        Err(Unwritten::Recount(counted)) => {
            let counts = counted.counts();
            STAGE.set(Some(Stage { counted, copy }));
            Ok(changed("device", counts))
        }
        Err(Unwritten::Refused(reason)) => why(reason).map(refused),
    }
}

/// Drops the stage held between the owner's taps: the model's `Cancel` (ADR-388 D9).
#[wasm_bindgen]
pub fn full_sync_cancel() {
    STAGE.with(|stage| {
        *stage.borrow_mut() = None;
    });
}

/// The device's reviews not yet synced, and whether anything else or its schema changed, read
/// offline by the dispatcher's one fixed statement (SPEC-377 R8). Answers JSON.
#[wasm_bindgen]
pub fn unsynced() -> Result<String, JsValue> {
    let read = dispatcher()?
        .unsynced()
        .map_err(|_| refuse("the unsynced read failed"))?;
    Ok(serde_json::json!({
        "reviews": read.reviews,
        "changed": read.changed,
        "schema": read.schema,
    })
    .to_string())
}

use std::path::Path;
use std::sync::Arc;

use deck_streak_engine_core::files::Files as CoreFiles;
use deck_streak_engine_core::full_sync::{Counted, Counts, Direction, Losses, SnapshotAnswer};
use deck_streak_engine_core::one_way::{self, Reason, Unwritten};

use crate::files::{self, Kind};

use deck_streak_engine_core::retention::{self, Held};
use sqlite_wasm_vfs::sahpool::OpfsSAHPoolUtil as Pool;

/// The installed pool, by a counted handle, so no borrow of its cell is held across a wait.
fn installed() -> Result<std::rc::Rc<Pool>, JsValue> {
    POOL.with(|p| p.borrow().clone())
        .ok_or_else(|| refuse("storage-refused: the pool is not installed"))
}

/// The pool's names once each backup and server copy it lists has a record of when it was first
/// listed (ADR-388 D17): an empty pool entry per name, room reserved for the entries first.
async fn recorded(pool: &Pool) -> Result<Vec<String>, JsValue> {
    let unrecorded = files::unrecorded(&pool.list());
    if !unrecorded.is_empty() {
        let records = u32::try_from(unrecorded.len()).map_err(storage)?;
        let room = pool.count().saturating_add(records);
        pool.reserve_minimum_capacity(room).await.map_err(storage)?;
        let made = now_millis();
        for name in &unrecorded {
            pool.import_db_unchecked(&files::record(name, made), &[])
                .map_err(storage)?;
        }
    }
    Ok(pool.list())
}

/// How long ago, in whole seconds, a file first listed at `made` was listed; never negative.
fn age(made: i64, now: i64) -> u64 {
    u64::try_from((now - made) / 1000).unwrap_or(0)
}

/// The refusal of an export whose id names no backup the pool lists.
fn unlisted(id: &str) -> JsValue {
    refuse(format!("no listed backup is named {id}"))
}

/// The backups and server copies the pool holds, newest first, each by its id, its kind and its
/// age in seconds, as JSON (SPEC-377 R15; ADR-388 D14, D17). No path leaves the engine.
#[wasm_bindgen]
pub async fn backups() -> Json {
    let pool = installed()?;
    let listed = recorded(&pool).await?;
    let now = now_millis();
    let listing: Vec<serde_json::Value> = files::backups(&listed)
        .iter()
        .map(|backup| {
            serde_json::json!({
                "id": backup.id,
                "kind": backup.kind.word(),
                "age_seconds": age(backup.made, now),
            })
        })
        .collect();
    Ok(serde_json::Value::from(listing).to_string())
}

/// The bytes of the backup or server copy `id` names, when the pool lists it as one (SPEC-377
/// R16): the Worker moves them to the page by transfer. Async like the list and the retention, so
/// the Worker awaits the three alike.
#[allow(clippy::unused_async)]
#[wasm_bindgen]
pub async fn export_backup(id: String) -> Result<Vec<u8>, JsValue> {
    let pool = installed()?;
    let name = files::exported(&pool.list(), COLLECTION_PATH, &id).ok_or_else(|| unlisted(&id))?;
    pool.export_db(&name).map_err(storage)
}

/// Keeps the core's number of each kind (SPEC-377 R17; ADR-388 D15, D18): each backup or server
/// copy the core's retention removes goes with its record, and the removed are answered by kind
/// and age as JSON. It refuses while a choice's stage is held, so it never runs inside a choice.
#[wasm_bindgen]
pub async fn retain() -> Json {
    if STAGE.with(|stage| stage.borrow().is_some()) {
        return Err(refuse("retention waits while a choice is held"));
    }
    let pool = installed()?;
    let listed = recorded(&pool).await?;
    let held = files::backups(&listed);
    let kept: Vec<Held<Kind, i64>> = held
        .iter()
        .map(|backup| Held {
            kind: backup.kind,
            made: backup.made,
        })
        .collect();
    let now = now_millis();
    let mut removed = Vec::new();
    for index in retention::removals(&kept) {
        let backup = &held[index];
        pool.delete_db(&backup.name).map_err(storage)?;
        pool.delete_db(&backup.record).map_err(storage)?;
        removed.push(serde_json::json!({
            "kind": backup.kind.word(),
            "age_seconds": age(backup.made, now),
        }));
    }
    Ok(serde_json::Value::from(removed).to_string())
}
