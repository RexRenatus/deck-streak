//! The dispatcher: the engine's backend, held privately, behind the table (SPEC-345 R1, R2, R4).

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use anki::backend::{Backend, init_backend};
use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::collection::{OpenCollectionRequest, UndoStatus};
use prost::Message;

use crate::answer::{AnswerRefusal, OwnerAnswer};
use crate::face::{self, Face, Side};
use crate::full_sync::{IdSets, Unsynced};
use crate::gesture::{Checked, GestureRefusal, OwnerGesture, Target};
use crate::login_guard;
use crate::media::Reader;
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
/// The engine's sync login, `BackendSyncService.SyncLogin`: the one admitted call whose request
/// the core reads, to guard its endpoint (SPEC-347 R2).
const SYNC_LOGIN: (u32, u32) = (1, 3);
/// The engine's collection open, `BackendCollectionService.OpenCollection`: the one admitted call
/// whose request the core keeps a field of, the media folder a face reads (SPEC-348 R5).
const OPEN_COLLECTION: (u32, u32) = (3, 0);
/// Every review-log id: the reviews a full sync can lose (SPEC-357 R5, R9).
const REVIEW_IDS_SQL: &str = "select id from revlog";
/// Every card id.
const CARD_IDS_SQL: &str = "select id from cards";
/// Every note id.
const NOTE_IDS_SQL: &str = "select id from notes";
/// The collection's modified stamp, which an upload's re-check compares (SPEC-357 R7).
const MODIFIED_SQL: &str = "select mod from col";
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
        })
    }

    /// Runs one ordinary call: the request's protobuf bytes in, the response's out. The table
    /// decides before the engine sees the call, so a pair it does not admit never reaches the
    /// engine's dispatch, whatever it would have done there.
    ///
    /// # Errors
    ///
    /// [`Refusal::NeedsGesture`] for an exempt write, [`Refusal::NeedsAnswer`] for the call that
    /// records a grade, [`Refusal::NotAllowed`] for every other pair this transport may not make,
    /// and [`Refusal::Engine`] when the engine answers an admitted
    /// call with an error, or when the login guard refuses a sync login's endpoint in the
    /// engine's own error shape before the engine sees it (SPEC-347 R2). When the engine opens a
    /// collection, the core keeps the media folder its request named (SPEC-348 R5).
    pub fn run(&self, service: u32, method: u32, input: &[u8]) -> Result<Vec<u8>, Refusal> {
        match decide(self.transport, service, method) {
            Decision::Admit => {
                if (service, method) == SYNC_LOGIN {
                    login_guard::check(input).map_err(|error| Refusal::Engine { error })?;
                }
                let reply = self
                    .backend
                    .run_service_method(service, method, input)
                    .map_err(|error| Refusal::Engine { error })?;
                if (service, method) == OPEN_COLLECTION {
                    self.keep_media_folder(input);
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
    /// [`GestureRefusal::NotTheTarget`] when the request names other than the gesture's target,
    /// [`GestureRefusal::Undecodable`] when it is not the write's message, and
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

    /// Every review-log, card and note id of the open collection, and its modified stamp: what a
    /// full sync's counts, its backup check and its re-check compare (SPEC-357 R5, R9). Every row,
    /// by the core's fixed statements; no adapter passes SQL.
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
