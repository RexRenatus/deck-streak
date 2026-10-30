//! Write-once bindings of names to hashes (SPEC-513 R3; ADR-531 D3).
//!
//! A name is bound to one hash, once. Binding it again to the same hash changes nothing, and to
//! another hash is refused. A sweep releases a binding when it deletes the file, and the released
//! binding keeps its hash: the name comes back only for the same bytes, so no name ever means two
//! files. [`step`] and [`release_step`] are the machine over one name's slot; [`Bindings`] holds a
//! slot per name.

use std::collections::BTreeMap;
use std::fmt;

use crate::hash::MediaHash;
use crate::name::MediaName;

/// Whether a bound name's file is held or was released by a sweep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindState {
    /// The name's file is held.
    Live,
    /// A sweep deleted the name's file; the name keeps its hash.
    Released,
}

/// One name's binding: its hash and its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    /// The hash the name is bound to, for good.
    pub hash: MediaHash,
    /// Whether its file is held.
    pub state: BindState,
}

/// What an admitted bind did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    /// The name was unbound and is now bound.
    New,
    /// The name was already bound to the same hash and nothing changed.
    Unchanged,
    /// The name's released binding is live again, for the same hash.
    Revived,
}

/// Why a bind is refused. A refused bind leaves the slot as it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindRefusal {
    /// The name is certified for another hash.
    Mismatch,
    /// The name is bound, live or released, to another hash.
    Conflict,
}

impl fmt::Display for BindRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mismatch => "the name certifies another hash",
            Self::Conflict => "the name is bound to another hash",
        })
    }
}

impl std::error::Error for BindRefusal {}

/// What an admitted release did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// The live binding is now released.
    Released,
    /// The binding was already released and nothing changed.
    AlreadyReleased,
}

/// Why a release is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseRefusal {
    /// The name is not bound.
    Unbound,
}

impl fmt::Display for ReleaseRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the name is not bound")
    }
}

impl std::error::Error for ReleaseRefusal {}

/// One bind over one name's slot: the slot after, and what happened. STUB: every bind is refused
/// as a conflict.
#[must_use]
pub fn step(
    slot: Option<Binding>,
    _claimed: Option<MediaHash>,
    _hash: MediaHash,
) -> (Option<Binding>, Result<Bound, BindRefusal>) {
    (slot, Err(BindRefusal::Conflict))
}

/// One release over one name's slot. STUB: every release is refused as unbound.
#[must_use]
pub fn release_step(slot: Option<Binding>) -> (Option<Binding>, Result<Release, ReleaseRefusal>) {
    (slot, Err(ReleaseRefusal::Unbound))
}

/// Every name's binding, keyed by the name as written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bindings {
    slots: BTreeMap<String, Binding>,
}

impl Bindings {
    /// No name bound.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds `name` to `hash` by [`step`].
    ///
    /// # Errors
    ///
    /// [`BindRefusal::Mismatch`] when `name` is certified for another hash, and
    /// [`BindRefusal::Conflict`] when it is bound, live or released, to another hash.
    pub fn bind(&mut self, name: &MediaName, hash: MediaHash) -> Result<Bound, BindRefusal> {
        let key = name.to_string();
        let (slot, outcome) = step(self.slots.get(&key).copied(), name.claimed(), hash);
        if let Some(slot) = slot {
            self.slots.insert(key, slot);
        }
        outcome
    }

    /// Releases `name`'s binding by [`release_step`].
    ///
    /// # Errors
    ///
    /// [`ReleaseRefusal::Unbound`] when the name is not bound.
    pub fn release(&mut self, name: &MediaName) -> Result<Release, ReleaseRefusal> {
        let key = name.to_string();
        let (slot, outcome) = release_step(self.slots.get(&key).copied());
        if let Some(slot) = slot {
            self.slots.insert(key, slot);
        }
        outcome
    }

    /// `name`'s binding, if it has one.
    #[must_use]
    pub fn get(&self, name: &MediaName) -> Option<Binding> {
        self.slots.get(&name.to_string()).copied()
    }

    /// How many names are bound, live or released.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether no name is bound.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}
