//! The owner's gesture: one exempt write and the one target it names (SPEC-345 R7, R8; ADR-356 D5).
//!
//! An exempt write runs only through [`crate::dispatch::Dispatcher::run_exempt`], which consumes
//! an [`OwnerGesture`]. The gesture's fields are private, it is neither `Clone` nor `Copy`, and
//! [`OwnerGesture::from_tap`] is its one constructor, so one tap reaches at most one write. Which
//! crates may name the constructor is the crate graph's to decide, and the containment census
//! holds it to the two UI adapters' entry files.

use std::fmt;

use crate::table::{ExemptWrite, TargetKind};

/// The one thing an exempt write acts on, by the engine's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// A card.
    Card(i64),
    /// A note.
    Note(i64),
    /// A preset (a deck options group).
    Preset(i64),
}

impl Target {
    /// The kind of thing this target is, as the exempt table names the kind each write takes.
    #[must_use]
    pub fn kind(self) -> TargetKind {
        match self {
            Self::Card(_) => TargetKind::Card,
            Self::Note(_) => TargetKind::Note,
            Self::Preset(_) => TargetKind::Preset,
        }
    }
}

/// Why a gesture's write did not run: refused before the engine saw it, or by the engine itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GestureRefusal {
    /// The target is not of the kind the write takes.
    WrongKind {
        /// The write the tap named.
        write: ExemptWrite,
        /// The target the tap named, of another kind.
        target: Target,
    },
    /// The request names other than the gesture's one target: none, another, or more than one.
    NotTheTarget {
        /// The gesture's write.
        write: ExemptWrite,
        /// The gesture's one target.
        target: Target,
    },
    /// The request is not the write's own message.
    Undecodable {
        /// The gesture's write.
        write: ExemptWrite,
    },
    /// The engine refused the checked write.
    Engine {
        /// The engine's `BackendError`, as the engine encoded it.
        error: Vec<u8>,
    },
}

impl fmt::Display for GestureRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongKind { write, target } => {
                write!(f, "the {write:?} write does not take {target:?}")
            }
            Self::NotTheTarget { write, target } => {
                write!(f, "the {write:?} request names other than {target:?}")
            }
            Self::Undecodable { write } => {
                write!(f, "the {write:?} request is not its write's own message")
            }
            Self::Engine { error } => {
                write!(f, "the engine refused the write ({} bytes)", error.len())
            }
        }
    }
}

/// One owner's tap on one exempt write and its one target. Only the UI adapters build one, from
/// the tap itself, and running its write consumes it.
#[derive(Debug)]
pub struct OwnerGesture {
    write: ExemptWrite,
    target: Target,
}

impl OwnerGesture {
    /// The gesture of a tap on `write` naming `target`.
    ///
    /// # Errors
    ///
    /// [`GestureRefusal::WrongKind`] when `target` is not of the kind `write` takes.
    pub fn from_tap(write: ExemptWrite, target: Target) -> Result<Self, GestureRefusal> {
        Err(GestureRefusal::WrongKind { write, target })
    }

    /// The service, the method and the request the engine runs for this gesture: `input` decoded
    /// as the write's own message, checked against the one target, and encoded again.
    pub(crate) fn checked(self, _input: &[u8]) -> Result<(u32, u32, Vec<u8>), GestureRefusal> {
        Err(GestureRefusal::NotTheTarget {
            write: self.write,
            target: self.target,
        })
    }
}
