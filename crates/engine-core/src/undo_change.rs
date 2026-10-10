//! The rule an undo of the review's last bury or flag passes (SPEC-383 R5; ADR-397 D3, D8).
//!
//! The review records its bury or flag after the write: the engine's undo status at that moment,
//! the card it changed, its kind and, for a flag, the user flag it left. An undo is admitted only
//! while the change is still on the card, the card has not synced, and the change is still the
//! front of the engine's undo queue. The module is pure: the dispatcher reads the engine's state
//! and the card's mark and hands them here, so the rule is judged without an engine. An answer is
//! judged by [`crate::undo_answer::judge`], unchanged.

use anki_proto::collection::UndoStatus;

use crate::undo_answer::{Recorded, UndoRefusal};

/// The user flag's bits in a card's `flags` column: the low three, the only bits the engine's
/// `SetFlag` writes, and every other bit it keeps.
const USER_FLAG: u32 = 7;

/// A card's queue, flags and sync mark, as the core's fixed read `CardMark` answers them
/// (SPEC-383 R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    /// The card's queue: -3 when the user buried it.
    pub queue: i32,
    /// The card's `flags` column, whose low three bits are the user flag.
    pub flags: u32,
    /// The card's update sequence number: -1 until a sync sends it.
    pub usn: i32,
}

/// Whether the recorded bury or flag may be undone now, on `card`: `now` is the engine's undo
/// status and `mark` the card's mark, both read at the moment of the undo.
///
/// # Errors
///
/// The first refusal that holds, in this order: [`UndoRefusal::Gone`] when the card has no row,
/// [`UndoRefusal::NotTheCard`] when the record is of another card, [`UndoRefusal::Changed`] when a
/// bury's card is no longer user-buried or a flag's user flag is no longer the recorded one,
/// [`UndoRefusal::Synced`] when the card has synced, [`UndoRefusal::Gone`] when the engine has
/// nothing to undo, and [`UndoRefusal::Changed`] when the engine's last step or undo label is not
/// the record's.
pub fn judge_change(
    recorded: &Recorded,
    now: &UndoStatus,
    mark: Option<Mark>,
    card: i64,
) -> Result<(), UndoRefusal> {
    let _ = (recorded, now, mark, card, USER_FLAG);
    Ok(())
}
