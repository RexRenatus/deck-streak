//! The rule that keeps, sends and drops the sync key (SPEC-363 R3; ADR-374 D5, D6, D11).
//!
//! A client keeps its sync key in a store of its own: the web client's Worker seals it into the
//! browser's indexed database, beside one [`Generation`]. The store only keeps; this module
//! decides. A login started at one generation is kept only when that generation is still the
//! stored one, so a sign-out or another kept login that landed while it was in flight wins. A send
//! is admitted only when a sealed record is stored at the generation the sender holds. Only the
//! sync server's refusal of the current generation drops the key: every other answer keeps it, and
//! a refusal of an older generation leaves a newer key alone. A removal raises the generation, so
//! a login or a send that started before it is refused afterwards.
//!
//! The module does no I/O and reads no clock. `formal/tla/SyncCredential` models these rules over
//! two Workers, a sign-out, a restart and the session's end.

use anki_proto::backend::BackendError;
use anki_proto::backend::backend_error::Kind;
use prost::Message;

/// The count that orders every write of the sync key: it rises at every kept login and every
/// removal, never falls, and stops at its maximum rather than wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Generation(u64);

impl Generation {
    /// The generation of a store that has never kept a key.
    pub const ZERO: Self = Self(0);

    /// The generation after this one, or `None` at the maximum: a count that wrapped would make
    /// an old login current again.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

impl From<u64> for Generation {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Generation> for u64 {
    fn from(generation: Generation) -> Self {
        generation.0
    }
}

/// What the store does with a login that has landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    /// Store the key, with the generation set to `at`.
    Store {
        /// The generation the key is stored at.
        at: Generation,
    },
    /// Drop the key: a removal or another kept login landed while this one was in flight.
    Discard,
}

/// How a sync's answer settles the key it was sent with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The server accepted the key.
    Accepted,
    /// The server refused the key: it no longer works.
    Refused,
    /// The sync failed for any other reason, and the key may still work.
    Failed,
}

/// Decides whether a login that started at `started` is kept, now that the store holds `current`.
#[must_use]
pub fn on_obtained(started: Generation, current: Generation) -> Kept {
    match current.next() {
        Some(at) if started == current => Kept::Store { at },
        _ => Kept::Discard,
    }
}

/// Whether a sender holding `held` may send, with `current` stored and `sealed` saying whether a
/// sealed record is stored.
#[must_use]
pub fn may_send(held: Generation, current: Generation, sealed: bool) -> bool {
    sealed && held == current
}

/// Reads a sync's answer: `None` is success, `Some` is the engine's encoded `BackendError`.
///
/// Only the kind `SYNC_AUTH_ERROR` is a refusal, because the sync server's refusal is the one
/// answer that says the key no longer works. Every other kind, a kind the engine does not name and
/// bytes that do not decode are failures that keep the key (ADR-374 D6).
#[must_use]
pub fn classify(error: Option<&[u8]>) -> Outcome {
    let Some(bytes) = error else {
        return Outcome::Accepted;
    };
    match BackendError::decode(bytes) {
        Ok(decoded) if decoded.kind == i32::from(Kind::SyncAuthError) => Outcome::Refused,
        _ => Outcome::Failed,
    }
}

/// Whether the answer to a send made at `sent` drops the key, with `current` stored now.
#[must_use]
pub fn on_outcome(sent: Generation, current: Generation, outcome: Outcome) -> bool {
    outcome == Outcome::Refused && sent == current
}

/// The generation a removal stores, or `None` at the maximum.
#[must_use]
pub fn on_removed(current: Generation) -> Option<Generation> {
    current.next()
}
