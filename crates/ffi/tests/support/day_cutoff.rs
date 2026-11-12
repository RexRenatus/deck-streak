//! The review fixture's day pin (SPEC-615 R1 to R3).
//!
//! The engine labels a learning step in days when it would end after the scheduler's next day
//! starts, as Anki does. The review fixture is a version-2 collection, whose next day starts at its
//! rollover hour on the reading host's own clock, and it sets no hour of its own, so its day ended
//! at the engine's default hour and a test run in the last minutes before it read a ten-minute step
//! in days. The pin sets the review collection's `rollover` hour half a day ahead of the writer,
//! measured through the engine's own timing, and reads the day back (R1). It writes that key alone:
//! no scheduler version and no creation stamp (R2). The `review-fixture` example calls it once, on
//! the review collection alone, after building it (R3).
//!
//! Half a day, because the hour is read on the reading host's clock in another job: a reader that
//! keeps the writer's offset meets the cutoff about twelve hours after the write, and a reader whose
//! offset differs by some hours meets it that many hours earlier or later. It returns every failure
//! as an error rather than panicking, because the example is not a test.

#![allow(
    dead_code,
    reason = "the example and each test that includes this file read the part they need"
)]

use std::error::Error;
use std::path::Path;

use anki::collection::{Collection, CollectionBuilder};

/// The lead the kept hour aims for, in seconds from the engine's reading of now to its next day
/// cutoff: half a day.
const TARGET_LEAD_SECS: i64 = 12 * 3600;
/// One hour, in seconds. The day's rollover hours lie one hour apart, so the kept one lies within
/// half an hour of the target, and the read-back's window is the target less one hour, exclusive,
/// to the target plus half an hour.
const HOUR_SECS: i64 = 3_600;

/// The day the pin set and read back.
pub struct Pinned {
    /// The rollover hour the pin set, as the collection stores it.
    pub hour: u32,
    /// The engine's instant of the read-back, in seconds since the epoch.
    pub t: i64,
    /// The engine's next day cutoff at the read-back, in seconds since the epoch.
    pub next_day_at: i64,
}

/// Sets the collection's rollover hour to the one whose next day cutoff lies nearest half a day
/// ahead, and reads the day back.
///
/// The engine caches its day per open collection, so each hour of the day is measured in a fresh
/// open of its own, closed before the next; the kept hour is then set, and read back in one more.
///
/// # Errors
///
/// Any step the engine refuses, or a read-back whose stored hour is not the kept one or whose lead
/// lies outside the window.
pub fn pin(collection: &Path) -> Result<Pinned, Box<dyn Error>> {
    let mut leads = Vec::with_capacity(24);
    for hour in 0_u32..24 {
        leads.push((hour, set_rollover(collection, hour)?));
    }
    let (hour, _) = leads
        .into_iter()
        .min_by_key(|&(_, lead)| (lead - TARGET_LEAD_SECS).abs())
        .ok_or("the day has no hour to keep")?;
    set_rollover(collection, hour)?;

    let mut col = CollectionBuilder::new(collection).build()?;
    let stored = stored_rollover(&col)?;
    let timing = col.timing_today()?;
    col.close(None)?;
    if stored != hour {
        return Err(
            format!("the collection stores rollover hour {stored}, not the kept {hour}").into(),
        );
    }
    let lead = timing.next_day_at.0 - timing.now.0;
    if lead <= TARGET_LEAD_SECS - HOUR_SECS || lead > TARGET_LEAD_SECS + HOUR_SECS / 2 {
        return Err(format!(
            "rollover hour {hour} reads a lead of {lead} seconds, outside the window around {TARGET_LEAD_SECS}"
        )
        .into());
    }
    Ok(Pinned {
        hour,
        t: timing.now.0,
        next_day_at: timing.next_day_at.0,
    })
}

/// Opens the collection, sets `hour` as its rollover, and returns the seconds from the engine's
/// reading of now to its next day cutoff, closing the collection before it returns.
fn set_rollover(collection: &Path, hour: u32) -> Result<i64, Box<dyn Error>> {
    let mut col = CollectionBuilder::new(collection).build()?;
    col.set_config_json("rollover", &hour, false)?;
    let timing = col.timing_today()?;
    col.close(None)?;
    Ok(timing.next_day_at.0 - timing.now.0)
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
