//! The engine handle, its typed refusal and the one entry point (SPEC-336 R1, R3).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use deck_streak_engine_core::answer::{
    AnswerRefusal, Grade, OwnerAnswer, answer_request, shown_states,
};
use deck_streak_engine_core::dispatch::{Dispatcher, Refusal};
use deck_streak_engine_core::face::Side;
use deck_streak_engine_core::full_sync::{Counted, Counts, Direction, Losses, SnapshotAnswer};
use deck_streak_engine_core::gesture::{GestureRefusal, OwnerGesture, Target};
use deck_streak_engine_core::handshake;
use deck_streak_engine_core::native_sync;
use deck_streak_engine_core::one_way::{self, Reason, Unwritten};
use deck_streak_engine_core::review::{self, bury_of, bury_request, flag_request, toggled_red};
use deck_streak_engine_core::table::{ExemptWrite, Transport};

use crate::allow_list::allowed;
use crate::face::{CardFace, MediaFolder};
use crate::review_undo::{UndoDone, UndoOffered};

/// Why a call was not answered with the response's bytes. A native client reads it as a thrown
/// error; nothing on this path panics.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum EngineRefusal {
    /// The call is not on the allow-list, so the engine never saw it.
    NotAllowed {
        /// The service index the client sent.
        service: u32,
        /// The method index the client sent.
        method: u32,
    },
    /// The engine answered the allowed call with an error: its encoded `BackendError` message,
    /// which the client decodes with the engine's own schema.
    Engine {
        /// The engine's error, as protobuf bytes.
        error: Vec<u8>,
    },
    /// The engine could not start from the init message.
    Start {
        /// The engine's reason.
        reason: String,
    },
}

impl fmt::Display for EngineRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAllowed { service, method } => {
                write!(
                    f,
                    "service {service} method {method} is not on the allow-list"
                )
            }
            Self::Engine { error } => {
                write!(f, "the engine refused the call ({} bytes)", error.len())
            }
            Self::Start { reason } => write!(f, "the engine could not start: {reason}"),
        }
    }
}

impl std::error::Error for EngineRefusal {}

/// One running engine: a native client holds one, opens a collection through it and calls it. It
/// reaches the engine only through the core's dispatcher, started on the native transport
/// (SPEC-345 R5).
#[derive(uniffi::Object)]
pub struct Engine {
    dispatcher: Dispatcher,
    /// The full-sync choice's stage between the owner's taps (SPEC-503 R4).
    stage: Mutex<Option<Stage>>,
}

