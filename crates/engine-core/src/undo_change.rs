//! The rule an undo of the review's last bury or flag passes (SPEC-383 R5; ADR-397 D3, D8).
//!
//! The review records its bury or flag after the write: the engine's undo status at that moment,
//! the card it changed, its kind and, for a flag, the user flag it left. An undo is admitted only
//! while the change is still on the card, the card has not synced, and the change is still the
//! front of the engine's undo queue. The module is pure: the dispatcher reads the engine's state
//! and the card's mark and hands them here, so the rule is judged without an engine. An answer is
//! judged by [`crate::undo_answer::judge`], unchanged.

use anki_proto::collection::UndoStatus;

use crate::undo_answer::{Kind, Recorded, UndoRefusal};

/// The user flag's bits in a card's `flags` column: the low three, the only bits the engine's
/// `SetFlag` writes, and every other bit it keeps.
const USER_FLAG: u32 = 7;

/// The engine's queue for a card the user buried, which a bury's undo finds it still in.
const USER_BURIED: i32 = -3;

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

/// The reply of the mark read held a row that is not a card's mark: a column missing, or out of
/// its column's range. The dispatcher answers it as the engine's own unreadable reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unreadable;

impl Mark {
    /// The mark the rows of `CardMark` hold, `[[queue, flags, usn]]`, as the engine's database door
    /// answers them; `None` when no row matched, because the card is gone. Both the core's restore
    /// and the web engine's record read the mark through here, so the two never read it apart.
    ///
    /// # Errors
    ///
    /// [`Unreadable`] when the first row's three columns are not a queue, a flags column and a
    /// sync mark.
    pub fn from_rows(rows: &serde_json::Value) -> Result<Option<Self>, Unreadable> {
        let Some(row) = rows.pointer("/0") else {
            return Ok(None);
        };
        let column = |at: &str| row.pointer(at).and_then(serde_json::Value::as_i64);
        let queue = column("/0").and_then(|queue| i32::try_from(queue).ok());
        let flags = column("/1").and_then(|flags| u32::try_from(flags).ok());
        let usn = column("/2").and_then(|usn| i32::try_from(usn).ok());
        match (queue, flags, usn) {
            (Some(queue), Some(flags), Some(usn)) => Ok(Some(Self { queue, flags, usn })),
            _ => Err(Unreadable),
        }
    }
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
    let Some(mark) = mark else {
        return Err(UndoRefusal::Gone);
    };
    if recorded.card != card {
        return Err(UndoRefusal::NotTheCard);
    }
    let kept = match Kind::try_from(recorded.kind) {
        Ok(Kind::Bury) => mark.queue == USER_BURIED,
        Ok(Kind::Flag) => (mark.flags & USER_FLAG) == recorded.flag,
        Ok(Kind::Answer) | Err(_) => false,
    };
    if !kept {
        return Err(UndoRefusal::Changed);
    }
    if mark.usn != -1 {
        return Err(UndoRefusal::Synced);
    }
    if now.undo.is_empty() {
        return Err(UndoRefusal::Gone);
    }
    let then = recorded.status.as_ref();
    let last_step = then.map_or(0, |then| then.last_step);
    let label = then.map_or("", |then| then.undo.as_str());
    if now.last_step != last_step {
        return Err(UndoRefusal::Changed);
    }
    if now.undo != label {
        return Err(UndoRefusal::Changed);
    }
    Ok(())
}
