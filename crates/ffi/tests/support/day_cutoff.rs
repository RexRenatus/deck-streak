//! The review fixture's day pin (SPEC-615 R1 to R3).
//!
//! The engine labels a learning step in days when it would end after the scheduler's next day
//! starts, as Anki does. The review fixture is a version-2 collection, whose next day starts at its
//! rollover hour on the reading host's own clock, and it sets no hour of its own. The pin is to set
//! the review collection's `rollover` hour half a day ahead of the writer, measured through the
//! engine's own timing, and to read the day back (R1); to write that key alone (R2); and to be
//! called by the `review-fixture` example once, on the review collection alone (R3).
//!
//! In this form the pin is inert: it opens the collection once, reads the engine's day and the
//! stored hour, and closes it, with no write of its own. It returns every failure as an error
//! rather than panicking, because the example is not a test.

#![allow(
    dead_code,
    reason = "the example and each test that includes this file read the part they need"
)]

use std::error::Error;
use std::path::Path;

use anki::collection::{Collection, CollectionBuilder};

/// The day the pin read back.
pub struct Pinned {
    /// The rollover hour the collection stores.
    pub hour: u32,
    /// The engine's instant of the read-back, in seconds since the epoch.
    pub t: i64,
    /// The engine's next day cutoff at the read-back, in seconds since the epoch.
    pub next_day_at: i64,
}

/// Reads the collection's day and its stored rollover hour.
///
/// # Errors
///
/// Any step the engine refuses: the open, the day's timing, the stored hour or the close.
pub fn pin(collection: &Path) -> Result<Pinned, Box<dyn Error>> {
    let mut col = CollectionBuilder::new(collection).build()?;
    let timing = col.timing_today()?;
    let hour = stored_rollover(&col)?;
    col.close(None)?;
    Ok(Pinned {
        hour,
        t: timing.now.0,
        next_day_at: timing.next_day_at.0,
    })
}

/// The rollover hour the collection's config stores. The engine keeps every config value as the
/// bytes of its JSON text.
fn stored_rollover(col: &Collection) -> Result<u32, Box<dyn Error>> {
    let bytes: Vec<u8> =
        col.storage
            .db()
            .query_row("select val from config where key = 'rollover'", [], |row| {
                row.get(0)
            })?;
    Ok(std::str::from_utf8(&bytes)?.trim().parse()?)
}
