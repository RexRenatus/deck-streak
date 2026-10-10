//! The dispatcher: the engine's backend, held privately, behind the table (SPEC-345 R1, R2, R4).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use anki::backend::{Backend, init_backend};
use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::collection::{CloseCollectionRequest, OpenCollectionRequest, UndoStatus};
use anki_proto::scheduler::SchedTimingTodayResponse;
use anki_proto::sync::{FullUploadOrDownloadRequest, SyncAuth};
use prost::Message;

use crate::answer::{AnswerRefusal, OwnerAnswer};
use crate::face::{self, Face, Side};
use crate::full_sync::{IdSets, Unsynced, Write};
use crate::gesture::{Checked, GestureRefusal, OwnerGesture, Target};
use crate::handshake::{self, Outcome};
use crate::late::EngineDay;
use crate::login_guard;
use crate::media::Reader;
use crate::one_way;
use crate::table::{ANSWERED, Decision, EXEMPT, ExemptWrite, Transport, decide};
use crate::undo_answer::{self, Recorded, Review};

/// The one read of a card the page may make: its scheduling fields, by id (moved from the web
/// engine, which passed it to the engine's database door itself).
const SNAPSHOT_SQL: &str = "select id, queue, type, due, ivl, reps, lapses from cards where id = ?";
/// The note count the web engine's `open` reports.
const NOTE_COUNT_SQL: &str = "select count() from notes";
/// The newest review-log row: its id and its card, the row an answer just wrote (SPEC-371 R4).
const NEWEST_REVIEW_SQL: &str = "select id, cid from revlog order by id desc limit 1";
/// One review-log row's card and sync mark, by its id (SPEC-371 R4).
const REVIEW_SQL: &str = "select cid, usn from revlog where id = ?";
/// The engine's undo status, `CollectionService.GetUndoStatus`: its label and its last step, which
/// an undo of the review's own last answer compares with its record (SPEC-371 R5).
const GET_UNDO_STATUS: (u32, u32) = (3, 7);
/// The engine's timing of today, `SchedulerService.SchedTimingToday`: the day count and the next
/// rollover a card's due is judged in (SPEC-376 R3). The core makes it itself; no adapter pair
/// names it.
const SCHED_TIMING_TODAY: (u32, u32) = (13, 5);
/// The engine's sync login, `BackendSyncService.SyncLogin`: the one admitted call whose request
/// the core reads, to guard its endpoint (SPEC-347 R2).
const SYNC_LOGIN: (u32, u32) = (1, 3);
/// The engine's collection open, `BackendCollectionService.OpenCollection`: the one admitted call
/// whose request the core keeps a field of, the media folder a face reads (SPEC-348 R5).
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// The engine's collection close, `BackendCollectionService.CloseCollection`: a private engine's
/// last call, so its file is whole for the next reader.
const CLOSE_COLLECTION: (u32, u32) = (3, 1);
/// The engine's one-way sync, `BackendSyncService.FullUploadOrDownload`: the choice's write, and
/// a private engine's fetch of a server copy (SPEC-364 R3, R4).
const FULL_SYNC: (u32, u32) = (1, 6);
/// The engine's normal sync, `BackendSyncService.SyncCollection`: admitted on the web, its
/// endpoint guarded and its media refused before the engine sees it (SPEC-364 R2).
const SYNC_COLLECTION: (u32, u32) = (1, 5);
/// The replay's one read (SPEC-386 R8): the review rows of every card whose home deck, the deck
/// it came from when it sits in a filtered deck, is in the deck set bound as a JSON array, each
/// with its card's type, by card and id. A row whose card no longer exists joins no card.
const HISTORY_SQL: &str = "SELECT r.cid, r.id, r.ease, r.type, r.factor, c.type \
    FROM revlog AS r JOIN cards AS c ON c.id = r.cid \
    WHERE (CASE WHEN c.odid != 0 THEN c.odid ELSE c.did END) \
    IN (SELECT value FROM json_each(?1)) ORDER BY r.cid, r.id";
