//! The engine handle, its typed refusal and the one entry point (SPEC-336 R1, R3).

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use deck_streak_engine_core::dispatch::{Dispatcher, Refusal};
use deck_streak_engine_core::face::Side;
use deck_streak_engine_core::table::Transport;

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

    /// Runs one allowed call: the request's protobuf bytes in, the response's out.
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
        self.dispatcher
            .run(service, method, &input)
            .map_err(refusal)
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
}

/// The adapter's refusal for the core's: a pair the native column does not admit reads as one the
/// allow-list does not carry, since the two are equal.
fn refusal(refusal: Refusal) -> EngineRefusal {
    match refusal {
        Refusal::Engine { error } => EngineRefusal::Engine { error },
        Refusal::NotAllowed { service, method } | Refusal::NeedsGesture { service, method } => {
            EngineRefusal::NotAllowed { service, method }
        }
    }
}
