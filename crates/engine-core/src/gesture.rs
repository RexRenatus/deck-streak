//! The owner's gesture: one exempt write and the one target it names (SPEC-345 R7, R8; ADR-356 D5).
//!
//! An exempt write runs only through [`crate::dispatch::Dispatcher::run_exempt`], which consumes
//! an [`OwnerGesture`]. The gesture's fields are private, it is neither `Clone` nor `Copy`, and
//! [`OwnerGesture::from_tap`] is its one constructor, so one tap reaches at most one write. Which
//! crates may name the constructor is the crate graph's to decide, and the containment census
//! holds it to the two UI adapters' entry files.

use std::fmt;

use anki_proto::cards::RemoveCardsRequest;
use anki_proto::deck_config::DeckConfigId;
use anki_proto::notes::RemoveNotesRequest;
use anki_proto::notetypes::ChangeNotetypeRequest;
use anki_proto::scheduler::{ScheduleCardsAsNewRequest, SetDueDateRequest};
use prost::Message;

use crate::table::{EXEMPT, ExemptWrite, TargetKind};
use crate::undo_answer::Recorded;

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

/// What a checked gesture runs: an engine call with its checked request, or an undo of the review's
/// own last answer, whose record the dispatcher judges against the engine's state before the
/// engine runs it (SPEC-371 R5).
#[derive(Debug)]
pub(crate) enum Checked {
    /// The service, the method and the request the engine runs.
    Run(u32, u32, Vec<u8>),
    /// An undo on `card` of the answer `recorded` names.
    Undo {
        /// The card the gesture names.
        card: i64,
        /// The record of the answer to undo.
        recorded: Recorded,
    },
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
        if EXEMPT
            .iter()
            .any(|row| row.write == write && row.kind == target.kind())
        {
            Ok(Self { write, target })
        } else {
            Err(GestureRefusal::WrongKind { write, target })
        }
    }

    /// The service, the method and the request the engine runs for this gesture: `input` decoded
    /// as the write's own message, checked against the one target, and encoded again. An undo's
    /// `input` is the answer's record, decoded for the dispatcher to judge.
    pub(crate) fn checked(self, input: &[u8]) -> Result<Checked, GestureRefusal> {
        let Self { write, target } = self;
        let (Target::Card(id) | Target::Note(id) | Target::Preset(id)) = target;
        let Some(row) = EXEMPT.iter().find(|row| row.write == write) else {
            return Err(GestureRefusal::WrongKind { write, target });
        };
        let undecodable = |_| GestureRefusal::Undecodable { write };
        // Each write's request is decoded as its own message and judged on the ids it names: the
        // one target, alone. The engine then runs the checked message, encoded again.
        let (names_only_the_target, request) = match write {
            ExemptWrite::Forget => {
                let request = ScheduleCardsAsNewRequest::decode(input).map_err(undecodable)?;
                (only(&request.card_ids, id), request.encode_to_vec())
            }
            ExemptWrite::SetDueDate => {
                let request = SetDueDateRequest::decode(input).map_err(undecodable)?;
                (only(&request.card_ids, id), request.encode_to_vec())
            }
            ExemptWrite::DeletePreset => {
                let request = DeckConfigId::decode(input).map_err(undecodable)?;
                (request.dcid == id, request.encode_to_vec())
            }
            ExemptWrite::ChangeNoteType => {
                let request = ChangeNotetypeRequest::decode(input).map_err(undecodable)?;
                (only(&request.note_ids, id), request.encode_to_vec())
            }
            ExemptWrite::DeleteCard => {
                let request = RemoveCardsRequest::decode(input).map_err(undecodable)?;
                (only(&request.card_ids, id), request.encode_to_vec())
            }
            ExemptWrite::Undo => {
                let recorded = Recorded::decode(input).unwrap_or_default();
                return Ok(Checked::Undo { card: id, recorded });
            }
            ExemptWrite::DeleteNote => {
                let request = RemoveNotesRequest::decode(input).map_err(undecodable)?;
                (
                    only(&request.note_ids, id) && request.card_ids.is_empty(),
                    request.encode_to_vec(),
                )
            }
        };
        if names_only_the_target {
            Ok(Checked::Run(row.service, row.method, request))
        } else {
            Err(GestureRefusal::NotTheTarget { write, target })
        }
    }
}

/// Whether a request's ids name the one target and nothing else.
fn only(ids: &[i64], id: i64) -> bool {
    ids == [id]
}