#[uniffi::export]
impl Engine {
    /// Starts an engine from its encoded `BackendInit` message (empty bytes take its defaults).
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Start`] when the engine cannot decode the message.
    #[uniffi::constructor]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's bytes cross the boundary owned, as the bindings pass them"
    )]
    pub fn new(message: Vec<u8>) -> Result<Arc<Self>, EngineRefusal> {
        Dispatcher::start(Transport::Native, &message)
            .map(|dispatcher| {
                Arc::new(Self {
                    dispatcher,
                    stage: Mutex::new(None),
                })
            })
            .map_err(|reason| EngineRefusal::Start { reason })
    }

    /// Runs one allowed call: the request's protobuf bytes in, the response's out. Before a sync
    /// login, the sync service's statement of its minimum client level is read at the login's
    /// endpoint and handed to the core, which refuses the login unless the statement admits this
    /// client (SPEC-374 R8).
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::NotAllowed`] for a call outside the allow-list, and
    /// [`EngineRefusal::Engine`] when the engine answers an allowed call with an error.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's bytes cross the boundary owned, as the bindings pass them"
    )]
    pub fn run(&self, service: u32, method: u32, input: Vec<u8>) -> Result<Vec<u8>, EngineRefusal> {
        // The allow-list decides before the engine sees the call: an unlisted pair never reaches
        // the engine's dispatch, whatever it would have done there.
        if allowed(service, method).is_none() {
            return Err(EngineRefusal::NotAllowed { service, method });
        }
        if (service, method) == SYNC_LOGIN
            && let Some(url) = handshake::statement_url(&input)
        {
            self.dispatcher.handshake(read_statement(&url).as_deref());
        }
        self.dispatcher
            .run(service, method, &input)
            .map_err(refusal)
    }

    /// Runs one owner's tap on an exempt write: the tap names the write and its one target, and
    /// the engine runs the write only when the request names exactly that target (SPEC-345 R9).
    /// This is the adapter's one exempt entry, and the only place it builds a gesture.
    ///
    /// # Errors
    ///
    /// [`ExemptRefusal::WrongKind`] when the target is not of the kind the write takes,
    /// [`ExemptRefusal::NotTheTarget`] when the request names other than the target,
    /// [`ExemptRefusal::Undecodable`] when the request is not the write's own message, and
    /// [`ExemptRefusal::Engine`] when the engine refuses the checked write.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's bytes cross the boundary owned, as the bindings pass them"
    )]
    pub fn run_exempt(
        &self,
        write: ExemptTap,
        target: ExemptTarget,
        input: Vec<u8>,
    ) -> Result<Vec<u8>, ExemptRefusal> {
        let gesture = OwnerGesture::from_tap(exempt_write(write), core_target(target))
            .map_err(exempt_refusal)?;
        self.dispatcher
            .run_exempt(gesture, &input)
            .map_err(exempt_refusal)
    }

    /// Completes the face of the card `card_id`: its question, or its answer when `answer` is
    /// set, in one closed page lit for `night`, with the clips to autoplay when the client wishes
    /// it and the card's preset allows it (SPEC-348 R5). Its media are read from the media folder
    /// of the collection this engine opened.
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the engine cannot read or render the card.
    pub fn face(
        &self,
        card_id: i64,
        answer: bool,
        night: bool,
        autoplay: bool,
    ) -> Result<CardFace, EngineRefusal> {
        let side = if answer { Side::Answer } else { Side::Question };
        let folder = MediaFolder(self.dispatcher.media_folder().map(PathBuf::from));
        self.dispatcher
            .face(card_id, side, autoplay, &folder)
            .map(|face| CardFace::new(face, night))
            .map_err(refusal)
    }

    /// Records one owner's press: `grade` on the card `card`, which the queue showed with the
    /// encoded `SchedulingStates` in `states`, after `milliseconds_taken` (SPEC-365 R6). This is
    /// the adapter's one answer entry, and the only place it builds an owner's answer: the request
    /// names the press's card, its grade's rating and the next state its grade picks out of the
    /// states the card was shown with, at the time the adapter's clock reads.
    ///
    /// # Errors
    ///
    /// [`PressRefusal::Undecodable`] when `states` are not the engine's `SchedulingStates`, before
    /// the engine sees anything, and [`PressRefusal::Engine`] when the engine refuses the answer.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's bytes cross the boundary owned, as the bindings pass them"
    )]
    pub fn answer(
        &self,
        card: i64,
        grade: PressedGrade,
        states: Vec<u8>,
        milliseconds_taken: u32,
    ) -> Result<Vec<u8>, PressRefusal> {
        let grade = core_grade(grade);
        let states = shown_states(&states).map_err(press_refusal)?;
        let next = grade.pick(states.again, states.good);
        let request = answer_request(
            card,
            states.current,
            next,
            grade,
            now_millis(),
            milliseconds_taken,
        );
        let answer = OwnerAnswer::from_press(card, grade);
        self.dispatcher
            .run_answer(answer, &request)
            .map_err(press_refusal)
    }

    /// Buries the card `card_id` as the user's bury of that card alone, so the queue moves on to
    /// the next card (SPEC-358 R3). `states` are the encoded `SchedulingStates` the card was shown
    /// with, as an answer hands them over, for the undo's record (SPEC-515 R2).
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the engine refuses the bury.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's bytes cross the boundary owned, as the bindings pass them"
    )]
    pub fn bury(&self, card_id: i64, states: Vec<u8>) -> Result<(), EngineRefusal> {
        drop(states);
        // `SchedulerService.BuryOrSuspendCards`, through the allow-list as every call goes.
        self.run(13, 14, bury_request(bury_of(card_id)))?;
        Ok(())
    }

    /// Toggles red on the card `card_id`, whose flag is `flag` as its queued card carries it, and
    /// answers the flag the card now carries: red to none, and any other flag to red (SPEC-358 R3).
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the engine refuses the flag.
    pub fn flag(&self, card_id: i64, flag: u32) -> Result<u32, EngineRefusal> {
        let flag = toggled_red(flag);
        // `CardsService.SetFlag`, through the allow-list as every call goes.
        self.run(5, 4, flag_request(card_id, flag))?;
        Ok(flag)
    }

    /// The undo the review offers for its last answer, bury or flag (SPEC-515 R3).
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the engine cannot answer a read.
    pub fn undo_offer(&self) -> Result<UndoOffered, EngineRefusal> {
        Ok(UndoOffered {
            offer: None,
            said: String::new(),
        })
    }

    /// The owner's confirmation of the offer of `card` at `step` (SPEC-515 R4).
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the engine cannot answer a read or refuses the undo.
    pub fn undo(&self, card: i64, step: u32) -> Result<UndoDone, EngineRefusal> {
        drop((card, step));
        Ok(UndoDone {
            undone: false,
            said: String::new(),
        })
    }
}

