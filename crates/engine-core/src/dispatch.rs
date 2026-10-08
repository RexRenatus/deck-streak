//! The dispatcher: the engine's backend, held privately, behind the table (SPEC-345 R1, R2, R4).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use anki::backend::{Backend, init_backend};
use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use anki_proto::collection::{CloseCollectionRequest, OpenCollectionRequest};
use anki_proto::sync::{FullUploadOrDownloadRequest, SyncAuth};
use prost::Message;

use crate::face::{self, Face, Side};
use crate::full_sync::{IdSets, Unsynced, Write};
use crate::gesture::{GestureRefusal, OwnerGesture};
use crate::login_guard;
use crate::media::Reader;
use crate::one_way;
use crate::table::{Decision, ExemptWrite, Transport, decide};

/// The one read of a card the page may make: its scheduling fields, by id (moved from the web
/// engine, which passed it to the engine's database door itself).
const SNAPSHOT_SQL: &str = "select id, queue, type, due, ivl, reps, lapses from cards where id = ?";
/// The note count the web engine's `open` reports.
const NOTE_COUNT_SQL: &str = "select count() from notes";
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
        })
    }

    /// Runs one ordinary call: the request's protobuf bytes in, the response's out. The table
    /// decides before the engine sees the call, so a pair it does not admit never reaches the
    /// engine's dispatch, whatever it would have done there.
    ///
    /// # Errors
    ///
    /// [`Refusal::NeedsGesture`] for an exempt write, [`Refusal::NotAllowed`] for every other pair
    /// this transport may not make, and [`Refusal::Engine`] when the engine answers an admitted
    /// call with an error, or when the login guard refuses a sync login's or a normal sync's
    /// endpoint, or a normal sync's media, in the engine's own error shape before the engine sees
    /// it (SPEC-347 R2, SPEC-364 R2). When the engine opens a collection, the core keeps the media
    /// folder and the collection path its request named (SPEC-348 R5, SPEC-364 R4).
    pub fn run(&self, service: u32, method: u32, input: &[u8]) -> Result<Vec<u8>, Refusal> {
        match decide(self.transport, service, method) {
            Decision::Admit => {
                if (service, method) == SYNC_LOGIN {
                    login_guard::check(input).map_err(|error| Refusal::Engine { error })?;
                }
                if (service, method) == SYNC_COLLECTION {
                    login_guard::check_sync(input).map_err(|error| Refusal::Engine { error })?;
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
        let (service, method, request) = gesture.checked(input)?;
        self.backend
            .run_service_method(service, method, &request)
            .map_err(|error| GestureRefusal::Engine { error })
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
        let engine = Self::start(Transport::Native, &self.start)
            .map_err(|message| failed(&format!("the private engine does not start: {message}")))?;
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
    /// auth meets the login's endpoint rule first (SPEC-364 R2). Its error is the engine's
    /// encoded `BackendError`, or the guard's in the same shape.
    pub(crate) fn full_sync(&self, request: &FullUploadOrDownloadRequest) -> Result<(), Vec<u8>> {
        login_guard::check_auth(&request.auth.clone().unwrap_or_default())?;
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

    /// Whether `path` names the collection this engine has open: as its open request named it, or
    /// as the same file reached by another spelling.
    pub(crate) fn opens(&self, path: &Path) -> bool {
        let open = self
            .open
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        open.is_some_and(|open| {
            open == path
                || matches!(
                    (open.canonicalize(), path.canonicalize()),
                    (Ok(open), Ok(path)) if open == path
                )
        })
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

    /// Runs one fixed read of the open collection and returns the engine's JSON reply: its first
    /// row only, as `[[column, ...]]`, or `[]` when no row matches.
    ///
    /// # Errors
    ///
    /// [`Refusal::Engine`] when the engine cannot run the read, a closed collection among them.
    pub fn read(&self, read: Read) -> Result<Vec<u8>, Refusal> {
        let (sql, args) = match read {
            Read::NoteCount => (NOTE_COUNT_SQL, Vec::new()),
            Read::CardSnapshot(card) => (SNAPSHOT_SQL, vec![serde_json::Value::from(card)]),
        };
        let request = serde_json::json!({
            "kind": "query",
            "sql": sql,
            "args": args,
            "first_row_only": true,
        });
        self.backend
            .run_db_command_bytes(request.to_string().as_bytes())
            .map_err(|error| Refusal::Engine { error })
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
fn failed(message: &str) -> Refusal {
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
        error: BackendError {
            message: format!("the engine's reply to `{sql}` is not the integers it selects"),
            kind: Kind::DbError.into(),
            ..BackendError::default()
        }
        .encode_to_vec(),
    }
}
