//! The hybrid clock every operation carries (SPEC-509 R4; ADR-527 D2).
//!
//! An operation's clock is one past the larger of the device's own last clock and the largest
//! clock it has pulled, and never below the device's wall clock. So an operation made after a
//! device saw another sorts after it whatever the wall clocks say, and the clock stays the wall
//! clock whenever that is not behind something the device has seen. The id allocator takes the
//! clock's last reading as its floor, so ids and clocks rise together under one rule.

use std::fmt;
use std::ops::Add;

use crate::ids::{IdError, Slot, next_id};

/// Why the clock cannot move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockError {
    /// The clock already reads the largest instant an `i64` holds.
    Exhausted,
}

impl fmt::Display for ClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted => f.write_str("the hybrid clock is at the largest instant it holds"),
        }
    }
}

impl std::error::Error for ClockError {}

/// The next clock from the device's own `last` clock, the largest clock it has `seen` and its
/// wall clock `pt`: one past the larger of the first two, and never below the wall.
///
/// # Errors
///
/// [`ClockError::Exhausted`] when the larger of `last` and `seen` is `i64::MAX`.
pub fn tick(last: i64, seen: i64, pt: i64) -> Result<i64, ClockError> {
    let top = last.max(seen);
    // The Kani witness (ADR-527 D7): with its feature on, `last + 1` is a plain add, which
    // overflows at `i64::MAX`, so `kani_proofs::hlc_tick_no_overflow` fails.
    #[expect(
        clippy::disallowed_macros,
        reason = "a Kani witness feature, off in every build but the formal check's witness run (ADR-527 D7)"
    )]
    let past = if cfg!(feature = "kani-witness-hlc-plain-add") {
        Add::add(top, 1)
    } else {
        top.checked_add(1).ok_or(ClockError::Exhausted)?
    };
    Ok(past.max(pt))
}

/// A device's clock: the largest clock it has made or seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hlc {
    last: i64,
}

impl Hlc {
    /// A clock whose last reading is `last`: the largest clock or id the device's store holds.
    #[must_use]
    pub const fn starting_at(last: i64) -> Self {
        Self { last }
    }

    /// The largest clock the device has made or seen.
    #[must_use]
    pub const fn last(self) -> i64 {
        self.last
    }

    /// Records a clock the device pulled: every later clock it makes is past it.
    pub fn recv(&mut self, seen: i64) {
        self.last = self.last.max(seen);
    }

    /// The clock of an operation made at wall instant `pt`, which becomes the last reading.
    ///
    /// # Errors
    ///
    /// [`ClockError::Exhausted`] when the last reading is `i64::MAX`; the reading is unchanged.
    pub fn tick_at(&mut self, pt: i64) -> Result<i64, ClockError> {
        let next = tick(self.last, self.last, pt)?;
        self.last = next;
        Ok(next)
    }

    /// A new id for `slot` made at wall instant `pt`: the allocator's floor is the clock's last
    /// reading, and the id becomes it.
    ///
    /// # Errors
    ///
    /// [`IdError::Exhausted`] when no id above the last reading fits; the reading is unchanged.
    pub fn next_id_at(&mut self, pt: i64, slot: Slot) -> Result<i64, IdError> {
        let id = next_id(self.last, pt, slot)?;
        self.last = id;
        Ok(id)
    }
}