/// Every review-log id: the reviews a full sync can lose (SPEC-357 R5, R9).
const REVIEW_IDS_SQL: &str = "select id from revlog";
/// Every card id.
const CARD_IDS_SQL: &str = "select id from cards";
/// Every note id.
const NOTE_IDS_SQL: &str = "select id from notes";
/// The upload re-check stamp, which an upload's re-check compares beside the ids (SPEC-364 R6,
/// ADR-375 D13, SPEC-357 R7 as amended): the engine's `fnvhash` of the greatest row usn over every
/// synced table that carries one, graves included, and the schema stamp. A normal sync and a full
/// upload move it; a download's restamp of `col.usn`, `col.mod` and `col.ls` leaves it still.
const MODIFIED_SQL: &str = "select fnvhash((select max(usn) from ( \
                            select max(usn) as usn from cards union all \
                            select max(usn) from notes union all \
                            select max(usn) from revlog union all \
                            select max(usn) from graves union all \
                            select max(usn) from decks union all \
                            select max(usn) from deck_config union all \
                            select max(usn) from notetypes union all \
                            select max(usn) from templates union all \
                            select max(usn) from tags union all \
                            select max(usn) from config)), \
                            (select scm from col))";
/// What a sync has not sent, in one statement: the reviews whose sequence number marks them
/// unsynced, and whether the collection or its schema changed since its last sync (SPEC-357 R9).
const UNSYNCED_SQL: &str = "select (select count() from revlog where usn = -1), \
                            (select mod > ls from col), (select scm > ls from col)";

/// One running engine on one transport. An adapter starts one and reaches the engine only through
/// it; the backend is never handed out.
#[derive(Clone)]
pub struct Dispatcher {
    backend: Backend,
    transport: Transport,
    /// The media folder the last successful open named, shared by every clone of this dispatcher.
    media_folder: Arc<Mutex<Option<String>>>,
    /// The start message this engine began from, which a private engine of the one-way sync
    /// starts from too, so it reads and writes in this engine's language and settings.
    start: Arc<[u8]>,
    /// The collection path the last successful open named, shared by every clone of this
    /// dispatcher: the file no server copy and no backup may be written into (SPEC-364 R4, R5).
    open: Arc<Mutex<Option<PathBuf>>>,
    /// What the latest statement of the service's minimum client level decided, shared by every
    /// clone of this dispatcher and by each private engine it starts (SPEC-374 R4, R7).
    handshake: Arc<Mutex<Outcome>>,
    /// Where the collection's files live, as the adapter installed it, shared by every clone of
    /// this dispatcher (SPEC-377 R4; ADR-388 D7).
    files: Arc<Mutex<Arc<dyn crate::files::Files>>>,
}

/// Why the dispatcher did not answer a call with the engine's reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The pair is not admitted on this transport, so the engine never saw it.
    NotAllowed {
        /// The service index the adapter sent.
        service: u32,
        /// The method index the adapter sent.
        method: u32,
    },
    /// The pair is an exempt write, which only an owner's gesture reaches; the engine never saw it.
    NeedsGesture {
        /// The service index the adapter sent.
        service: u32,
        /// The method index the adapter sent.
        method: u32,
    },
    /// The pair records a grade, which only an owner's answer reaches; the engine never saw it
    /// (SPEC-365 R5).
    NeedsAnswer {
        /// The service index the adapter sent.
        service: u32,
        /// The method index the adapter sent.
        method: u32,
    },
    /// The engine answered an admitted call with an error: its encoded `BackendError` message.
    Engine {
        /// The engine's error, as protobuf bytes.
        error: Vec<u8>,
    },
}

/// A read of the open collection whose statement the core holds. An adapter names one; it never
/// passes SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Read {
    /// The number of notes in the collection.
    NoteCount,
    /// One card's scheduling fields, by its id.
    CardSnapshot(i64),
    /// The newest review-log row's id and card (SPEC-371 R4).
    NewestReview,
    /// One review-log row's card and sync mark, by its id (SPEC-371 R4).
    Review(i64),
}

impl Dispatcher {
    /// Starts the engine from its encoded `BackendInit` message on `transport`.
    ///
    /// # Errors
    ///
    /// The engine's reason when it cannot decode the message.
    pub fn start(transport: Transport, message: &[u8]) -> Result<Self, String> {
        init_backend(message).map(|backend| Self {
            backend,
            transport,
            media_folder: Arc::default(),
            start: Arc::from(message),
            open: Arc::default(),
            handshake: Arc::default(),
            files: Arc::new(Mutex::new(crate::files::target_default())),
        })
    }