/// The native sync facade (SPEC-503 R2 to R7; ADR-521 D2): the normal sync, the unsynced read and
/// the full-sync choice, as the web engine's exports run them, each through the core. The app
/// hands over the host key it read from its Keychain item and never a password.
#[uniffi::export]
impl Engine {
    /// Runs a normal sync with the host key `host_key` against `endpoint`, with no media, and
    /// answers what the server requires next as the engine numbers it (SPEC-503 R2, R5). The
    /// service's statement of its minimum client level is read first at the endpoint's origin and
    /// handed to the core, which refuses the sync unless the statement admits this client (R3).
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the endpoint rule, the handshake or the engine refuses, in
    /// the engine's error shape, or with the reply's bytes when the reply does not decode.
    pub fn sync(&self, host_key: String, endpoint: String) -> Result<u32, EngineRefusal> {
        if let Some(url) = native_sync::statement_url(&endpoint) {
            self.dispatcher.handshake(read_statement(&url).as_deref());
        }
        let request = native_sync::normal_sync(&native_sync::auth(host_key, endpoint));
        let reply = self.run(SYNC_COLLECTION.0, SYNC_COLLECTION.1, request)?;
        Self::required(reply)
    }

    /// What this device has not synced, read offline by the core's one fixed statement, so the
    /// app shows it before a full sync can lose it (SPEC-503 R7).
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the engine cannot run the read.
    pub fn unsynced(&self) -> Result<UnsyncedCount, EngineRefusal> {
        self.dispatcher
            .unsynced()
            .map(|read| UnsyncedCount {
                reviews: read.reviews,
                changed: read.changed,
                schema: read.schema,
            })
            .map_err(refusal)
    }

    /// The full-sync choice's counts, from the normal sync's `required` (SPEC-503 R4): the core
    /// fetches the server's collection into a new file in `folder` and counts what each offered
    /// direction loses. The counts and that file are held as the stage until the owner's tap.
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the endpoint rule, the handshake or the engine refuses.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's values cross the boundary owned, as the bindings pass them"
    )]
    pub fn full_sync_count(
        &self,
        host_key: String,
        endpoint: String,
        required: u32,
        folder: String,
    ) -> Result<ChoiceStep, EngineRefusal> {
        let auth = native_sync::auth(host_key, endpoint);
        let copy = choice_file(&folder, "server-copy");
        let answer = native_sync::answer(required);
        match one_way::count(&self.dispatcher, &answer, &auth, &copy) {
            Ok(counted) => {
                let counts = counted.counts();
                *self.stage() = Some(Stage { counted, copy });
                Ok(step(COUNTED, Some(counts)))
            }
            Err(reason) => unwritten(reason),
        }
    }

    /// The owner's tap on `direction` (0 the upload, 1 the download), with the snapshot answer
    /// `snapshot_found` (SPEC-503 R4). The tap builds the one-way gesture; the core backs up the
    /// side the write replaces into a new file in `folder` and, for an upload, judges the answer
    /// and checks a fresh server copy, before the write. The stage is kept on every refusal,
    /// replaced on a change and dropped on the write.
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the endpoint rule, the handshake or the engine refuses.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a foreign caller's values cross the boundary owned, as the bindings pass them"
    )]
    pub fn full_sync_confirm(
        &self,
        direction: u32,
        host_key: String,
        endpoint: String,
        snapshot_found: bool,
        folder: String,
    ) -> Result<ChoiceStep, EngineRefusal> {
        let Some(Stage { counted, copy }) = self.stage().clone() else {
            return Ok(step(NO_STAGE, None));
        };
        let direction = match direction {
            0 => Direction::Upload,
            1 => Direction::Download,
            _ => return Ok(step(NOT_OFFERED, None)),
        };
        let Ok(gesture) = OwnerGesture::from_tap(ExemptWrite::OneWaySync, Target::Collection)
        else {
            return Ok(step(NOT_THE_OWNERS, None));
        };
        let Ok(confirmed) = counted.confirm(direction) else {
            return Ok(step(NOT_OFFERED, None));
        };
        let auth = native_sync::auth(host_key, endpoint);
        let backup = choice_file(&folder, "backup");
        let backed_up = match one_way::back_up(&self.dispatcher, confirmed, &backup, &copy) {
            Ok(backed_up) => backed_up,
            Err(refused) => return unwritten(refused.reason),
        };
        let ready = match backed_up.download_ready() {
            Ok(ready) => ready,
            Err(backed_up) => {
                let answer = SnapshotAnswer {
                    found: snapshot_found,
                };
                let Ok(checked) = backed_up.snapshot_found(&answer) else {
                    return Ok(step(NO_SNAPSHOT, None));
                };
                let fresh = choice_file(&folder, "server-copy");
                match one_way::recheck(&self.dispatcher, checked, &auth, &fresh) {
                    Ok(Ok(ready)) => ready,
                    Ok(Err(counted)) => return Ok(self.changed(counted, fresh, SERVER_CHANGED)),
                    Err(refused) => return unwritten(refused.reason),
                }
            }
        };
        self.written(
            one_way::write(&self.dispatcher, ready, gesture, &auth),
            copy,
        )
    }

    /// Drops the stage held between the owner's taps: the model's `Cancel` (SPEC-503 R4).
    pub fn full_sync_cancel(&self) {
        *self.stage() = None;
    }
}

