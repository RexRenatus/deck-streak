//! The adapter tests' one clock (SPEC-443 R1; ADR-457 D1, D2).
//!
//! The engine schedules an answer from its own reading of now against the collection's next
//! cutoff, and no seam fixes that reading, so the clock fixes the cutoff against it. Each operation
//! reads the engine's own instant once, `TimestampMillis::now()`, and fixes the clock's instant one
//! millisecond before it. The collections these tests build run the engine's version-2 timing:
//! their next cutoff falls on the rollover hour in the zone the engine reads, and their creation
//! stamp moves only the day count. `place` sets a start to the second by selecting the engine's
//! version-1 timing (`schedVer` 1), whose next cutoff is the creation stamp plus whole days; the
//! engine refuses the review's bury under it, so a placed collection is a start only. `pin` keeps
//! the version-2 timing production runs and moves the rollover hour, so the next cutoff is the last
//! local hour boundary at or before twelve hours after the instant, at day 0. Each operation reads
//! the day count and the next cutoff back through the engine's own timing of today and asserts
//! both, so its arithmetic is checked by the engine at every run. Every time a scheduler test hands
//! the engine is the clock's instant, or a stated offset from it.

#![allow(
    dead_code,
    reason = "each test target calls the part of the clock it needs"
)]

use std::path::Path;

use anki::collection::CollectionBuilder;
use anki::timestamp::TimestampMillis;

/// One day: the placement's creation stamp a day before its cutoff, and the pin's creation stamp
/// at the cutoff before its predicted one.
pub const DAY_SECS: i64 = 86_400;
/// One step of the rollover hour, the grain of the engine's version-2 cutoff.
pub const HOUR_SECS: i64 = 3_600;
/// The pin: the next cutoff the last local hour boundary at or before 12 hours after the instant.
pub const LEAD_SECS: i64 = 43_200;
/// A side start: the next cutoff five minutes after the instant.
pub const JUST_BEFORE_SECS: i64 = 300;
/// A side start: the previous cutoff five minutes before the instant.
pub const JUST_AFTER_SECS: i64 = 86_100;

/// The instant a scheduler test hands the engine: one millisecond before the engine's own reading
/// of now when the collection was placed or pinned, so no later read of any clock in the run
/// equals it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    instant_millis: i64,
}

impl Clock {
    /// The instant, in milliseconds since the epoch: an answer's time.
    pub fn at_millis(self) -> i64 {
        self.instant_millis
    }

    /// The instant, in whole seconds since the epoch: a deck tree's `now`.
    pub fn at_secs(self) -> i64 {
        self.instant_millis / 1000
    }
}

/// Reads the engine's instant once, selects the engine's version-1 timing on the closed
/// `collection` and places its next cutoff `start` seconds after the clock's instant, at day 0,
/// then reads both back through the engine's own timing of today and asserts them.
pub fn place(collection: &Path, start: i64) -> Clock {
    let clock = Clock {
        instant_millis: TimestampMillis::now().0 - 1,
    };
    let mut col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the collection to place its cutoff");
    col.set_config_json("schedVer", &1_u8, false)
        .expect("the engine accepts schedVer 1, its version-1 timing");
    let written = col
        .storage
        .db()
        .execute(
            "update col set crt = ?1",
            [clock.at_secs() + start - DAY_SECS],
        )
        .expect("the collection's creation stamp is written");
    assert_eq!(written, 1, "the collection holds its one row");
    col.close(None).expect("the engine closes the collection");
    let mut col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine reopens the placed collection");
    let timing = col
        .timing_today()
        .expect("the engine reads its timing of today");
    assert_eq!(
        (timing.days_elapsed, timing.next_day_at.0),
        (0, clock.at_secs() + start),
        "the engine's day count and next cutoff on the placed collection: day 0, the cutoff {start} \
         s after the clock's instant"
    );
    col.close(None)
        .expect("the engine closes the placed collection");
    clock
}

/// Reads the engine's instant once and pins the closed `collection`'s next cutoff, on the engine's
/// version-2 timing, to the last local hour boundary at or before [`LEAD_SECS`] after the clock's
/// instant, at day 0: from one reading of the engine's next cutoff and rollover hour, it predicts
/// the cutoff in whole [`HOUR_SECS`], moves the rollover hour by as many hours, writes the creation
/// stamp at the cutoff before the predicted one, then reads both back through the engine's own
/// timing of today and asserts them.
pub fn pin(collection: &Path) -> Clock {
    let clock = Clock {
        instant_millis: TimestampMillis::now().0 - 1,
    };
    let mut col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the collection to pin its cutoff");
    col.set_config_json("schedVer", &2_u8, false)
        .expect("the engine accepts schedVer 2, its version-2 timing");
    let first = col
        .timing_today()
        .expect("the engine reads its timing of today");
    let rollover = i64::from(
        col.rollover_for_current_scheduler()
            .expect("the engine reads its rollover hour"),
    );
    let hours = (clock.at_secs() + LEAD_SECS - first.next_day_at.0).div_euclid(HOUR_SECS);
    let predicted = first.next_day_at.0 + hours * HOUR_SECS;
    let hour = u32::try_from((rollover + hours).rem_euclid(24)).expect("an hour of the day");
    col.set_config_json("rollover", &hour, false)
        .expect("the engine accepts the rollover hour");
    let written = col
        .storage
        .db()
        .execute("update col set crt = ?1", [predicted - DAY_SECS])
        .expect("the collection's creation stamp is written");
    assert_eq!(written, 1, "the collection holds its one row");
    col.close(None).expect("the engine closes the collection");
    let mut col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine reopens the pinned collection");
    let timing = col
        .timing_today()
        .expect("the engine reads its timing of today");
    assert_eq!(
        (timing.days_elapsed, timing.next_day_at.0),
        (0, predicted),
        "the engine's day count and next cutoff on the pinned collection: day 0, the cutoff \
         predicted from the cutoff {} under rollover hour {rollover}, moved {hours} hours to hour \
         {hour}, {} s after the clock's instant",
        first.next_day_at.0,
        predicted - clock.at_secs()
    );
    col.close(None)
        .expect("the engine closes the pinned collection");
    clock
}