    /// Hands the dispatcher the latest statement of the service's minimum client level: its body,
    /// or `None` when no answer was read (SPEC-374 R4). Its outcome replaces the last one, so a
    /// later below or unread statement refuses again.
    pub fn handshake(&self, statement: Option<&[u8]>) {
        *self
            .handshake
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = handshake::decide(statement);
    }

    /// Whether the latest statement lets a sync call reach the engine (SPEC-374 R5): the core's
    /// refusal, in the login guard's shape, unless it was admitted.
    fn admitted(&self) -> Result<(), Vec<u8>> {
        handshake::admits(
            *self
                .handshake
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        )
    }

    /// Runs one ordinary call: the request's protobuf bytes in, the response's out. The table
    /// decides before the engine sees the call, so a pair it does not admit never reaches the
    /// engine's dispatch, whatever it would have done there.
    ///
    /// # Errors
    ///
    /// [`Refusal::NeedsGesture`] for an exempt write, [`Refusal::NeedsAnswer`] for the call that
    /// records a grade, [`Refusal::NotAllowed`] for every other pair this transport may not make,
    /// and [`Refusal::Engine`] when the engine answers an admitted call with an error, or when the
    /// login guard refuses a sync login's or a normal sync's endpoint, or a normal sync's media, in
    /// the engine's own error shape before the engine sees it (SPEC-347 R2, SPEC-364 R2), or when
    /// the latest statement of the service's minimum client level has not admitted this client,
    /// in the same shape, after the guard (SPEC-374 R5). When the engine opens a collection, the
    /// core keeps the media folder and the collection path its request named (SPEC-348 R5,
    /// SPEC-364 R4).
    pub fn run(&self, service: u32, method: u32, input: &[u8]) -> Result<Vec<u8>, Refusal> {
        match decide(self.transport, service, method) {
            Decision::Admit => {
                if (service, method) == SYNC_LOGIN {
                    login_guard::check(input).map_err(|error| Refusal::Engine { error })?;
                }
                if (service, method) == SYNC_COLLECTION {
                    login_guard::check_sync(input).map_err(|error| Refusal::Engine { error })?;
                }
                if matches!((service, method), SYNC_LOGIN | SYNC_COLLECTION) {
                    self.admitted().map_err(|error| Refusal::Engine { error })?;
                }
                let reply = self
                    .backend
                    .run_service_method(service, method, input)
                    .map_err(|error| Refusal::Engine { error })?;
                if (service, method) == OPEN_COLLECTION {
                    self.keep_media_folder(input);
                    self.keep_open_path(input);
                }
                Ok(reply)
            }
            Decision::NeedsGesture => Err(Refusal::NeedsGesture { service, method }),
            Decision::NeedsAnswer => Err(Refusal::NeedsAnswer { service, method }),
            Decision::NotAllowed => Err(Refusal::NotAllowed { service, method }),
        }
    }

    /// Runs the one exempt write an owner's gesture names, consuming the gesture (SPEC-345 R8).
    /// The request is decoded as the write's own message and checked against the gesture's one
    /// target, and the engine runs the message the check passed, encoded again, never the
    /// caller's bytes.
    ///
    /// # Errors
    ///
    /// [`GestureRefusal::NeedsTheChoice`] for a one-way sync's gesture, which only the full-sync
    /// choice's write runs, [`GestureRefusal::NotTheTarget`] when the request names other than the
    /// gesture's target, [`GestureRefusal::Undecodable`] when it is not the write's message, and
    /// [`GestureRefusal::Engine`] when the engine refuses the checked write.
    pub fn run_exempt(
        &self,
        gesture: OwnerGesture,
        input: &[u8],
    ) -> Result<Vec<u8>, GestureRefusal> {
        match gesture.checked(input)? {
            Checked::Run(service, method, request) => self
                .backend
                .run_service_method(service, method, &request)
                .map_err(|error| GestureRefusal::Engine { error }),
            Checked::Undo { card, recorded } => self.run_undo(card, &recorded),
        }
    }

