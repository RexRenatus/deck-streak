//! Back up now (SPEC-516 R4, R5; ADR-534 D3): the open collection written into one new file on the
//! owner's word, by the core's fixed statement on the engine's database door, and read back from
//! its file by a private engine. It reaches the engine as a download's backup does (SPEC-364 R5),
//! through the one-way module's fill check, statement and read, so it adds no pair to the
//! per-transport table (R10). Where the file lives, and its name, are the adapter's.

use std::path::Path;

use crate::dispatch::Dispatcher;
use crate::full_sync::IdSets;
use crate::one_way::{self, Reason};

/// Writes the collection `dispatcher` has open into the new file `backup`, and answers the ids a
/// private engine reads back from it, once they hold every review, card and note id the device
/// held before the write.
///
/// # Errors
///
/// [`Reason::OpenCollection`] when `backup` names the open collection, however it is spelled, and
/// [`Reason::HoldsRows`] when its file already holds a review, a card or a note, each before the
/// engine writes anything; [`Reason::Unheld`] when the file, read back, lacks an id the device
/// held; and [`Reason::Engine`] when the engine, a private engine or a read refuses.
pub fn back_up(dispatcher: &Dispatcher, backup: &Path) -> Result<IdSets, Reason> {
    let device = dispatcher.id_sets().map_err(Reason::Engine)?;
    one_way::fillable(dispatcher, backup)?;
    dispatcher
        .execute(one_way::BACKUP_SQL, backup)
        .map_err(Reason::Engine)?;
    let read = one_way::read(dispatcher, backup)?;
    let holds = read.reviews.is_superset(&device.reviews)
        && read.cards.is_superset(&device.cards)
        && read.notes.is_superset(&device.notes);
    if holds { Ok(read) } else { Err(Reason::Unheld) }
}
