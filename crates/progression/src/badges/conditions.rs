//! The study conditions (SPEC-073 R6): each of the 24 study badges is earned on a day whose badge
//! context meets its threshold, equal to `goldens/badge_conditions.json`, with every threshold
//! equal to the predecessor's constant (`goldens/badges.constants.json`).

/// The lifetime study reviews that earn Grinder.
pub const LIFETIME_REVIEWS_GRINDER: u64 = 1000;
/// The lifetime study reviews that earn Marathoner.
pub const LIFETIME_REVIEWS_MARATHONER: u64 = 10_000;
/// The day's study reviews that earn Centurion Day.
pub const CENTURION_DAY_REVIEWS: u64 = 100;
/// The 7-day first-answer retention, in percent, that earns Sharpshooter.
pub const SHARPSHOOTER_RETENTION: f64 = 90.0;
/// The 7-day study reviews Sharpshooter needs beside its retention.
pub const SHARPSHOOTER_MIN_REVIEWS: u64 = 50;
/// The 30-day mature first-answer retention, in percent, that earns Sniper Elite.
pub const SNIPER_ELITE_RETENTION: f64 = 95.0;
/// The 30-day mature answers Sniper Elite needs beside its retention.
pub const SNIPER_ELITE_MIN_MATURE: u64 = 50;
/// The reviews cleared with no backlog left that earn Backlog Slayer.
pub const BACKLOG_SLAYER_CLEARED: u64 = 200;
/// The reviews between midnight and the rollover hour that earn Night Owl.
pub const NIGHT_OWL_REVIEWS: u64 = 50;
/// The reviews between the rollover hour and the early-bird hour that earn Early Bird.
pub const EARLY_BIRD_REVIEWS: u64 = 50;
/// The local hour the early-bird window ends at, not included.
pub const EARLY_BIRD_END_HOUR: u8 = 7;
/// The mature cards that earn Maturity Milestone.
pub const MATURITY_MILESTONE_COUNT: i64 = 100;
/// The mature cards that earn Forest Guardian.
pub const FOREST_GUARDIAN_COUNT: i64 = 1000;
/// The decks studied in one day that earn Polyglot.
pub const POLYGLOT_DECKS_DAY: u64 = 3;
/// The decks studied in 7 days that earn Globetrotter.
pub const GLOBETROTTER_DECKS_WEEK: u64 = 5;
/// The score each of 7 days must reach for Perfect Week.
pub const PERFECT_WEEK_SCORE: i64 = 75;
/// The day's study reviews Speed Demon needs.
pub const SPEED_DEMON_REVIEWS: u64 = 100;
/// The average answer, in seconds, Speed Demon must stay under.
pub const SPEED_DEMON_AVG_SECONDS: f64 = 6.0;
/// The study days in a row, ending on the day, that earn Iron Will.
pub const IRON_WILL_DAYS: u64 = 30;
/// The lifetime study reviews Leech Tamer needs beside no active leech.
pub const LEECH_TAMER_MIN_LIFETIME: u64 = 500;
/// The score of a legendary day.
pub const LEGENDARY_DAY_SCORE: i64 = 100;
/// The scores Perfect Week reads.
pub const PERFECT_WEEK_DAYS: usize = 7;

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
    let streak = context.streak_current;
    let lifetime = context.lifetime;
    let snapshot = context.snapshot;
    // The predecessor reads the last seven scores it was given (`week_scores[-7:]`).
    let week = &context.week_scores[context.week_scores.len().saturating_sub(PERFECT_WEEK_DAYS)..];
    [
        ("first_steps", lifetime >= 1),
        ("week_warrior", streak >= 7),
        ("monthly_monk", streak >= 30),
        ("century_flame", streak >= 100),
        ("year_of_iron", streak >= 365),
        ("grinder", lifetime >= LIFETIME_REVIEWS_GRINDER),
        ("marathoner", lifetime >= LIFETIME_REVIEWS_MARATHONER),
        (
            "centurion_day",
            context.day_reviews >= CENTURION_DAY_REVIEWS,
        ),
        (
            "sharpshooter",
            context.week_retention >= SHARPSHOOTER_RETENTION
                && context.week_reviews >= SHARPSHOOTER_MIN_REVIEWS,
        ),
        (
            "sniper_elite",
            context.mature30_retention >= SNIPER_ELITE_RETENTION
                && context.mature30_answered >= SNIPER_ELITE_MIN_MATURE,
        ),
        (
            "inbox_zero",
            snapshot.backlog == 0 && snapshot.due_today == 0 && lifetime > 0,
        ),
        (
            "backlog_slayer",
            context.cleared_backlog >= BACKLOG_SLAYER_CLEARED,
        ),
        ("night_owl", context.night_owl >= NIGHT_OWL_REVIEWS),
        ("early_bird", context.early_bird >= EARLY_BIRD_REVIEWS),
        ("comeback_kid", context.comeback_armed),
        (
            "maturity_milestone",
            snapshot.mature_count >= MATURITY_MILESTONE_COUNT,
        ),
        (
            "forest_guardian",
            snapshot.mature_count >= FOREST_GUARDIAN_COUNT,
        ),
        (
            "leech_tamer",
            snapshot.leech_active == 0 && lifetime >= LEECH_TAMER_MIN_LIFETIME,
        ),
        ("polyglot", context.day_decks >= POLYGLOT_DECKS_DAY),
        (
            "globetrotter",
            context.week_decks >= GLOBETROTTER_DECKS_WEEK,
        ),
        (
            "perfect_week",
            week.len() >= PERFECT_WEEK_DAYS
                && week.iter().all(|score| *score >= PERFECT_WEEK_SCORE),
        ),
        ("legendary_day", context.score_total >= LEGENDARY_DAY_SCORE),
        (
            "speed_demon",
            context.day_reviews >= SPEED_DEMON_REVIEWS
                && 0.0 < context.day_avg_seconds
                && context.day_avg_seconds < SPEED_DEMON_AVG_SECONDS,
        ),
        ("iron_will", context.iron_will_ok),
    ]
}