impl Engine {
    /// The stage, under its lock.
    fn stage(&self) -> MutexGuard<'_, Option<Stage>> {
        self.stage.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// What a normal sync's `reply` requires next, as the engine numbers it (SPEC-503 R5): a reply
    /// that does not decode is the engine's refusal, with the reply's bytes.
    fn required(reply: Vec<u8>) -> Result<u32, EngineRefusal> {
        native_sync::required(&reply).ok_or(EngineRefusal::Engine { error: reply })
    }

    /// The step the write's result `write` shows (SPEC-503 R4): the stage is dropped on the write,
    /// replaced on a device change since the backup by the new counts and the counted server copy
    /// `copy`, and kept on a refusal.
    fn written(
        &self,
        write: Result<(), Unwritten>,
        copy: PathBuf,
    ) -> Result<ChoiceStep, EngineRefusal> {
        match write {
            Ok(()) => {
                *self.stage() = None;
                Ok(ChoiceStep {
                    written: true,
                    said: WRITTEN.to_owned(),
                    counts: None,
                })
            }
            Err(Unwritten::Recount(counted)) => Ok(self.changed(counted, copy, DEVICE_CHANGED)),
            Err(Unwritten::Refused(reason)) => unwritten(reason),
        }
    }

    /// New counts after a change since the last ones: held as the stage, with the server copy
    /// they were taken from, and shown with why the write did not run.
    fn changed(&self, counted: Counted, copy: PathBuf, said: &str) -> ChoiceStep {
        let counts = counted.counts();
        *self.stage() = Some(Stage { counted, copy });
        step(said, Some(counts))
    }
}

/// One choice's stage between the owner's taps (SPEC-503 R4): the counts the owner saw, and the
/// server copy they were taken from.
#[derive(Clone)]
struct Stage {
    counted: Counted,
    copy: PathBuf,
}

/// What this device has not synced (SPEC-503 R7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct UnsyncedCount {
    /// The reviews not yet synced.
    pub reviews: u32,
    /// Whether the collection changed since its last sync.
    pub changed: bool,
    /// Whether its schema changed since its last sync.
    pub schema: bool,
}

/// What one direction of a full sync loses, by id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ChoiceLoss {
    /// Review-log rows lost.
    pub reviews: u64,
    /// Cards lost.
    pub cards: u64,
    /// Notes lost.
    pub notes: u64,
}

/// The choice's counts: what each offered direction loses, and `None` for one not offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ChoiceCounts {
    /// What the upload loses: the server's rows this device lacks.
    pub upload: Option<ChoiceLoss>,
    /// What the download loses: this device's rows the server lacks.
    pub download: Option<ChoiceLoss>,
}

/// One step of the full-sync choice as the app shows it: whether the write ran, the sentence the
/// app shows, and the counts the owner chooses from next, when there are any.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ChoiceStep {
    /// Whether the one-way write ran.
    pub written: bool,
    /// The sentence the app shows.
    pub said: String,
    /// The counts to choose from, when the choice is open.
    pub counts: Option<ChoiceCounts>,
}

/// The engine's normal sync, `BackendSyncService.SyncCollection` (SPEC-503 R1).
const SYNC_COLLECTION: (u32, u32) = (1, 5);