    /// Runs the undo of the recorded answer on `card` (SPEC-371 R5): the engine's undo status and
    /// the recorded review row are read at the write, the rule judges them against the record, and
    /// only then does the engine undo, with an empty request. A refusal is the gesture's
    /// `NotTheTarget`: the record names other than an undo of the card's own last answer.
    fn run_undo(&self, card: i64, recorded: &Recorded) -> Result<Vec<u8>, GestureRefusal> {
        let target = Target::Card(card);
        let Some(row) = EXEMPT.iter().find(|row| row.write == ExemptWrite::Undo) else {
            return Err(GestureRefusal::WrongKind {
                write: ExemptWrite::Undo,
                target,
            });
        };
        let engine = |error| GestureRefusal::Engine { error };
        let refused = |_| GestureRefusal::NotTheTarget {
            write: ExemptWrite::Undo,
            target,
        };
        let status = self
            .backend
            .run_service_method(GET_UNDO_STATUS.0, GET_UNDO_STATUS.1, &[])
            .map_err(engine)?;
        let now = UndoStatus::decode(status.as_slice()).map_err(|_| engine(status.clone()))?;
        let review = self.review(recorded.review).map_err(engine)?;
        undo_answer::judge(recorded, &now, review, card).map_err(refused)?;
        self.backend
            .run_service_method(row.service, row.method, &[])
            .map_err(engine)
    }

