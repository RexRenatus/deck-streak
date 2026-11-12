//! The personal board (SPEC-075 R1, #79): the learner ranked against their own past and never
//! against another person, as the predecessor's `ReadApiLayer.leaderboard` ranks it.
//!
//! The board is four rows at most: the best day among the stored rollups the caller read, today's
//! score, the language streak and the level. The best day and today appear only when a rollup was
//! read. Everything here is pure: the caller reads the rollups, the streak and the total, and this
//! module only ranks and labels them.

use deck_streak_kernel::StudyDay;

use crate::level::LevelInfo;

/// One stored day's score, as the board ranks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayScore {
    /// The study day.
    pub day: StudyDay,
    /// Its stored score.
    pub score: i64,
}

/// The streak the board shows: the language streak's run and its longest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardStreak {
    /// The current run, in days.
    pub current: u32,
    /// The longest run, in days.
    pub longest: u32,
}

/// One row of the board, in the order the board shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardRow {
    /// The highest stored score, and the day that set it.
    BestDay {
        /// The score.
        score: i64,
        /// The day that set it.
        day: StudyDay,
    },
    /// Today's stored score, 0 when today has no rollup.
    Today {
        /// The score.
        score: i64,
        /// Today.
        day: StudyDay,
    },
    /// The language streak.
    Streak {
        /// The current run.
        current: u32,
        /// The longest run.
        longest: u32,
    },
    /// The level the total reaches, and its title.
    Level {
        /// The level.
        level: u32,
        /// Its title.
        title: &'static str,
    },
}

impl BoardRow {
    /// The row's kind, as the API names it: a surface that cannot name this type reads it here.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::BestDay { .. } => "best_day",
            Self::Today { .. } => "today",
            Self::Streak { .. } => "streak",
            Self::Level { .. } => "level",
        }
    }

    /// The row's emoji, apart from its label so a screen can hide it from assistive technology.
    #[must_use]
    pub const fn emoji(&self) -> &'static str {
        match self {
            Self::BestDay { .. } => "\u{1f3c5}",
            Self::Today { .. } => "\u{1f4c5}",
            Self::Streak { .. } => "\u{1f525}",
            Self::Level { .. } => "\u{26a1}",
        }
    }

    /// The row's label, the predecessor's words without the emoji.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::BestDay { .. } => "Best day",
            Self::Today { .. } => "Today",
            Self::Streak { .. } => "Streak",
            Self::Level { .. } => "Level",
        }
    }

    /// The row's number: a score, the current run or the level.
    #[must_use]
    pub fn value(&self) -> i64 {
        match *self {
            Self::BestDay { score, .. } | Self::Today { score, .. } => score,
            Self::Streak { current, .. } => i64::from(current),
            Self::Level { level, .. } => i64::from(level),
        }
    }

    /// The day a score row is of; none for the streak and the level.
    #[must_use]
    pub const fn study_day(&self) -> Option<StudyDay> {
        match *self {
            Self::BestDay { day, .. } | Self::Today { day, .. } => Some(day),
            Self::Streak { .. } | Self::Level { .. } => None,
        }
    }

    /// The longest run, on the streak row alone.
    #[must_use]
    pub const fn longest(&self) -> Option<u32> {
        match *self {
            Self::Streak { longest, .. } => Some(longest),
            _ => None,
        }
    }

    /// The level's title, on the level row alone.
    #[must_use]
    pub const fn title(&self) -> Option<&'static str> {
        match *self {
            Self::Level { title, .. } => Some(title),
            _ => None,
        }
    }
}

/// The best day among `totals`, which are most recent first: the highest score, and on a tie the
/// most recent of the tied days, as Python's `max` keeps the first maximum it meets.
#[must_use]
pub fn best_day(totals: &[DayScore]) -> Option<DayScore> {
    let (first, rest) = totals.split_first()?;
    let mut best = *first;
    for total in rest {
        if total.score > best.score {
            best = *total;
        }
    }
    Some(best)
}

/// The board of `totals` (most recent first), `today`, the language `streak` and the `level` the
/// total reaches.
#[must_use]
pub fn board_rows(
    totals: &[DayScore],
    today: StudyDay,
    streak: BoardStreak,
    level: &LevelInfo,
) -> Vec<BoardRow> {
    let mut rows = Vec::with_capacity(4);
    if let Some(best) = best_day(totals) {
        rows.push(BoardRow::BestDay {
            score: best.score,
            day: best.day,
        });
        let score = totals
            .iter()
            .find(|total| total.day == today)
            .map_or(0, |total| total.score);
        rows.push(BoardRow::Today { score, day: today });
    }
    rows.push(BoardRow::Streak {
        current: streak.current,
        longest: streak.longest,
    });
    rows.push(BoardRow::Level {
        level: level.level.get(),
        title: level.title,
    });
    rows
}
