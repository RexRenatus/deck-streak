//! The adapter tests' one clock (SPEC-443 R1; ADR-457 D1, D2).
//!
//! The engine schedules an answer from its own reading of now against the collection's next
//! cutoff, and no seam fixes that reading. So the clock reads the engine's own instant once,
//! `TimestampMillis::now()`, fixes its instant one millisecond before it, and places the
//! collection's next cutoff a stated lead after that instant by writing the closed collection's
//! creation stamp: a collection that sets no scheduler version runs the engine's version-1 timing,
//! whose next cutoff is its creation stamp plus whole days. It reads the day count and the next
//! cutoff back through the engine's own timing of today and asserts both, so its arithmetic is
//! checked by the engine at every run. Every time a scheduler test hands the engine is the clock's
//! instant, or a stated offset from it.

#![allow(
    dead_code,
    reason = "each test target calls the part of the clock it needs"
)]

use std::path::Path;

use anki::collection::CollectionBuilder;
use anki::timestamp::TimestampMillis;

/// One day of the engine's version-1 timing.
pub const DAY_SECS: i64 = 86_400;
/// The pin: the next cutoff 12 hours after the instant, and the previous one 12 hours before it.
pub const LEAD_SECS: i64 = 43_200;
/// A side start: the next cutoff five minutes after the instant.
pub const JUST_BEFORE_SECS: i64 = 300;
/// A side start: the previous cutoff five minutes before the instant.
pub const JUST_AFTER_SECS: i64 = 86_100;

/// The instant a scheduler test hands the engine: one millisecond before the engine's own reading
/// of now when the collection was placed, so no later read of any clock in the run equals it.
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

/// Reads the engine's instant once and places the closed `collection`'s next cutoff `lead`
/// seconds after the clock's instant, at day 0, then reads both back through the engine's own
/// timing of today and asserts them.
pub fn place(collection: &Path, lead: i64) -> Clock {
    let clock = Clock {
        instant_millis: TimestampMillis::now().0 - 1,
    };
    let col = CollectionBuilder::new(collection)
        .build()
        .expect("the engine opens the collection to place its cutoff");
    let written = col
        .storage
        .db()
        .execute(
            "update col set crt = ?1",
            [clock.at_secs() + lead - DAY_SECS],
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
        (0, clock.at_secs() + lead),
        "the engine's day count and next cutoff on the placed collection: day 0, the cutoff {lead} s \
         after the clock's instant"
    );
    col.close(None)
        .expect("the engine closes the placed collection");
    clock
}

/// Places the collection's next cutoff [`LEAD_SECS`] after the clock's instant.
pub fn pin(collection: &Path) -> Clock {
    let _ = collection;
    Clock {
        instant_millis: TimestampMillis::now().0 - 1,
    }
}
