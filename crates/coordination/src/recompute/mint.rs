//! Phase 6's step (SPEC-082 R4; ADR-315): each study day the fold evaluates is minted from the
//! day's final base, in a savepoint of the day's write.

use deck_streak_economy::rules::mint_for_base_xp;
use deck_streak_economy::wallet::settle_mint_on;
use deck_streak_kernel::PortFuture;
use deck_streak_progression::consistency::day_base_xp;
use deck_streak_progression::settle::day_rows;
use sqlx::{Connection, SqliteConnection};

use super::{DayEvaluation, DayStep, Evaluation, Phase};

/// The name the fold's report gives this step.
pub const MINT_STEP: &str = "economy.coin_mint";

/// Economy's mint step.
#[derive(Clone, Copy, Debug, Default)]
pub struct MintStep;

impl DayStep for MintStep {
    fn phase(&self) -> Phase {
        Phase::CoinMint
    }

    fn name(&self) -> &'static str {
        MINT_STEP
    }

    /// Mints `day` from its final base, for every evaluation the fold makes (ADR-315 ruling 2):
    /// the day's XP rows summed as phase 5 sums them, minted, and settled in a savepoint of the
    /// fold's own write, so the mint commits or rolls back with the day (ruling 1). Every day but
    /// the current one is reported closed, so a settled day's mint is only raised.
    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let closed = !matches!(day.evaluation, Evaluation::Current);
            let rows = day_rows(write, day.day).await?;
            let base = day_base_xp(
                rows.iter()
                    .map(|(source, amount)| (source.as_str(), *amount)),
            );
            let amount = mint_for_base_xp(base);
            let mut savepoint = write.begin().await?;
            let _settled =
                settle_mint_on(&mut savepoint, day.day, amount, closed, day.facts.now).await?;
            savepoint.commit().await?;
            Ok(())
        })
    }
}
