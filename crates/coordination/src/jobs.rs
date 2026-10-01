//! The job table: the one schedule every timer is held equal to (SPEC-027 R1, R2; ADR-027,
//! ADR-037).
//!
//! Each job has an id, a schedule in the configured local time (the kernel's rollover hour and
//! fixed offset), and one flag, `catch_up`. No job syncs first: only `sync` syncs, once per study
//! day, and every other job reads the study day's sync outcome. While the predecessor still runs,
//! the slots keep off its minutes (its in-process slots and its sync ticks, named by the port of
//! its tick function below) and off the reserved minutes (ADR-011).

use deck_streak_kernel::{Hour, StudyDayRule, UtcMillis, UtcOffset};

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
const MINUTES_PER_HOUR: i64 = 60;

/// How late a catch-up job's fire may run, in minutes; a later one is recorded `missed` and not
/// run (`scheduler.py:CATCHUP_MAX_LATE_MIN`, proved by `goldens/scheduler.constants.json`).
pub const CATCHUP_MAX_LATE_MIN: i64 = 360;
/// The minute the predecessor's sync ticks are offset by (`constants.py:SYNC_TICK_OFFSET_MIN`).
pub const SYNC_TICK_OFFSET_MIN: i64 = 2;
/// The shortest sync interval the predecessor anchors to the hour
/// (`constants.py:TICK_ALIGN_MIN_INTERVAL_MIN`).
pub const TICK_ALIGN_MIN_INTERVAL_MIN: i64 = 5;
/// One study day, in seconds: the one scheduled sync's cadence (ADR-037).
pub const SYNC_CADENCE_SECS: i64 = 86_400;

/// When a job fires, in the configured local time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Schedule {
    /// Once a day, at the configured rollover hour and this minute: after the day turns over, so
    /// the fire's local date is the study day it runs in.
    DailyAtRollover {
        /// The minute of the hour, 0 to 59.
        minute: u8,
    },
    /// Once a day, at this local hour and minute.
    DailyAt {
        /// The hour, 0 to 23.
        hour: u8,
        /// The minute of the hour, 0 to 59.
        minute: u8,
    },
    /// Every hour, at this minute.
    Hourly {
        /// The minute of the hour, 0 to 59.
        minute: u8,
    },
}

/// One job of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Job {
    /// The id its timer names: `deckstreakd job <id>`.
    pub id: &'static str,
    /// When it fires.
    pub schedule: Schedule,
    /// Whether a fire missed by at most [`CATCHUP_MAX_LATE_MIN`] minutes runs late, once.
    pub catch_up: bool,
}

/// The one scheduled sync of the study day (ADR-037): at the rollover hour, minute 7, five minutes
/// after the predecessor's first sync tick of the day, and claimed per study day. A sync missed by
/// at most [`CATCHUP_MAX_LATE_MIN`] minutes runs once when its timer activates.
pub const SYNC: Job = Job {
    id: "sync",
    schedule: Schedule::DailyAtRollover { minute: 7 },
    catch_up: true,
};

/// The database's daily upkeep, after the day turns over: between the reserved minute 25 (ADR-011)
/// and the predecessor's minute 33 (ADR-027).
pub const MAINTENANCE: Job = Job {
    id: "maintenance",
    schedule: Schedule::DailyAtRollover { minute: 28 },
    catch_up: false,
};

/// The hourly dead-man watch and drift check, at a minute the predecessor never uses, seven
/// minutes after the daily sync's (ADR-027).
pub const LIVENESS: Job = Job {
    id: "liveness",
    schedule: Schedule::Hourly { minute: 14 },
    catch_up: false,
};

/// The hourly law drill post-back (SPEC-110 R9): it records each graded drill and pays it once, at
/// a minute the predecessor never uses and no other job of the table takes.
pub const DRILL_POSTBACK: Job = Job {
    id: "drill_postback",
    schedule: Schedule::Hourly { minute: 19 },
    catch_up: false,
};

/// The scheduled flush of the held notifications (SPEC-041 R7, amended for #291): once a day, at a
/// local hour and minute outside the quiet window, so the first flush after the window ends
/// delivers what the window held.
pub const HELD_FLUSH: Job = Job {
    id: "held_flush",
    schedule: Schedule::DailyAt {
        hour: 7,
        minute: 36,
    },
    catch_up: true,
};

/// The one schedule: every job a timer may start.
pub const TABLE: [Job; 4] = [SYNC, MAINTENANCE, LIVENESS, DRILL_POSTBACK];

/// The table's job with `id`, or `None` when the table holds none.
#[must_use]
pub fn job(id: &str) -> Option<Job> {
    TABLE.into_iter().find(|job| job.id == id)
}

