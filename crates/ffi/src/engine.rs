//! The engine handle, its typed refusal and the one entry point (SPEC-336 R1, R3).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use deck_streak_engine_core::answer::{
    AnswerRefusal, Grade, OwnerAnswer, answer_request, shown_states,
};
use deck_streak_engine_core::dispatch::{Dispatcher, Refusal};
use deck_streak_engine_core::face::Side;
use deck_streak_engine_core::gesture::{GestureRefusal, OwnerGesture, Target};
use deck_streak_engine_core::handshake;
use deck_streak_engine_core::review::{self, bury_of, bury_request, flag_request, toggled_red};
use deck_streak_engine_core::table::{ExemptWrite, Transport};

use crate::allow_list::allowed;
use crate::face::{CardFace, MediaFolder};

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
            .map(|dispatcher| Arc::new(Self { dispatcher }))
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
    /// the next card (SPEC-358 R3).
    ///
    /// # Errors
    ///
    /// [`EngineRefusal::Engine`] when the engine refuses the bury.
    pub fn bury(&self, card_id: i64) -> Result<(), EngineRefusal> {
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
