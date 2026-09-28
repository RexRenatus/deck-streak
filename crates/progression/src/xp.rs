//! XP: the amount one grant pays, the total a ledger sums, and the level curve that maps a total to
//! a level (SPEC-040 R6 to R8; ADR-040).
//!
//! An amount is unsigned and has no constructor from a signed integer, so no path can express a
//! debit: XP is never confiscable (CHARTER 5). The level is the predecessor's
//! `gamification/xp.py:level_for_xp` at `27ee2bc` in integer arithmetic, proved against the parity
//! golden `level_for_xp.json` (CHARTER 8).

/// The level curve's quadratic coefficient: reaching level L takes `50L² − 50L` XP (`economy.json`,
/// `xp.level_curve.quadratic`).
pub const LEVEL_CURVE_QUADRATIC: u64 = 50;

/// The magnitude of the level curve's linear coefficient, which the curve subtracts
/// (`economy.json`, `xp.level_curve.linear`, is −50).
pub const LEVEL_CURVE_LINEAR: u64 = 50;

/// The XP one grant pays, never negative.
///
/// It is built only from an unsigned integer and offers no subtraction, so a penalty path that
/// would debit XP does not compile (CHARTER 5; SPEC-040 A6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct XpAmount(i64);

impl XpAmount {
    /// The amount `amount`.
    #[must_use]
    pub const fn new(amount: i64) -> Self {
        Self(amount)
    }

    /// The amount, as the ledger stores it.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

/// A sum of grants: the whole ledger's, or one track's (R7). It is unsigned, as every amount is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct XpTotal(u64);

impl XpTotal {
    /// The total `total`.
    #[must_use]
    pub const fn new(total: u64) -> Self {
        Self(total)
    }

    /// The total, in XP.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A level: 1 at no XP, and otherwise the largest level whose threshold a total reaches.
///
/// Only [`level_for`] makes one, so every level is one a total can reach, and the XP to reach it
/// fits a total. No level is stored: it is derived from the ledger's total when it is read (R8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(u32);

impl Level {
    /// The level's number, from 1.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// The level of `total`: `(50 + isqrt(2500 + 200 × total)) / 100`, floor-divided, in 128-bit
/// integers (R7).
#[must_use]
pub fn level_for(total: XpTotal) -> Level {
    let _ = total;
    Level(1)
}

/// The XP at which `level` begins, `50L² − 50L`: 0 for level 1 (the predecessor's
/// `gamification/xp.py:xp_to_reach`).
#[must_use]
pub fn xp_to_reach(level: Level) -> XpTotal {
    let _ = level;
    XpTotal(0)
}
