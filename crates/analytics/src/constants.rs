//! The analytics constants the port uses verbatim (SPEC-071 R6, R12; CHARTER 8): the predecessor's
//! `constants.py` at `27ee2bc`, proved by `goldens/analytics.constants.json`, never re-derived.
//!
//! A number the predecessor writes as a literal inside a function (the consistency pillar's
//! weights, the volume baseline's floors, the baseline window's 31 and 30) is not a constant there
//! and is not one here: those live beside their function and are proved by that function's golden.

/// Each answer's time is capped at this many seconds before it is summed.
pub const ANSWER_TIME_CAP_SECONDS: f64 = 0.0;
/// An interval of this many days or more is mature: the young and mature split, and graduation.
pub const MATURE_IVL_DAYS: i64 = 0;
/// The consistency pillar's weight in the total.
pub const WEIGHT_CONSISTENCY: f64 = 0.0;
/// The retention pillar's weight in the total.
pub const WEIGHT_RETENTION: f64 = 0.0;
/// The workload pillar's weight in the total.
pub const WEIGHT_WORKLOAD: f64 = 0.0;
/// The volume pillar's weight in the total.
pub const WEIGHT_VOLUME: f64 = 0.0;
/// The mastery pillar's weight in the total.
pub const WEIGHT_MASTERY: f64 = 0.0;
/// The true retention, in percent, at which the retention pillar starts to score.
pub const RETENTION_FLOOR_PCT: f64 = 0.0;
/// The true retention, in percent, at which the retention pillar is full.
pub const RETENTION_CEIL_PCT: f64 = 0.0;
/// Below this many answered cards, the retention pillar is blended toward its target.
pub const RETENTION_MIN_SAMPLE: i64 = 0;
/// What a small sample's retention pillar is blended toward.
pub const RETENTION_BLEND_TARGET: f64 = 0.0;
/// The reviews' share of the volume pillar.
pub const VOLUME_REVIEW_WEIGHT: f64 = 0.0;
/// The minutes' share of the volume pillar.
pub const VOLUME_TIME_WEIGHT: f64 = 0.0;
/// The ratio to the baseline at which the volume pillar is full.
pub const VOLUME_CAP_RATIO: f64 = 0.0;
/// The graduations that fill the mastery pillar's reward.
pub const MASTERY_GRADUATION_TARGET: i64 = 0;
/// The mastery penalty of each active leech.
pub const MASTERY_LEECH_PENALTY: f64 = 0.0;
/// The most the leeches take off the mastery pillar.
pub const MASTERY_LEECH_PENALTY_CAP: f64 = 0.0;
/// The backlog is divided by this for the workload penalty.
pub const WORKLOAD_BACKLOG_DIVISOR: f64 = 0.0;
/// The most the backlog takes off the workload pillar.
pub const WORKLOAD_BACKLOG_PENALTY_CAP: f64 = 0.0;

/// One grade band: a total at or above `threshold` takes its label and emoji.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GradeBand {
    /// The lowest total in the band.
    pub threshold: i64,
    /// The band's label.
    pub label: &'static str,
    /// The band's emoji.
    pub emoji: &'static str,
}

/// The grade bands, highest first: a total takes the first band it reaches.
pub const GRADE_BANDS: [GradeBand; 6] = [
    GradeBand {
        threshold: 0,
        label: "",
        emoji: "",
    },
    GradeBand {
        threshold: 0,
        label: "",
        emoji: "",
    },
    GradeBand {
        threshold: 0,
        label: "",
        emoji: "",
    },
    GradeBand {
        threshold: 0,
        label: "",
        emoji: "",
    },
    GradeBand {
        threshold: 0,
        label: "",
        emoji: "",
    },
    GradeBand {
        threshold: 0,
        label: "",
        emoji: "",
    },
];