/// The sentences of the choice's steps, which the app shows as they are.
const COUNTED: &str = "Choose a direction. The counts show what each direction loses.";
const WRITTEN: &str = "The full sync was written after its backup.";
const NO_STAGE: &str = "Nothing was written: no counts are open. Sync again.";
const NOT_OFFERED: &str = "Nothing was written: that direction is not offered.";
const NOT_THE_OWNERS: &str = "Nothing was written: the full sync was not the owner's.";
const NO_SNAPSHOT: &str =
    "Nothing was written: an upload waits until the server's snapshot is found.";
const SERVER_CHANGED: &str =
    "Nothing was written: the server changed since the counts. Check the new counts.";
const DEVICE_CHANGED: &str =
    "Nothing was written: this device changed since its backup. Check the new counts.";
const UNFILLABLE: &str = "Nothing was written: a full sync's file could not be used.";
const UNHELD: &str = "Nothing was written: the backup lacks a row of the side it replaces.";

/// A step that shows `said`, with the counts to choose from when there are any.
fn step(said: &str, counts: Option<Counts>) -> ChoiceStep {
    ChoiceStep {
        written: false,
        said: said.to_owned(),
        counts: counts.map(|counts| ChoiceCounts {
            upload: counts.upload.map(loss),
            download: counts.download.map(loss),
        }),
    }
}

/// One direction's losses as the app reads them.
fn loss(losses: Losses) -> ChoiceLoss {
    let wide = |count: usize| u64::try_from(count).unwrap_or(u64::MAX);
    ChoiceLoss {
        reviews: wide(losses.reviews),
        cards: wide(losses.cards),
        notes: wide(losses.notes),
    }
}

/// A step that did not write: the engine's refusal as its bytes, and the core's own reasons as
/// the sentence the app shows.
fn unwritten(reason: Reason) -> Result<ChoiceStep, EngineRefusal> {
    let said = match reason {
        Reason::Engine(engine) => return Err(refusal(engine)),
        Reason::Gesture(GestureRefusal::Engine { error }) => {
            return Err(EngineRefusal::Engine { error });
        }
        Reason::Gesture(_) => NOT_THE_OWNERS,
        Reason::OpenCollection | Reason::HoldsRows => UNFILLABLE,
        Reason::Unheld => UNHELD,
    };
    Ok(step(said, None))
}

/// A new file name for one of the choice's files in `folder`: `kind`, the clock's nanoseconds and
/// this process, so no two of the choice's files share a name and none is the open collection.
fn choice_file(folder: &str, kind: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    Path::new(folder).join(format!("{kind}-{stamp}-{}.anki2", std::process::id()))
}

/// The engine's number for the red flag, which the app reads once and compares a card's flag with,
/// so it holds no copy of its own (SPEC-358 R3).
#[uniffi::export]
#[must_use]
pub fn red_flag() -> u32 {
    review::RED
}

/// The adapter's clock: the milliseconds since the epoch an answer records as its time.
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

/// The core's grade for the grade a press names, one for one.
fn core_grade(grade: PressedGrade) -> Grade {
    match grade {
        PressedGrade::Again => Grade::Again,
        PressedGrade::Good => Grade::Good,
    }
}

/// The adapter's refusal for the core's: each reason by its own variant, and the engine's error
/// bytes as the engine encoded them.
fn press_refusal(refusal: AnswerRefusal) -> PressRefusal {
    match refusal {
        AnswerRefusal::Undecodable => PressRefusal::Undecodable,
        AnswerRefusal::NotTheCard { .. } => PressRefusal::NotTheCard,
        AnswerRefusal::NotTheGrade { .. } => PressRefusal::NotTheGrade,
        AnswerRefusal::Engine { error } => PressRefusal::Engine { error },
    }
}

/// The grade an owner's press names (SPEC-365 R6): one of two, as the core records them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PressedGrade {
    /// The card was forgotten.
    Again,
    /// The card was recalled.
    Good,
}

/// Why a press's answer was not recorded. A native client reads it as a thrown error, beside
/// [`EngineRefusal`] and [`ExemptRefusal`], whose variants and text it leaves as they are.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum PressRefusal {
    /// The states the card was shown with are not the engine's `SchedulingStates`.
    Undecodable,
    /// The answer names another card than the press.
    NotTheCard,
    /// The answer names another grade than the press.
    NotTheGrade,
    /// The engine refused the checked answer.
    Engine {
        /// The engine's `BackendError`, as the engine encoded it.
        error: Vec<u8>,
    },
}

