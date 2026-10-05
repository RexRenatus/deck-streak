//! The dispatcher: the engine's backend, held privately, behind the table (SPEC-345 R1, R2, R4).

use anki::backend::{Backend, init_backend};

use crate::login_guard;
use crate::table::{Decision, Transport, decide};

/// The one read of a card the page may make: its scheduling fields, by id (moved from the web
/// engine, which passed it to the engine's database door itself).
const SNAPSHOT_SQL: &str = "select id, queue, type, due, ivl, reps, lapses from cards where id = ?";
/// The note count the web engine's `open` reports.
const NOTE_COUNT_SQL: &str = "select count() from notes";
/// The engine's sync login, `BackendSyncService.SyncLogin`: the one admitted call whose request
/// the core reads, to guard its endpoint (SPEC-347 R2).
const SYNC_LOGIN: (u32, u32) = (1, 3);

/// One running engine on one transport. An adapter starts one and reaches the engine only through
/// it; the backend is never handed out.
#[derive(Clone)]
pub struct Dispatcher {
    backend: Backend,
    transport: Transport,
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
        init_backend(message).map(|backend| Self { backend, transport })
    }

    /// Runs one ordinary call: the request's protobuf bytes in, the response's out. The table
    /// decides before the engine sees the call, so a pair it does not admit never reaches the
    /// engine's dispatch, whatever it would have done there.
    ///
    /// # Errors
    ///
    /// [`Refusal::NeedsGesture`] for an exempt write, [`Refusal::NotAllowed`] for every other pair
    /// this transport may not make, and [`Refusal::Engine`] when the engine answers an admitted
    /// call with an error, or when the login guard refuses a sync login's endpoint in the
    /// engine's own error shape before the engine sees it (SPEC-347 R2).
    pub fn run(&self, service: u32, method: u32, input: &[u8]) -> Result<Vec<u8>, Refusal> {
        match decide(self.transport, service, method) {
            Decision::Admit => {
                if (service, method) == SYNC_LOGIN {
                    login_guard::check(input).map_err(|error| Refusal::Engine { error })?;
                }
                self.backend
                    .run_service_method(service, method, input)
                    .map_err(|error| Refusal::Engine { error })
            }
            Decision::NeedsGesture => Err(Refusal::NeedsGesture { service, method }),
            Decision::NotAllowed => Err(Refusal::NotAllowed { service, method }),
        }
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
}
