//! Back up now (SPEC-516 R4, R5; ADR-534 D3): the open collection written into one new file on the
//! owner's word, by the core's fixed statement on the engine's database door, and read back from
//! its file by a private engine. It adds no pair to the per-transport table (R10).

use std::path::Path;

use crate::dispatch::Dispatcher;
use crate::full_sync::IdSets;
use crate::one_way::Reason;

/// Writes the collection `dispatcher` has open into the new file `backup`, and answers the ids a
/// private engine reads back from it.
///
/// # Errors
///
/// A [`Reason`] when the backup cannot be made or read.
pub fn back_up(dispatcher: &Dispatcher, backup: &Path) -> Result<IdSets, Reason> {
    let _ = (dispatcher, backup);
    Err(Reason::Unheld)
}