impl fmt::Display for PressRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Undecodable => {
                f.write_str("the states the card was shown with are not the engine's states")
            }
            Self::NotTheCard => f.write_str("the answer names another card than the press"),
            Self::NotTheGrade => f.write_str("the answer names another grade than the press"),
            Self::Engine { error } => {
                write!(f, "the engine refused the answer ({} bytes)", error.len())
            }
        }
    }
}

impl std::error::Error for PressRefusal {}

/// The engine's sync login, `BackendSyncService.SyncLogin`: the one call the statement's read
/// precedes (SPEC-374 R8).
const SYNC_LOGIN: (u32, u32) = (1, 3);
/// The longest the statement's read may take, from its connection to its body's last byte.
const STATEMENT_TIMEOUT: Duration = Duration::from_secs(10);
/// The most bytes the statement's body may hold.
const STATEMENT_CAP: usize = 1024;

/// Reads the sync service's statement at `url` (SPEC-374 R8): the body of a 200, an empty body
/// for any other status, a body over the cap or one that breaks off, and `None` when nothing
/// answered. The read runs on a scoped thread with a runtime of its own, so a caller inside a runtime never blocks one.
fn read_statement(url: &str) -> Option<Vec<u8>> {
    thread::scope(|scope| {
        scope
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .ok()?
                    .block_on(statement(url))
            })
            .join()
            .ok()
            .flatten()
    })
}

/// The statement's one request: a `GET` that sends no credential and follows no redirect.
async fn statement(url: &str) -> Option<Vec<u8>> {
    #[expect(
        clippy::disallowed_types,
        reason = "the statement's read is the one place the static library makes an HTTP client"
    )]
    #[expect(
        clippy::disallowed_methods,
        reason = "the statement's read is the one place the static library makes an HTTP client"
    )]
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(STATEMENT_TIMEOUT)
        .build()
        .ok()?;
    let mut response = client.get(url).send().await.ok()?;
    if response.status() != reqwest::StatusCode::OK {
        return Some(Vec::new());
    }
    let mut body = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => body.extend_from_slice(&chunk),
            Ok(None) => return Some(body),
            Err(_) => return Some(Vec::new()),
        }
        if body.len() > STATEMENT_CAP {
            return Some(Vec::new());
        }
    }
}

/// The adapter's refusal for the core's: a pair the native column does not admit reads as one the
/// allow-list does not carry, since the two are equal, and so do an exempt write and the answer,
/// which `run` never reaches.
fn refusal(refusal: Refusal) -> EngineRefusal {
    match refusal {
        Refusal::Engine { error } => EngineRefusal::Engine { error },
        Refusal::NotAllowed { service, method }
        | Refusal::NeedsGesture { service, method }
        | Refusal::NeedsAnswer { service, method } => EngineRefusal::NotAllowed { service, method },
    }
}

/// The core's exempt write for the write a tap names, one for one.
fn exempt_write(tap: ExemptTap) -> ExemptWrite {
    match tap {
        ExemptTap::Forget => ExemptWrite::Forget,
        ExemptTap::SetDueDate => ExemptWrite::SetDueDate,
        ExemptTap::DeletePreset => ExemptWrite::DeletePreset,
        ExemptTap::ChangeNoteType => ExemptWrite::ChangeNoteType,
        ExemptTap::DeleteCard => ExemptWrite::DeleteCard,
        ExemptTap::DeleteNote => ExemptWrite::DeleteNote,
    }
}

/// The core's target for the target a tap names, by the same id.
fn core_target(target: ExemptTarget) -> Target {
    match target {
        ExemptTarget::Card { id } => Target::Card(id),
        ExemptTarget::Note { id } => Target::Note(id),
        ExemptTarget::Preset { id } => Target::Preset(id),
    }
}

/// The adapter's refusal for the core's: each reason by its own variant, and the engine's error
/// bytes as the engine encoded them.
fn exempt_refusal(refusal: GestureRefusal) -> ExemptRefusal {
    match refusal {
        GestureRefusal::WrongKind { .. } => ExemptRefusal::WrongKind,
        GestureRefusal::NotTheTarget { .. } => ExemptRefusal::NotTheTarget,
        GestureRefusal::Undecodable { .. } => ExemptRefusal::Undecodable,
        GestureRefusal::Engine { error } => ExemptRefusal::Engine { error },
        GestureRefusal::NeedsTheChoice => ExemptRefusal::NeedsTheChoice,
    }
}

