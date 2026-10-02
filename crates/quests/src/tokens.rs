//! The double-XP token's constants (SPEC-081 R12, R13). Part 3 adds its rules.

/// How long an activated token's window lasts, in hours.
pub const TOKEN_WINDOW_HOURS: i64 = 2;
/// The most a token's bonus can pay on one study day, in XP.
pub const TOKEN_BONUS_CAP_XP: i64 = 300;
