//! The double-XP token: its store, its activation and its bonus (SPEC-081 R9, R12, R13).
//!
//! Every function runs on the caller's connection, inside the caller's write, as the chest store
//! does (ADR-081): an activation's read of the open window and its write of the next one are one
//! step no other write can come between, so at most one window is ever open. A bonus is never
//! credited here: it is answered for the caller to settle through the settle port in that write.

use deck_streak_ingest::reader::{Review, is_study_event};
use deck_streak_kernel::{StudyDay, StudyDayRule, Track, UtcMillis};
use sqlx::SqliteConnection;

use crate::chests::ChestError;

/// How long an activated token's window lasts, in hours.
pub const TOKEN_WINDOW_HOURS: i64 = 2;
/// The most a token's bonus can pay on one study day, in XP.
pub const TOKEN_BONUS_CAP_XP: i64 = 300;

/// An hour, in milliseconds.
const HOUR_MS: i64 = 3_600_000;

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

/// Stores a token for Epic `chest_id`, granted at `at`, unless the chest already holds one, and
/// answers its id, or none when nothing was written (R9). The unique index over the chests that
/// name a token is the existence check.
///
/// # Errors
///
/// [`ChestError::Database`] when the write fails.
pub(crate) async fn grant_token(
    connection: &mut SqliteConnection,
    chest_id: i64,
    at: UtcMillis,
) -> Result<Option<i64>, ChestError> {
    let at = at.epoch_millis();
    let written = sqlx::query!(
        "INSERT INTO xp_tokens (chest_id, granted_at, created_at) VALUES (?1, ?2, ?2) \
         ON CONFLICT DO NOTHING",
        chest_id,
        at
    )
    .execute(&mut *connection)
    .await?;
    Ok((written.rows_affected() == 1).then(|| written.last_insert_rowid()))
}

/// Activates the oldest held token at `now` (R12), in the caller's write.
///
/// A token whose window is still open refuses the activation, the newest such token first, as the
/// predecessor reads it. Otherwise the oldest token never activated has its window opened, from
/// `now` for [`TOKEN_WINDOW_HOURS`]. The write is guarded on the token still being held, so a
/// token another write activated first is told apart: a window is open, and nothing changed.
///
/// # Errors
///
/// [`ChestError::Database`] when a read or the write fails.
pub async fn activate_token_on(
    connection: &mut SqliteConnection,
    now: UtcMillis,
) -> Result<Activation, ChestError> {
    let now_ms = now.epoch_millis();
    let active = sqlx::query_scalar!(
        "SELECT id FROM xp_tokens WHERE consumed = 0 AND activated_at > 0 \
         AND window_ends_at > ?1 ORDER BY id DESC LIMIT 1",
        now_ms
    )
    .fetch_optional(&mut *connection)
    .await?;
    if active.is_some() {
        return Ok(Activation::AlreadyActive);
    }
    let held = sqlx::query_scalar!(
        "SELECT id FROM xp_tokens WHERE consumed = 0 AND activated_at = 0 ORDER BY id LIMIT 1"
    )
    .fetch_optional(&mut *connection)
    .await?;
    let Some(token_id) = held else {
        return Ok(Activation::NoneHeld);
    };
    let window = TokenWindow {
        start: now,
        end: UtcMillis::from_epoch_millis(now_ms + TOKEN_WINDOW_HOURS * HOUR_MS),
    };
    let end_ms = window.end.epoch_millis();
    let written = sqlx::query!(
        "UPDATE xp_tokens SET activated_at = ?2, window_ends_at = ?3 \
         WHERE id = ?1 AND consumed = 0 AND activated_at = 0",
        token_id,
        now_ms,
        end_ms
    )
    .execute(&mut *connection)
    .await?;
    if written.rows_affected() == 1 {
        Ok(Activation::Activated { token_id, window })
    } else {
        Ok(Activation::AlreadyActive)
    }
}

/// A token's bonus on `study_day` (R13): its window's study reviews of that study day, each at
/// its XP at the base rate, summed and capped at [`TOKEN_BONUS_CAP_XP`]; none when no study
/// review falls in the window on that day. The bonus is its own source and never part of the
/// day's base, so it never compounds with another bonus.
#[must_use]
pub fn token_bonus_xp(
    window: TokenWindow,
    study_day: StudyDay,
    rule: StudyDayRule,
    reviews: &[ReviewXp],
) -> Option<i64> {
    let (start, end) = (window.start.epoch_millis(), window.end.epoch_millis());
    let mut windowed = false;
    let mut xp: i64 = 0;
    for counted in reviews {
        let review = counted.review;
        if is_study_event(review.kind, review.ease)
            && (start..end).contains(&review.id)
            && rule.study_day(UtcMillis::from_epoch_millis(review.id)) == study_day
        {
            windowed = true;
            xp = xp.saturating_add(counted.base_xp);
        }
    }
    windowed.then_some(xp.min(TOKEN_BONUS_CAP_XP))
}

/// Settles every activated token's bonus on `study_day` and consumes each token whose window has
/// ended by `now` (R13), in the caller's write.
///
/// Every activated token is read, a consumed one included, so a re-sync of the day answers the
/// same bonus again for the caller's settle to refresh, never to stack. A token whose window ended
/// before `now` is consumed once, by a guarded update.
///
/// # Errors
///
/// [`ChestError::Database`] when the read or a write fails.
pub async fn settle_token_bonuses_on(
    connection: &mut SqliteConnection,
    study_day: StudyDay,
    rule: StudyDayRule,
    reviews: &[ReviewXp],
    now: UtcMillis,
) -> Result<TokenSettlement, ChestError> {
    let tokens = sqlx::query!(
        "SELECT id, activated_at, window_ends_at, consumed FROM xp_tokens \
         WHERE activated_at > 0 ORDER BY id"
    )
    .fetch_all(&mut *connection)
    .await?;
    let mut settlement = TokenSettlement::default();
    for token in tokens {
        let window = TokenWindow {
            start: UtcMillis::from_epoch_millis(token.activated_at),
            end: UtcMillis::from_epoch_millis(token.window_ends_at),
        };
        if let Some(xp) = token_bonus_xp(window, study_day, rule, reviews) {
            settlement.bonuses.push(TokenBonus {
                token_id: token.id,
                study_day,
                xp,
                track: Track::Language,
            });
        }
        if token.window_ends_at < now.epoch_millis() && token.consumed == 0 {
            let written = sqlx::query!(
                "UPDATE xp_tokens SET consumed = 1 WHERE id = ?1 AND consumed = 0",
                token.id
            )
            .execute(&mut *connection)
            .await?;
            if written.rows_affected() == 1 {
                settlement.consumed.push(token.id);
            }
        }
    }
    Ok(settlement)
}