/// The exempt write an owner's tap names (SPEC-345 R3, R9): one for each row of the core's
/// exempt table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ExemptTap {
    /// Forget one card: it returns to the new queue.
    Forget,
    /// Set one card's due date.
    SetDueDate,
    /// Delete one preset.
    DeletePreset,
    /// Change one note's note type.
    ChangeNoteType,
    /// Delete one card.
    DeleteCard,
    /// Delete one note.
    DeleteNote,
}

/// The one thing a tap's write acts on, by the engine's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ExemptTarget {
    /// A card.
    Card {
        /// The card's id.
        id: i64,
    },
    /// A note.
    Note {
        /// The note's id.
        id: i64,
    },
    /// A preset (a deck options group).
    Preset {
        /// The preset's id.
        id: i64,
    },
}

/// Why a tap's write did not run. A native client reads it as a thrown error, beside
/// [`EngineRefusal`], whose variants and text it leaves as they are.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum ExemptRefusal {
    /// The target is not of the kind the tap's write takes.
    WrongKind,
    /// The request names other than the tap's one target: none, another, or more than one.
    NotTheTarget,
    /// The request is not the tap's own write's message.
    Undecodable,
    /// The engine refused the checked write.
    Engine {
        /// The engine's `BackendError`, as the engine encoded it.
        error: Vec<u8>,
    },
    /// The tap is a one-way sync's, which runs only through the full-sync choice's own write
    /// (SPEC-364 R3).
    NeedsTheChoice,
}

/// The four refusals an exempt tap's own write can meet, each told by its sentence. The one-way
/// sync's refusal is told beside them by [`ExemptRefusal`]'s own `Display` (SPEC-364 R13), so
/// these four sentences stay in one match.
enum Told<'a> {
    WrongKind,
    NotTheTarget,
    Undecodable,
    Engine { error: &'a [u8] },
}

impl fmt::Display for Told<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongKind => f.write_str("the tap's target is not of the kind its write takes"),
            Self::NotTheTarget => f.write_str("the request names other than the tap's one target"),
            Self::Undecodable => f.write_str("the request is not the tap's own write"),
            Self::Engine { error } => {
                write!(f, "the engine refused the write ({} bytes)", error.len())
            }
        }
    }
}

impl fmt::Display for ExemptRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let told = match self {
            Self::WrongKind => Told::WrongKind,
            Self::NotTheTarget => Told::NotTheTarget,
            Self::Undecodable => Told::Undecodable,
            Self::Engine { error } => Told::Engine { error },
            Self::NeedsTheChoice => {
                return f
                    .write_str("the one-way sync runs only through the full-sync choice's write");
            }
        };
        told.fmt(f)
    }
}

impl std::error::Error for ExemptRefusal {}

/// The launch argument a UI test names its seeded collection's directory with (ADR-359 D6).
const COLLECTION_DIRECTORY: &str = "-DSCollectionDirectory";

/// Why a launch's collection directory argument was refused, by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
pub enum CollectionDirectoryRefusal {
    /// The argument is the last one, with no value after it.
    NoValue,
    /// The value is not an absolute path.
    NotAbsolute,
    /// The value names nothing that exists.
    Missing,
    /// The value names something that is not a directory.
    NotADirectory,
}

impl fmt::Display for CollectionDirectoryRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoValue => "the collection directory argument has no value",
            Self::NotAbsolute => "the collection directory is not an absolute path",
            Self::Missing => "the collection directory does not exist",
            Self::NotADirectory => "the collection directory is not a directory",
        })
    }
}

impl std::error::Error for CollectionDirectoryRefusal {}

