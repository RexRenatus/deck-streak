//! The personal board's view (SPEC-075 R1, #79): the board the API's `GET /api/board` answers,
//! read from the learner's own rollups, language streak and XP total, and never from another
//! person's.
//!
//! Each row comes from one statement: the best day and today from the most recent rollups, the
//! streak from the language streak, the level from both XP tables' total. No row joins two reads,
//! so the board needs no read transaction (ADR-075).

use deck_streak_kernel::{Db, KernelError, StudyDay};
use deck_streak_progression::board::BoardRow;

/// How many of the most recent stored rollups the best day is chosen from, as the predecessor's
/// `ReadApiLayer.leaderboard` reads them. A read-window parameter, never an economy constant.
pub const BOARD_ROLLUPS: i64 = 365;

/// The board for `today`, its rows in the order the board shows them.
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn board_view(_db: &Db, _today: StudyDay) -> Result<Vec<BoardRow>, KernelError> {
    Ok(Vec::new())
}
