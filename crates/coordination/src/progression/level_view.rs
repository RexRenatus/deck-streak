//! The level view and the law-tiers view (SPEC-072 R23, R24, R25): the owner's level, today's XP
//! by source and track, the run and its multiplier, the one-miss preview, and whether today is an
//! Ascendant day.
//!
//! Every rule is progression's; this module reads what the fold settled and says what it holds.
//! The API's `GET /api/level` and the bot's `/level` both read [`level_view`], so the numbers
//! cannot drift between them.

use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

use deck_streak_analytics::rollup::stored;
use deck_streak_ingest::window::INGEST_WINDOW_DAYS;
use deck_streak_kernel::{Db, KernelError, StudyDay};
use deck_streak_progression::buffs::is_ascendant_day;
use deck_streak_progression::consistency::{on_pace_run, projected_multiplier_drop};
use deck_streak_progression::economy_config::xp;
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::level::{LevelInfo, level_info};
use deck_streak_progression::{SettledRow, settled_of_day};

/// The owner's level and the day's XP, as the surfaces show them.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelView {
    /// The study day the view is for.
    pub study_day: StudyDay,
    /// The total's place on the level curve.
    pub info: LevelInfo,
    /// The day's settled rows; a row whose `closed` is false is provisional.
    pub today: Vec<SettledRow>,
    /// The run of on-pace days before the day.
    pub run: i64,
    /// The consistency multiplier the run earns.
    pub multiplier: f64,
    /// The multiplier a missed day would leave.
    pub multiplier_after_a_miss: f64,
    /// Whether the day is an Ascendant day.
    pub ascendant: bool,
}

/// The view for `today`.
///
/// # Errors
///
/// [`KernelError::Database`] when a read fails.
pub async fn level_view(db: &Db, today: StudyDay) -> Result<LevelView, KernelError> {
    let total = SqliteXpLedger::new(db.clone()).total().await?;
    let mut connection = db.reader().acquire().await?;
    let rows = settled_of_day(&mut connection, today).await?;
    let ascendant = is_ascendant_day(&mut connection, today).await?;
    let window = xp().window_days;
    let first = today.epoch_day() - INGEST_WINDOW_DAYS;
    let mut scores = Vec::new();
    let mut number = today.epoch_day();
    while scores.len() < window && number >= first {
        if let Some(rolled) = stored(&mut connection, StudyDay::from_epoch_day(number)).await? {
            scores.push((number, rolled.score.total));
        }
        number -= 1;
    }
    let run = on_pace_run(&scores, &BTreeSet::new(), today.epoch_day());
    let (multiplier, multiplier_after_a_miss) = projected_multiplier_drop(run);
    Ok(LevelView {
        study_day: today,
        info: level_info(total),
        today: rows,
        run,
        multiplier,
        multiplier_after_a_miss,
        ascendant,
    })
}

/// The law-track cards in scope counted by tier, and today's law review XP split by the tier of
/// each review's card (R24).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LawTiers {
    /// Cards per tier, `T1` to `T4`, then the cards with none.
    pub cards: [u64; 5],
    /// Today's `reviews_law` XP per tier, in the same order.
    pub xp: [u64; 5],
}

/// Where the law tiers come from: the collection, which only the cycle that reads it can open.
pub trait LawTierSource: Send + Sync + std::fmt::Debug {
    /// The law tiers as of `today`.
    fn law_tiers<'a>(
        &'a self,
        today: StudyDay,
    ) -> Pin<Box<dyn Future<Output = Result<LawTiers, KernelError>> + Send + 'a>>;
}
