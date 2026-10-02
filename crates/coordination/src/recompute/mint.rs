//! Phase 6's step (SPEC-082 R4; ADR-315): each study day the fold evaluates is minted from the
//! day's final base, in a savepoint of the day's write.

use deck_streak_kernel::PortFuture;
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

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

    fn evaluate<'a>(
        &'a self,
        _day: &'a DayEvaluation<'a>,
        _write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move { Ok(()) })
    }
}
