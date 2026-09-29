//! The streak, freeze, strength, governor and relight constants (SPEC-076 R1 to R13). Each equals
//! the predecessor's value at `27ee2bc` and `economy.json`'s, by the test
//! `the_streak_constants_and_economy_json_match_the_predecessors`.

/// Freezes a new track starts with.
pub const STREAK_START_FREEZES: u32 = 1;
/// Freezes held at most.
pub const STREAK_FREEZE_CAP: u32 = 3;
/// Streak days between two earned freezes.
pub const STREAK_DAYS_PER_FREEZE: u32 = 7;
/// The shortest broken streak that arms the comeback.
pub const STREAK_COMEBACK_MIN: u32 = 7;
/// The heat tiers, longest first: a streak of at least the days wears the emoji.
pub const STREAK_HEAT: [(u32, &str); 6] = [
    (365, "\u{1f31f}\u{2604}\u{fe0f}\u{1f31f}"),
    (100, "\u{2604}\u{fe0f}"),
    (30, "\u{1f525}\u{1f525}\u{1f525}"),
    (7, "\u{1f525}\u{1f525}"),
    (1, "\u{1f525}"),
    (0, ""),
];
/// Drop-style freezes a calendar month admits.
pub const FREEZE_DROP_MONTHLY_CAP: u32 = 1;
/// Days in which habit strength halves.
pub const STRENGTH_HALF_LIFE_DAYS: u32 = 13;
/// The strength below which the governor stands by.
pub const STRENGTH_ARM_THRESHOLD: f64 = 0.6;
/// Reviews a return day needs to relight.
pub const RELIGHT_CARDS: u32 = 3;
/// XP a relight grants.
pub const RELIGHT_XP: u32 = 100;
/// Days the silence walk reads at most.
pub const SILENCE_WALK_CAP_DAYS: i64 = 120;
/// Days of strength the first run stores.
pub const STRENGTH_PERSIST_DAYS: i64 = 400;
/// Days a standby notice waits after the last.
pub const STANDBY_NOTICE_GAP_DAYS: i64 = 7;
