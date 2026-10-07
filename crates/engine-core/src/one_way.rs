//! The one-way sync's steps, in the order the full-sync choice's model checks them, each reading
//! its side from its file (SPEC-364 R4 to R8, ADR-375 D3).
//!
//! An adapter names the paths and holds the stage between the owner's taps; it never passes ids.
//! Part a's types ([`crate::full_sync`]) hold the rule; this module makes the files the rule reads.

use std::path::Path;

use crate::dispatch::{Dispatcher, Refusal};
use crate::full_sync::{BackedUp, Confirmed};

/// Why a step stopped, with the state the owner's choice is kept at.
#[derive(Debug)]
pub struct Refused<S> {
    /// The state the choice stays at: the owner's tap stands, and the step can be tried again.
    pub state: S,
    /// Why the step stopped.
    pub reason: Reason,
}

/// What stopped a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// The engine refused a statement or a read: the dispatcher's refusal, in the engine's error
    /// shape.
    Engine(Refusal),
    /// The ids read from the file lack one of the side the write replaces.
    Unheld,
}

/// The backup of the side the write replaces, made durable and read back by the core before
/// anything destructive runs (SPEC-364 R5): for a download, the open collection written into
/// `backup`; for an upload, the counted server copy at `copy`. The ids [`Confirmed::backed_up`]
/// judges are read from the file, never passed by an adapter.
///
/// # Errors
///
/// [`Reason::Engine`] when the engine refuses the backup or a read of it, and [`Reason::Unheld`]
/// when the backup lacks an id of the replaced side; either way the confirmed choice is kept.
pub fn back_up(
    dispatcher: &Dispatcher,
    confirmed: Confirmed,
    _backup: &Path,
    _copy: &Path,
) -> Result<BackedUp, Refused<Confirmed>> {
    let device = match dispatcher.id_sets() {
        Ok(ids) => ids,
        Err(refusal) => {
            return Err(Refused {
                state: confirmed,
                reason: Reason::Engine(refusal),
            });
        }
    };
    confirmed.backed_up(&device).map_err(|state| Refused {
        state,
        reason: Reason::Unheld,
    })
}