    /// The review-log row `id`, by the core's fixed read, or `None` when the collection lacks it.
    /// An error is the engine's, encoded.
    fn review(&self, id: i64) -> Result<Option<Review>, Vec<u8>> {
        let reply = self.query(Read::Review(id))?;
        let rows: serde_json::Value =
            serde_json::from_slice(&reply).map_err(|_| unreadable_error(REVIEW_SQL))?;
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
            _ => Err(unreadable_error(REVIEW_SQL)),
        }
    }

    /// Runs the full-sync choice's one write: the one-way sync the owner confirmed, built by the
    /// core from the choice's [`Write`] and never from a caller's bytes (SPEC-364 R3). Crate-private:
    /// only [`crate::one_way::write`] reaches it, after the write's last check. It consumes a
    /// one-way sync's gesture and refuses a gesture of any other write.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "the gesture and the write are taken whole, so one tap makes one write"
    )]
    pub(crate) fn run_one_way(
        &self,
        gesture: OwnerGesture,
        write: Write,
        auth: &SyncAuth,
    ) -> Result<(), GestureRefusal> {
        let (named, target) = gesture.parts();
        if named != ExemptWrite::OneWaySync {
            return Err(GestureRefusal::WrongKind {
                write: named,
                target,
            });
        }
        self.full_sync(&one_way::request(&write, auth))
            .map_err(|error| GestureRefusal::Engine { error })
    }

    /// A private engine on `collection`: started from this engine's own start message, with the
    /// file open and no media folder, so it touches no media. It holds the file until
    /// [`Self::close`].
    pub(crate) fn private(&self, collection: &Path) -> Result<Self, Refusal> {
        let path = utf8(collection)?;
        let mut engine = Self::start(Transport::Native, &self.start)
            .map_err(|message| failed(&format!("the private engine does not start: {message}")))?;
        engine.handshake = Arc::clone(&self.handshake);
        let request = OpenCollectionRequest {
            collection_path: path.to_owned(),
            ..OpenCollectionRequest::default()
        };
        let (service, method) = OPEN_COLLECTION;
        engine.run(service, method, &request.encode_to_vec())?;
        Ok(engine)
    }

    /// Runs the engine's one-way sync with a request the core built: the choice's write on the
    /// open collection, or the fetch of a server copy into a private engine's empty file. Its
    /// auth meets the login's endpoint rule first (SPEC-364 R2), and then the latest statement of
    /// the service's minimum client level must have admitted this client (SPEC-374 R5). Its error
    /// is the engine's encoded `BackendError`, or the guard's or the handshake's in the same shape.
    pub(crate) fn full_sync(&self, request: &FullUploadOrDownloadRequest) -> Result<(), Vec<u8>> {
        login_guard::check_auth(&request.auth.clone().unwrap_or_default())?;
        self.admitted()?;
        let (service, method) = FULL_SYNC;
        self.backend
            .run_service_method(service, method, &request.encode_to_vec())
            .map(drop)
    }

    /// Closes a private engine's collection, so its file is whole and free for the next reader.
    pub(crate) fn close(&self) -> Result<(), Refusal> {
        let (service, method) = CLOSE_COLLECTION;
        let request = CloseCollectionRequest::default();
        self.backend
            .run_service_method(service, method, &request.encode_to_vec())
            .map(drop)
            .map_err(|error| Refusal::Engine { error })
    }

    /// Installs the adapter's `Files` port, which every clone of this dispatcher asks from now on
    /// (SPEC-377 R4; ADR-388 D7).
    pub fn install_files(&self, port: Arc<dyn crate::files::Files>) {
        *self.files.lock().unwrap_or_else(PoisonError::into_inner) = port;
    }

    /// Whether `path` names the collection this engine has open, however it is spelled, as the
    /// installed `Files` port answers (SPEC-377 R4; ADR-388 D7).
    pub(crate) fn opens(&self, path: &Path) -> bool {
        let open = self
            .open
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let port = self.port();
        open.is_some_and(|open| port.same(&open, path))
    }

    /// Whether `path` holds a file, as the installed `Files` port answers (SPEC-377 R4).
    pub(crate) fn holds(&self, path: &Path) -> bool {
        self.port().holds(path)
    }

    /// The installed `Files` port, taken out of its lock so no answer is given while it is held.
    fn port(&self) -> Arc<dyn crate::files::Files> {
        Arc::clone(&self.files.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Runs one of the core's fixed statements over the open collection, with `path` bound as its
    /// one parameter, through the engine's database door (SPEC-364 R5).
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the path is not UTF-8 or the engine cannot run the statement.
    pub(crate) fn execute(&self, sql: &'static str, path: &Path) -> Result<(), Refusal> {
        let request = serde_json::json!({
            "kind": "query",
            "sql": sql,
            "args": [utf8(path)?],
            "first_row_only": true,
        });
        self.backend
            .run_db_command_bytes(request.to_string().as_bytes())
            .map(drop)
            .map_err(|error| Refusal::Engine { error })
    }

    /// Records the grade one owner's press names, consuming the answer (SPEC-365 R3). The request
    /// is decoded as the engine's `CardAnswer` and checked against the press's card and grade, and
    /// the engine runs the message the check passed, encoded again, never the caller's bytes.
    ///
    /// # Errors
    ///
    /// [`AnswerRefusal::Undecodable`] when the request is not a `CardAnswer`,
    /// [`AnswerRefusal::NotTheCard`] when it names another card, [`AnswerRefusal::NotTheGrade`]
    /// when its rating is not the pressed grade's, and [`AnswerRefusal::Engine`] when the engine
    /// refuses the checked answer.
    pub fn run_answer(&self, answer: OwnerAnswer, input: &[u8]) -> Result<Vec<u8>, AnswerRefusal> {
        let request = answer.checked(input)?;
        let [row] = ANSWERED;
        self.backend
            .run_service_method(row.service, row.method, &request)
            .map_err(|error| AnswerRefusal::Engine { error })
    }

    /// Runs one fixed read of the open collection and returns the engine's JSON reply: its first
    /// row only, as `[[column, ...]]`, or `[]` when no row matches.
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the engine cannot run the read, a closed collection among them.
    pub fn read(&self, read: Read) -> Result<Vec<u8>, Refusal> {
        self.query(read).map_err(|error| Refusal::Engine { error })
    }

    /// One fixed read's JSON reply, or the engine's error, encoded.
    fn query(&self, read: Read) -> Result<Vec<u8>, Vec<u8>> {
        let (sql, args) = match read {
            Read::NoteCount => (NOTE_COUNT_SQL, Vec::new()),
            Read::CardSnapshot(card) => (SNAPSHOT_SQL, vec![serde_json::Value::from(card)]),
            Read::NewestReview => (NEWEST_REVIEW_SQL, Vec::new()),
            Read::Review(review) => (REVIEW_SQL, vec![serde_json::Value::from(review)]),
        };
        let request = serde_json::json!({
            "kind": "query",
            "sql": sql,
            "args": args,
            "first_row_only": true,
        });
        self.backend
            .run_db_command_bytes(request.to_string().as_bytes())
    }

    /// The review rows of every card whose home deck is in `decks`, each as `[card, id, ease,
    /// kind, factor, card type]`, by card and id: the replay's one read, by one fixed statement
    /// through the engine's database door (SPEC-386 R8). It writes nothing.
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the engine cannot run the read, a closed collection among them, or
    /// answers it with a row that is not the six integers the statement selects.
    pub(crate) fn history(&self, decks: &[i64]) -> Result<Vec<[i64; 6]>, Refusal> {
        let request = serde_json::json!({
            "kind": "query",
            "sql": HISTORY_SQL,
            "args": [serde_json::Value::from(decks.to_vec()).to_string()],
            "first_row_only": false
        });
        let reply = self
            .backend
            .run_db_command_bytes(request.to_string().as_bytes())
            .map_err(|error| Refusal::Engine { error })?;
        serde_json::from_slice(&reply).map_err(|_| unreadable(HISTORY_SQL))
    }

    /// Every review-log, card and note id of the open collection, and its upload re-check stamp:
    /// what a full sync's counts, its backup check and its re-check compare (SPEC-357 R5, R9;
    /// SPEC-364 R6). Every row, by the core's fixed statements; no adapter passes SQL.
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the engine cannot run a read, a closed collection among them, or
    /// answers one with a row that is not the integers its statement selects.
    pub fn id_sets(&self) -> Result<IdSets, Refusal> {
        let [modified] = self.one_row(MODIFIED_SQL)?[..] else {
            return Err(unreadable(MODIFIED_SQL));
        };
        Ok(IdSets {
            reviews: self.ids(REVIEW_IDS_SQL)?,
            cards: self.ids(CARD_IDS_SQL)?,
            notes: self.ids(NOTE_IDS_SQL)?,
            modified,
        })
    }

    /// What this device has not synced, by one fixed statement and with no network, so a client
    /// can warn offline (SPEC-357 R9): the reviews not yet synced, and whether the collection or
    /// its schema changed since its last sync.
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the engine cannot run the read, a closed collection among them, or
    /// answers it with a row that is not the three integers it selects.
    pub fn unsynced(&self) -> Result<Unsynced, Refusal> {
        let [reviews, changed, schema] = self.one_row(UNSYNCED_SQL)?[..] else {
            return Err(unreadable(UNSYNCED_SQL));
        };
        Ok(Unsynced {
            reviews: u32::try_from(reviews).map_err(|_| unreadable(UNSYNCED_SQL))?,
            changed: changed != 0,
            schema: schema != 0,
        })
    }

    /// The engine's day, read from the engine's own timing of today through one call the core
    /// holds and no adapter makes: the day count and the next rollover a card's due is judged in
    /// (SPEC-376 R3, ADR-387 D1).
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the engine cannot run the read, a closed collection among them, or
    /// answers it with a reply that is not the timing's message.
    pub fn engine_day(&self) -> Result<EngineDay, Refusal> {
        let reply = self
            .backend
            .run_service_method(SCHED_TIMING_TODAY.0, SCHED_TIMING_TODAY.1, &[])
            .map_err(|error| Refusal::Engine { error })?;
        let timing = SchedTimingTodayResponse::decode(reply.as_slice())
            .map_err(|_| failed("the engine's timing of today is not its message"))?;
        Ok(EngineDay {
            days_elapsed: timing.days_elapsed,
            next_day_at: timing.next_day_at,
        })
    }

    /// The ids one fixed statement selects, one per row.
    fn ids(&self, sql: &str) -> Result<BTreeSet<i64>, Refusal> {
        self.rows(sql)?
            .into_iter()
            .map(|row| match row[..] {
                [id] => Ok(id),
                _ => Err(unreadable(sql)),
            })
            .collect()
    }

    /// The one row a fixed statement over the collection's single row selects.
    fn one_row(&self, sql: &str) -> Result<Vec<i64>, Refusal> {
        let mut rows = self.rows(sql)?;
        match (rows.pop(), rows.is_empty()) {
            (Some(row), true) => Ok(row),
            _ => Err(unreadable(sql)),
        }
    }

    /// Runs one of the core's fixed statements and returns its rows, each cell an integer. The id
    /// reads and the unsynced read all come here, so every one of them reads every row.
    fn rows(&self, sql: &str) -> Result<Vec<Vec<i64>>, Refusal> {
        let request = serde_json::json!({
            "kind": "query",
            "sql": sql,
            "args": [],
            "first_row_only": false,
        });
        let reply = self
            .backend
            .run_db_command_bytes(request.to_string().as_bytes())
            .map_err(|error| Refusal::Engine { error })?;
        serde_json::from_slice(&reply).map_err(|_| unreadable(sql))
    }

    /// The media folder of the collection this engine last opened, as its open request named it;
    /// `None` before an open succeeds, or when the request named none. The native adapter reads a
    /// face's media from it (SPEC-348 R5).
    #[must_use]
    pub fn media_folder(&self) -> Option<String> {
        self.media_folder
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Keeps the media folder a successful open's request named, replacing the last one.
    fn keep_media_folder(&self, input: &[u8]) {
        let folder = OpenCollectionRequest::decode(input)
            .ok()
            .map(|request| request.media_folder_path)
            .filter(|path| !path.is_empty());
        *self
            .media_folder
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = folder;
    }

    /// Keeps the collection path a successful open's request named, replacing the last one.
    fn keep_open_path(&self, input: &[u8]) {
        let path = OpenCollectionRequest::decode(input)
            .ok()
            .map(|request| request.collection_path)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from);
        *self.open.lock().unwrap_or_else(PoisonError::into_inner) = path;
    }

    /// Completes `card`'s face for `side`, as the engine's own reviewer completes it, reading its
    /// media through `media` (SPEC-348 R2). `autoplay` is the client's wish.
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the engine cannot read or render the card.
    pub fn face(
        &self,
        card: i64,
        side: Side,
        autoplay: bool,
        media: &dyn Reader,
    ) -> Result<Face, Refusal> {
        face::complete(&self.backend, card, side, autoplay, media)
    }
}

/// `path` as the engine's open request names it: a path that is not UTF-8 is refused, never
/// rewritten into another path.
fn utf8(path: &Path) -> Result<&str, Refusal> {
    path.to_str()
        .ok_or_else(|| failed("the one-way sync's path is not UTF-8"))
}

/// A refusal of the core's own, in the engine's error shape: an encoded `BackendError` of kind
/// `INVALID_INPUT` whose message names what failed.
pub(crate) fn failed(message: &str) -> Refusal {
    Refusal::Engine {
        error: BackendError {
            message: message.to_owned(),
            kind: Kind::InvalidInput.into(),
            ..BackendError::default()
        }
        .encode_to_vec(),
    }
}

/// The refusal for a fixed read whose reply is not the integers its statement selects, in the
/// engine's own error shape: an encoded `BackendError` of kind `DB_ERROR` naming the statement, so
/// an adapter reads it as it reads any engine error and never a guessed value.
fn unreadable(sql: &str) -> Refusal {
    Refusal::Engine {
        error: unreadable_error(sql),
    }
}

/// The engine's error for a fixed read whose reply is not what its statement selects, encoded.
fn unreadable_error(sql: &str) -> Vec<u8> {
    BackendError {
        message: format!("the engine's reply to `{sql}` is not the integers it selects"),
        kind: Kind::DbError.into(),
        ..BackendError::default()
    }
    .encode_to_vec()
}

#[cfg(test)]
mod tests {
    use anki_proto::backend::BackendError;
    use anki_proto::backend::backend_error::Kind;
    use prost::Message;

    use super::{Dispatcher, Refusal};
    use crate::table::Transport;

    /// A close with no collection open reaches the engine and answers the engine's own refusal, so
    /// a private engine's close never reports a success the engine did not give. `close` is
    /// crate-private, so only a test inside the crate can call it with no collection open.
    #[test]
    fn a_close_with_no_collection_open_is_the_engines_refusal() {
        let engine = Dispatcher::start(Transport::Native, &[])
            .expect("the engine starts from the default init");

        let closed = engine.close().map_err(|refusal| match refusal {
            Refusal::Engine { error } => {
                let error = BackendError::decode(error.as_slice())
                    .expect("a refusal decodes as the engine's error");
                Some((error.kind(), error.message))
            }
            Refusal::NotAllowed { .. }
            | Refusal::NeedsGesture { .. }
            | Refusal::NeedsAnswer { .. } => None,
        });

        assert_eq!(
            closed,
            Err(Some((Kind::InvalidInput, "CollectionNotOpen".to_owned()))),
            "a close with no collection open is the engine's own refusal, its kind and message \
             byte for byte"
        );
    }
}