impl Job {
    /// Whether the job fires at most once a day, so its runs claim their fire date (R5).
    #[must_use]
    pub const fn once_a_day(&self) -> bool {
        !matches!(self.schedule, Schedule::Hourly { .. })
    }
}

impl Schedule {
    /// The local hour and minute a daily schedule fires at, the rollover hour being `rollover`;
    /// `None` for an hourly one.
    #[must_use]
    pub const fn daily_slot(self, rollover: Hour) -> Option<(u8, u8)> {
        match self {
            Self::DailyAtRollover { minute } => Some((rollover.get(), minute)),
            Self::DailyAt { hour, minute } => Some((hour, minute)),
            Self::Hourly { .. } => None,
        }
    }

    /// The minute of the hour the schedule fires at.
    #[must_use]
    pub const fn minute(self) -> u8 {
        match self {
            Self::DailyAtRollover { minute }
            | Self::DailyAt { minute, .. }
            | Self::Hourly { minute } => minute,
        }
    }

    /// The latest instant the schedule fires at, at or before `now`, local to `rule`'s offset.
    #[must_use]
    pub fn latest_at_or_before(self, now: UtcMillis, rule: StudyDayRule) -> UtcMillis {
        let offset = offset_millis(rule.utc_offset());
        let local = now.epoch_millis().saturating_add(offset);
        let fire = if let Some((hour, minute)) = self.daily_slot(rule.rollover_hour()) {
            let slot = local.div_euclid(DAY_MS) * DAY_MS
                + i64::from(hour) * HOUR_MS
                + i64::from(minute) * MINUTE_MS;
            if slot <= local { slot } else { slot - DAY_MS }
        } else {
            let minute = i64::from(self.minute()) * MINUTE_MS;
            (local - minute).div_euclid(HOUR_MS) * HOUR_MS + minute
        };
        UtcMillis::from_epoch_millis(fire.saturating_sub(offset))
    }

    /// The latest instant the schedule fires at between the start of `now`'s local calendar day
    /// and `now`, both included, or `None` when it has not fired yet that day: the predecessor's
    /// `scheduler.py:_latest_elapsed_fire_today`.
    #[must_use]
    pub fn latest_elapsed_today(self, now: UtcMillis, rule: StudyDayRule) -> Option<UtcMillis> {
        let fire = self.latest_at_or_before(now, rule);
        let offset = rule.utc_offset();
        (FireDate::of(fire, offset) == FireDate::of(now, offset)).then_some(fire)
    }
}

/// The local calendar date of a scheduled fire under the configured offset, as an epoch day
/// number: the ledger's `fire_date` (R3). A calendar date, deliberately not a study day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FireDate(i64);

impl FireDate {
    /// The date with epoch day number `day`.
    #[must_use]
    pub const fn from_epoch_day(day: i64) -> Self {
        Self(day)
    }

    /// The epoch day number.
    #[must_use]
    pub const fn epoch_day(self) -> i64 {
        self.0
    }

    /// The local calendar date `instant` falls on, `offset` east of UTC.
    #[must_use]
    pub fn of(instant: UtcMillis, offset: UtcOffset) -> Self {
        Self(
            instant
                .epoch_millis()
                .saturating_add(offset_millis(offset))
                .div_euclid(DAY_MS),
        )
    }
}

/// Whether the predecessor anchors a sync every `interval_min` minutes to the hour: the port of
/// `timebase.py:is_hour_alignable`.
#[must_use]
pub fn is_hour_alignable(interval_min: i64) -> bool {
    (TICK_ALIGN_MIN_INTERVAL_MIN..=MINUTES_PER_HOUR).contains(&interval_min)
        && MINUTES_PER_HOUR % interval_min == 0
}

/// The minutes of the hour the predecessor's sync ticks fire at, every `interval_min` minutes
/// offset by `offset_min`, or none when the interval cannot be anchored to the hour: the port of
/// `timebase.py:tick_minutes`, kept only to name the predecessor's sync minutes, which the table's
/// slots keep off (ADR-011).
#[must_use]
pub fn tick_minutes(interval_min: i64, offset_min: i64) -> Vec<i64> {
    if !is_hour_alignable(interval_min) {
        return Vec::new();
    }
    let base = offset_min.rem_euclid(interval_min);
    let span = MINUTES_PER_HOUR / interval_min;
    let mut ticks: Vec<i64> = (0..span).map(|tick| base + tick * interval_min).collect();
    ticks.sort_unstable();
    ticks
}

/// A fixed offset, in milliseconds east of UTC.
fn offset_millis(offset: UtcOffset) -> i64 {
    i64::from(offset.minutes()) * MINUTE_MS
}
