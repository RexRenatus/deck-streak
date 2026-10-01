//! The study conditions (SPEC-073 R6): each of the 24 study badges is earned on a day whose badge
//! context meets its threshold, equal to `goldens/badge_conditions.json`, with every threshold
//! equal to the predecessor's constant (`goldens/badges.constants.json`).

/// The lifetime study reviews that earn Grinder.
pub const LIFETIME_REVIEWS_GRINDER: u64 = 0;
/// The lifetime study reviews that earn Marathoner.
pub const LIFETIME_REVIEWS_MARATHONER: u64 = 0;
/// The day's study reviews that earn Centurion Day.
pub const CENTURION_DAY_REVIEWS: u64 = 0;
/// The 7-day first-answer retention, in percent, that earns Sharpshooter.
pub const SHARPSHOOTER_RETENTION: f64 = 0.0;
/// The 7-day study reviews Sharpshooter needs beside its retention.
pub const SHARPSHOOTER_MIN_REVIEWS: u64 = 0;
/// The 30-day mature first-answer retention, in percent, that earns Sniper Elite.
pub const SNIPER_ELITE_RETENTION: f64 = 0.0;
/// The 30-day mature answers Sniper Elite needs beside its retention.
pub const SNIPER_ELITE_MIN_MATURE: u64 = 0;
/// The reviews cleared with no backlog left that earn Backlog Slayer.
pub const BACKLOG_SLAYER_CLEARED: u64 = 0;
/// The reviews between midnight and the rollover hour that earn Night Owl.
pub const NIGHT_OWL_REVIEWS: u64 = 0;
/// The reviews between the rollover hour and the early-bird hour that earn Early Bird.
pub const EARLY_BIRD_REVIEWS: u64 = 0;
/// The local hour the early-bird window ends at, not included.
pub const EARLY_BIRD_END_HOUR: u8 = 0;
/// The mature cards that earn Maturity Milestone.
pub const MATURITY_MILESTONE_COUNT: i64 = 0;
/// The mature cards that earn Forest Guardian.
pub const FOREST_GUARDIAN_COUNT: i64 = 0;
/// The decks studied in one day that earn Polyglot.
pub const POLYGLOT_DECKS_DAY: u64 = 0;
/// The decks studied in 7 days that earn Globetrotter.
pub const GLOBETROTTER_DECKS_WEEK: u64 = 0;
/// The score each of 7 days must reach for Perfect Week.
pub const PERFECT_WEEK_SCORE: i64 = 0;
/// The day's study reviews Speed Demon needs.
pub const SPEED_DEMON_REVIEWS: u64 = 0;
/// The average answer, in seconds, Speed Demon must stay under.
pub const SPEED_DEMON_AVG_SECONDS: f64 = 0.0;
/// The study days in a row, ending on the day, that earn Iron Will.
pub const IRON_WILL_DAYS: u64 = 0;
/// The lifetime study reviews Leech Tamer needs beside no active leech.
pub const LEECH_TAMER_MIN_LIFETIME: u64 = 0;
/// The score of a legendary day.
pub const LEGENDARY_DAY_SCORE: i64 = 0;
/// The scores Perfect Week reads.
pub const PERFECT_WEEK_DAYS: usize = 0;

/// The 24 study badges' keys, in the catalog's order.
pub const STUDY_KEYS: [&str; 24] = [
    "first_steps",
    "week_warrior",
    "monthly_monk",
    "century_flame",
    "year_of_iron",
    "grinder",
    "marathoner",
    "centurion_day",
    "sharpshooter",
    "sniper_elite",
    "inbox_zero",
    "backlog_slayer",
    "night_owl",
    "early_bird",
    "comeback_kid",
    "maturity_milestone",
    "forest_guardian",
    "leech_tamer",
    "polyglot",
    "globetrotter",
    "perfect_week",
    "legendary_day",
    "speed_demon",
    "iron_will",
];

/// The end-of-day card snapshot a day is judged with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// The mature cards.
    pub mature_count: i64,
    /// The active leeches.
    pub leech_active: i64,
    /// The overdue cards.
    pub backlog: i64,
    /// The cards due today.
    pub due_today: i64,
}

/// What one evaluated day's study conditions read (R5).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BadgeContext {
    /// The study reviews ever made, through the day.
    pub lifetime: u64,
    /// The language streak's current run.
    pub streak_current: u64,
    /// Whether the streak's comeback is armed and the run has restarted.
    pub comeback_armed: bool,
    /// The day's study reviews.
    pub day_reviews: u64,
    /// The decks studied on the day.
    pub day_decks: u64,
    /// The day's average answer, in seconds.
    pub day_avg_seconds: f64,
    /// The day's card snapshot.
    pub snapshot: Snapshot,
    /// The day's score.
    pub score_total: i64,
    /// The scores of the 7 most recent rollups on or before the day, oldest first.
    pub week_scores: Vec<i64>,
    /// The study reviews of the 7 days ending on the day.
    pub week_reviews: u64,
    /// The decks studied in the 7 days ending on the day.
    pub week_decks: u64,
    /// The first-answer retention of the 7 days ending on the day, in percent.
    pub week_retention: f64,
    /// The mature first answers of the 30 days ending on the day.
    pub mature30_answered: u64,
    /// Their retention, in percent.
    pub mature30_retention: f64,
    /// The day's reviews between midnight and the rollover hour.
    pub night_owl: u64,
    /// The day's reviews between the rollover hour and [`EARLY_BIRD_END_HOUR`].
    pub early_bird: u64,
    /// The reviews cleared with no backlog left, counted only at [`BACKLOG_SLAYER_CLEARED`].
    pub cleared_backlog: u64,
    /// Whether each of the [`IRON_WILL_DAYS`] days ending on the day is a study day.
    pub iron_will_ok: bool,
}

/// Each study badge's condition over `context`, in [`STUDY_KEYS`]' order.
#[must_use]
pub fn conditions(context: &BadgeContext) -> [(&'static str, bool); 24] {
    let _ = context;
    STUDY_KEYS.map(|key| (key, false))
}
