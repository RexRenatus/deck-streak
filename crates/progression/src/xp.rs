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
pub struct XpAmount(u32);

impl XpAmount {
    /// The amount `amount`.
    #[must_use]
    pub const fn new(amount: u32) -> Self {
        Self(amount)
    }

    /// The amount, as the ledger stores it.
    #[must_use]
    pub const fn get(self) -> u32 {
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
///
/// The square root is an integer square root, never a floating-point one, which places the total
/// one XP below a large threshold a level too high; and 128-bit, so `200 × total` cannot overflow
/// for the widest total. The predecessor's `max(1, …)` never binds here: the least total, 0, gives
/// `(50 + 50) / 100 = 1`.
#[must_use]
pub fn level_for(total: XpTotal) -> Level {
    let linear = u128::from(LEVEL_CURVE_LINEAR);
    let quadratic = u128::from(LEVEL_CURVE_QUADRATIC);
    let root = (linear * linear + 4 * quadratic * u128::from(total.get())).isqrt();
    let level = (linear + root) / (2 * quadratic);
    // The widest total, `u64::MAX`, is level 607,400,100, so the conversion never saturates.
    Level(u32::try_from(level).unwrap_or(u32::MAX))
}

/// The XP at which `level` begins, `50L² − 50L`: 0 for level 1 (the predecessor's
/// `gamification/xp.py:xp_to_reach`).
#[must_use]
pub fn xp_to_reach(level: Level) -> XpTotal {
    let level = u64::from(level.get());
    // `L × (50L − 50)`: a level is one a total reaches, so the product is at most that total and
    // fits, and `50L − 50` is never negative from level 1.
    XpTotal(level * (LEVEL_CURVE_QUADRATIC * level - LEVEL_CURVE_LINEAR))
}
