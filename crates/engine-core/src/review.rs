//! The review's flag and bury rules, one copy for both clients (SPEC-358 R2; ADR-369 D5).
//!
//! The web engine's `wasm32` module and the native adapter both take them from here: the flag the
//! flag action sets, and the user's bury of the one shown card. They moved from the web engine's
//! study rule unchanged in behaviour (SPEC-350 R2, R7). Beside them sit the two requests the native
//! adapter sends for a bury and a flag, encoded here because the adapter holds no protobuf codec of
//! its own (SPEC-358 R3).

use anki_proto::cards::SetFlagRequest;
use anki_proto::scheduler::BuryOrSuspendCardsRequest;
use prost::Message;

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

/// The engine's `BuryOrSuspendCardsRequest` for `bury`, encoded: the bytes the native adapter sends
/// for (13,14) (SPEC-358 R3).
#[must_use]
pub fn bury_request(bury: BuryOf) -> Vec<u8> {
    BuryOrSuspendCardsRequest {
        card_ids: bury.card_ids,
        note_ids: bury.note_ids,
        mode: bury.mode,
    }
    .encode_to_vec()
}

/// The engine's `SetFlagRequest` that sets `flag` on the one card `card`, encoded: the bytes the
/// native adapter sends for (5,4) (SPEC-358 R3).
#[must_use]
pub fn flag_request(card: i64, flag: u32) -> Vec<u8> {
    let card_ids = vec![card];
    SetFlagRequest { card_ids, flag }.encode_to_vec()
}
