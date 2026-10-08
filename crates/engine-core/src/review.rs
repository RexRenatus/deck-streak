//! The review's flag and bury rules, one copy for both clients (SPEC-358 R2; ADR-369 D5).
//!
//! The web engine's `wasm32` module and the native adapter both take them from here: the flag the
//! flag action sets, and the user's bury of the one shown card. They moved from the web engine's
//! study rule unchanged in behaviour (SPEC-350 R2, R7).

/// The engine's number for the red flag.
pub const RED: u32 = 1;

/// The flag the flag action sets: red to none, and any other flag, none included, to red, as the
/// desktop's red flag key does.
#[must_use]
pub fn toggled_red(flag: u32) -> u32 {
    if flag == RED { 0 } else { RED }
}

/// The engine's bury mode for the user's own bury, which the next day does not undo alone.
pub const BURY_USER: i32 = 2;

/// A bury request's fields, in the engine's order: the cards, the notes and the mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuryOf {
    /// The card ids to bury.
    pub card_ids: Vec<i64>,
    /// The note ids whose cards to bury.
    pub note_ids: Vec<i64>,
    /// The bury mode.
    pub mode: i32,
}

/// The user's bury of one card: that card's id, no note, the user's mode.
#[must_use]
pub fn bury_of(card: i64) -> BuryOf {
    BuryOf {
        card_ids: vec![card],
        note_ids: Vec::new(),
        mode: BURY_USER,
    }
}