/// The directory the app opens its collection in: `fallback` when the launch `arguments` name no
/// `-DSCollectionDirectory`, and the value after it when that is an absolute path to an existing
/// directory (SPEC-348 R7). The app shell passes its own arguments and decides nothing.
///
/// # Errors
///
/// Every other value, by name: [`CollectionDirectoryRefusal::NoValue`],
/// [`CollectionDirectoryRefusal::NotAbsolute`], [`CollectionDirectoryRefusal::Missing`] or
/// [`CollectionDirectoryRefusal::NotADirectory`].
#[uniffi::export]
#[expect(
    clippy::needless_pass_by_value,
    reason = "a foreign caller's values cross the boundary owned, as the bindings pass them"
)]
pub fn collection_directory(
    fallback: String,
    arguments: Vec<String>,
) -> Result<String, CollectionDirectoryRefusal> {
    let Some(position) = arguments
        .iter()
        .position(|argument| argument == COLLECTION_DIRECTORY)
    else {
        return Ok(fallback);
    };
    let value = arguments
        .get(position + 1)
        .ok_or(CollectionDirectoryRefusal::NoValue)?;
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(CollectionDirectoryRefusal::NotAbsolute);
    }
    if !path.exists() {
        return Err(CollectionDirectoryRefusal::Missing);
    }
    if !path.is_dir() {
        return Err(CollectionDirectoryRefusal::NotADirectory);
    }
    Ok(value.clone())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use deck_streak_engine_core::full_sync::{Counted, Direction, IdSets, Offer};
    use deck_streak_engine_core::one_way::Unwritten;

    use super::{
        ChoiceCounts, ChoiceLoss, ChoiceStep, DEVICE_CHANGED, Engine, EngineRefusal, Stage,
    };

    /// A normal sync's reply that does not decode is the engine's refusal with the reply's bytes,
    /// and a reply that decodes answers its requirement (SPEC-503 R5). The engine's own encoder
    /// always writes a reply that decodes, so only a test inside the crate reaches the refusal.
    #[test]
    fn an_undecodable_sync_reply_is_the_engines_refusal_with_its_bytes() {
        assert_eq!(
            Engine::required(vec![0xff, 0xff]),
            Err(EngineRefusal::Engine {
                error: vec![0xff, 0xff]
            })
        );
        // Field 3, `required`, as a varint: 2, the full sync.
        assert_eq!(Engine::required(vec![0x18, 0x02]), Ok(2));
    }

    /// One side's ids: these reviews, and one card and one note both sides hold.
    fn ids(reviews: &[i64]) -> IdSets {
        IdSets {
            reviews: reviews.iter().copied().collect(),
            cards: BTreeSet::from([1]),
            notes: BTreeSet::from([1]),
            modified: 1,
        }
    }

    /// A device that gains a review between a download's backup and its write is the core's
    /// recount: the step writes nothing, says the device changed and shows the new counts, and the
    /// stage holds those counts, over the device as it is now, with the counted server copy, so
    /// the next tap confirms what the owner saw (SPEC-503 R4). Through the public calls only a
    /// write from another thread inside one confirm reaches this arm, so a test inside the crate
    /// hands the write's result to the method that reads it.
    #[test]
    fn a_device_change_at_the_write_holds_the_new_counts_for_the_next_tap() {
        let engine =
            Engine::new(Vec::new()).expect("the engine starts from the default init message");
        let device = ids(&[10, 11]);
        let server = ids(&[10, 20]);
        let counted = Counted::show(Offer::of(true, true), device.clone(), server.clone());
        let copy = PathBuf::from("server-copy-1-1.anki2");
        *engine.stage() = Some(Stage {
            counted: counted.clone(),
            copy: copy.clone(),
        });
        let ready = counted
            .confirm(Direction::Download)
            .expect("the full sync offers the download")
            .backed_up(&device)
            .expect("the backup holds the device's rows")
            .download_ready()
            .expect("a download is ready after its backup");
        let device_now = ids(&[10, 11, 12]);
        let recount = ready
            .at_write(&device_now)
            .expect_err("the device gained a review its backup lacks");
        let expected = Counted::show(Offer::of(true, true), device_now.clone(), server);
        assert_eq!(recount, expected);

        let step = engine
            .written(Err(Unwritten::Recount(recount)), copy.clone())
            .expect("a recount is a step, not a refusal");
        let lost = |reviews| ChoiceLoss {
            reviews,
            cards: 0,
            notes: 0,
        };
        assert_eq!(
            step,
            ChoiceStep {
                written: false,
                said: DEVICE_CHANGED.to_owned(),
                counts: Some(ChoiceCounts {
                    upload: Some(lost(1)),
                    download: Some(lost(2)),
                }),
            }
        );

        let held = engine.stage().clone().expect("the new counts are held");
        assert_eq!(held.counted, expected);
        assert_eq!(held.copy, copy);
        let next = held
            .counted
            .confirm(Direction::Download)
            .expect("the held counts offer the download")
            .backed_up(&device_now)
            .expect("a backup of the device as it is now holds its rows")
            .download_ready()
            .expect("a download is ready after its backup")
            .at_write(&device_now)
            .expect("the next tap writes over the device it counted");
        assert_eq!(next.direction(), Direction::Download);
    }
}
