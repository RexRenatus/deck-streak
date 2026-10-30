//! The one id allocator: every new note, card, deck, notetype and review id a device or the
//! server makes (SPEC-509 R5; ADR-527 D4).
//!
//! An id is a millisecond instant moved forward to the allocating slot's residue mod
//! [`SLOTS`], above a floor: the largest id the store has made or seen. So a device's next id is
//! above every id it has seen, it stays its creation instant within 63 ms, and two slots never
//! make the same id. The floor is the hybrid clock's last reading
//! ([`crate::clock::Hlc::next_id_at`]).

use std::fmt;

/// How many slots ids are spread over: an id's residue mod `SLOTS` is the slot that made it.
pub const SLOTS: i64 = 64;

/// A slot: 0 is the server's, and 1 to 63 are given once each to an enrolled device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Slot(u8);

impl Slot {
    /// The server's slot.
    pub const SERVER: Self = Self(0);

    /// The slot numbered `number`, or `None` when the number is not below [`SLOTS`].
    #[must_use]
    pub fn new(number: u8) -> Option<Self> {
        (i64::from(number) < SLOTS).then_some(Self(number))
    }

    /// The slot's number.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Why no id can be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    /// The id would pass the largest value an `i64` holds.
    Exhausted,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted => f.write_str("no id above the floor fits in 64 bits"),
        }
    }
}

impl std::error::Error for IdError {}

/// The least id above `floor` and at or above `now` whose residue mod [`SLOTS`] is `slot`.
///
/// # Errors
///
/// [`IdError::Exhausted`] when that id would pass `i64::MAX`.
pub fn next_id(floor: i64, now: i64, slot: Slot) -> Result<i64, IdError> {
    let base = floor.checked_add(1).ok_or(IdError::Exhausted)?.max(now);
    let add = i64::from(slot.get());
    base.checked_add(add).ok_or(IdError::Exhausted)
}
