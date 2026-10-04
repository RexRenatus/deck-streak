//! The dispatcher: the engine's backend, held privately, behind the table (SPEC-345 R1, R2, R4).

use anki::backend::{Backend, init_backend};

use crate::table::Transport;

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
    pub fn start(transport: Transport, init: &[u8]) -> Result<Self, String> {
        init_backend(init).map(|backend| Self { backend, transport })
    }

    /// Runs one ordinary call: the request's protobuf bytes in, the response's out.
    ///
    /// # Errors
    ///
    /// Every call is refused until the table is filled.
    pub fn run(&self, service: u32, method: u32, _input: &[u8]) -> Result<Vec<u8>, Refusal> {
        let _ = (&self.backend, self.transport);
        Err(Refusal::NotAllowed { service, method })
    }

    /// Runs one fixed read and returns the engine's JSON reply.
    ///
    /// # Errors
    ///
    /// Every read is refused until the statements are moved here.
    pub fn read(&self, _read: Read) -> Result<Vec<u8>, Refusal> {
        Err(Refusal::Engine { error: Vec::new() })
    }
}
