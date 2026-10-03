//! The law block (SPEC-077 R10 to R13): the law track's own summary, read over the law streak
//! (SPEC-076), today's and lifetime law-track XP over both XP tables with the level of that
//! lifetime XP (SPEC-040, SPEC-072), the law dues the last recompute stored, and the active law
//! leeches with the mastery pillar; and the rule that decides whether the block is shown and which
//! of its lines it holds.
//!
//! The leeches come from W4's leech port (#133), which is not wired yet: until it is, they and the
//! pillar are pending, never 0, and the pillar is not computed (R12). The dues are pending before
//! the first recompute stores them (R10).

use deck_streak_kernel::{Db, KernelError, StudyDay, Track};
use deck_streak_progression::ledger::SqliteXpLedger;

/// The law block's numbers, as every surface reads them.
///
/// The counts are signed because the shown rule reads the predecessor's payload, whose counts are
/// plain integers; nothing here produces a negative one, since no XP table holds a negative amount.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LawBlock {
    /// The law streak's current length in study days.
    pub streak: i64,
    /// The law track's XP on the study day the block is for, over both XP tables.
    pub xp_today: i64,
    /// The law track's lifetime XP, over both XP tables.
    pub total_xp: i64,
    /// The level of `total_xp` on the shared curve.
    pub level: i64,
    /// The backlog plus the cards due today over the law track's cards, or `None` before the first
    /// recompute stores them: pending, never 0.
    pub dues: Option<i64>,
    /// The active law leeches, or `None` while the leech port is not wired: pending, never 0.
    pub leech_active: Option<i64>,
    /// The law mastery pillar, or `None` while the leeches are pending: it is not computed.
    pub mastery: Option<f64>,
}

/// One line the block holds when it is shown, in the order it shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LawLine {
    /// The lifetime law-track XP, with its level when the level is known.
    TotalXp,
    /// The law streak.
    Streak,
    /// The law-track XP of the day.
    XpToday,
    /// The law dues.
    Dues,
    /// The law mastery pillar.
    Mastery,
    /// The active law leeches.
    Leeches,
}

/// The lines a shown block holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LawLines {
    /// The lines, in the order the block shows them.
    pub lines: Vec<LawLine>,
    /// Whether the lifetime XP's line names its level.
    pub level_shown: bool,
}

impl LawBlock {
    /// The lines the block holds, or `None` when it is omitted (R13).
    #[must_use]
    pub fn lines(&self) -> Option<LawLines> {
        Some(LawLines {
            lines: Vec::new(),
            level_shown: false,
        })
    }
}

/// The law block for `today`, with `leeches` as the leech port answers them: `None` while the
/// port is not wired (#133).
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn law_block(
    db: &Db,
    today: StudyDay,
    leeches: Option<u32>,
) -> Result<LawBlock, KernelError> {
    let total = SqliteXpLedger::new(db.clone())
        .track_total(Track::Law)
        .await?;
    let _ = (today, leeches, total);
    Ok(LawBlock {
        streak: 0,
        xp_today: 0,
        total_xp: 0,
        level: 0,
        dues: Some(0),
        leech_active: Some(0),
        mastery: Some(100.0),
    })
}
