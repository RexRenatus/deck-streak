//! The double-XP token's constants (SPEC-081 R12, R13). Part 3 adds its rules.

use deck_streak_ingest::reader::Review;
use deck_streak_kernel::{StudyDay, StudyDayRule, Track, UtcMillis};
use sqlx::SqliteConnection;

use crate::chests::ChestError;

/// How long an activated token's window lasts, in hours.
pub const TOKEN_WINDOW_HOURS: i64 = 2;
/// The most a token's bonus can pay on one study day, in XP.
pub const TOKEN_BONUS_CAP_XP: i64 = 300;

/// An activated token's window: from its activation instant, up to and not including its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenWindow {
    /// The activation instant, the window's first.
    pub start: UtcMillis,
    /// The window's end, its first instant outside it.
    pub end: UtcMillis,
}

/// What an activation answered (R12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activation {
    /// The oldest held token is activated with its window.
    Activated {
        /// The token activated.
        token_id: i64,
        /// Its window.
        window: TokenWindow,
    },
    /// Another token's window is open: nothing changed.
    AlreadyActive,
    /// No token is held: nothing changed.
    NoneHeld,
}

/// A study review with the XP the caller reckoned for it at the base rate: this context computes
/// no XP (docs/CONTEXT-MAP.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewXp {
    /// The review.
    pub review: Review,
    /// Its XP at the base rate.
    pub base_xp: i64,
}

/// A token's bonus on one study day, for the caller to settle through the settle port as the
/// derived source `2x:<token id>` (R13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenBonus {
    /// The token whose window earned it.
    pub token_id: i64,
    /// The study day it is settled on.
    pub study_day: StudyDay,
    /// The XP it settles.
    pub xp: i64,
    /// The track it settles on: `language`.
    pub track: Track,
}

impl TokenBonus {
    /// The settled source: `2x:<token id>`.
    #[must_use]
    pub fn source(self) -> String {
        format!("2x:{}", self.token_id)
    }
}

/// What a study day's token settlement answered: the bonuses to settle and the tokens consumed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenSettlement {
    /// Each activated token's bonus on the study day, for a token with a windowed review.
    pub bonuses: Vec<TokenBonus>,
    /// The tokens whose window ended, marked consumed by this settlement.
    pub consumed: Vec<i64>,
}

/// Activates the oldest held token at `now` (R12), in the caller's write. RED-FIRST STUB: answers
/// that no token is held.
///
/// # Errors
///
/// A store error as the chest store answers it.
pub async fn activate_token_on(
    _connection: &mut SqliteConnection,
    _now: UtcMillis,
) -> Result<Activation, ChestError> {
    Ok(Activation::NoneHeld)
}

/// A token's bonus on `study_day` (R13). RED-FIRST STUB: settles nothing.
#[must_use]
pub fn token_bonus_xp(
    _window: TokenWindow,
    _study_day: StudyDay,
    _rule: StudyDayRule,
    _reviews: &[ReviewXp],
) -> Option<i64> {
    None
}

/// Settles every activated token's bonus on `study_day` and consumes each token whose window has
/// ended by `now` (R13), in the caller's write. RED-FIRST STUB: settles and consumes nothing.
///
/// # Errors
///
/// A store error as the chest store answers it.
pub async fn settle_token_bonuses_on(
    _connection: &mut SqliteConnection,
    _study_day: StudyDay,
    _rule: StudyDayRule,
    _reviews: &[ReviewXp],
    _now: UtcMillis,
) -> Result<TokenSettlement, ChestError> {
    Ok(TokenSettlement::default())
}
