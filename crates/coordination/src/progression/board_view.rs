//! The personal board's view (SPEC-075 R1, #79): the board the API's `GET /api/board` answers,
//! read from the learner's own rollups, language streak and XP total, and never from another
//! person's.
//!
//! Each row comes from one statement: the best day and today from the most recent rollups, the
//! streak from the language streak, the level from both XP tables' total. No row joins two reads,
//! so the board needs no read transaction (ADR-075).

use deck_streak_kernel::{Db, KernelError, StudyDay};
use deck_streak_progression::board::{BoardRow, BoardStreak, DayScore, board_rows};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::level::level_info;
use deck_streak_streaks::{store, streak::StreakState};

/// How many of the most recent stored rollups the best day is chosen from, as the predecessor's
/// `ReadApiLayer.leaderboard` reads them. A read-window parameter, never an economy constant.
pub const BOARD_ROLLUPS: i64 = 365;

/// The board for `today`, its rows in the order the board shows them.
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn board_view(db: &Db, today: StudyDay) -> Result<Vec<BoardRow>, KernelError> {
    let total = SqliteXpLedger::new(db.clone()).total().await?;
    let mut read = db.reader().acquire().await?;
    let totals: Vec<DayScore> =
        deck_streak_analytics::rollup::recent_totals(&mut read, today, BOARD_ROLLUPS)
            .await?
            .into_iter()
            .map(|total| DayScore {
                day: total.day,
                score: total.score,
            })
            .collect();
    let streak = store::state(&mut read, "language")
        .await?
        .unwrap_or_else(StreakState::start);
    let streak = BoardStreak {
        current: streak.current,
        longest: streak.longest,
    };
    Ok(board_rows(&totals, today, streak, &level_info(total)))
}
