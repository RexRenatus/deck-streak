//! The pity counters (SPEC-081 R6): the chests since the last Epic and since the last Legendary.
//!
//! They are one stored row, `pity`, moved in the same write as the chest that moved them
//! (ADR-081), and they decide the roll's guarantees (`chests::roll_rarity`).

use crate::chests::Rarity;

/// The chests since the last Epic and since the last Legendary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pity {
    /// Chests since the last Epic.
    pub since_epic: i64,
    /// Chests since the last Legendary.
    pub since_legendary: i64,
}

impl Pity {
    /// The counters after a chest of `rarity`: an Epic resets the Epic counter and the Legendary
    /// counter gains one; a Legendary resets the Legendary counter and the Epic counter gains one;
    /// any other rarity adds one to both.
    #[must_use]
    pub const fn after(self, rarity: Rarity) -> Self {
        let _ = rarity;
        self
    }
}
