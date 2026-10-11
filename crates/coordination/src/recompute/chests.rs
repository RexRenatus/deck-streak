//! The quests context's fold steps (SPEC-411 R2 to R6; ADR-425): phase 4's chest step, which
//! grants a study day's session chests at its settle and while it is current, and its sweep, which
//! pays the stale chests of the days before today; and phase 5's token-bonus step, which settles
//! each double-XP window's bonus under its own source. Each step's work is one free function, and
//! everything a step pays it writes in the day's write.

use std::sync::Mutex;

use deck_streak_kernel::{KernelError, PortFuture};
use deck_streak_quests::draw::{Draw, OsDraw};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Phase};

/// The name the fold's report gives the chest step.
pub const CHEST_STEP: &str = "quests.chests";
/// The name the fold's report gives the sweep step.
pub const CHEST_SWEEP_STEP: &str = "quests.chest_sweep";
/// The name the fold's report gives the token-bonus step.
pub const TOKEN_BONUS_STEP: &str = "quests.token_bonuses";

/// The chest step: a study day's session chests, rolled from the draw it holds.
#[derive(Debug)]
pub struct ChestStep<D = OsDraw> {
    draw: Mutex<D>,
}

impl ChestStep<OsDraw> {
    /// The step over the operating system's generator, the production draw.
    #[must_use]
    pub fn new() -> Self {
        Self::with_draw(OsDraw)
    }
}

impl Default for ChestStep<OsDraw> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Draw + Send> ChestStep<D> {
    /// The step over `draw`.
    #[must_use]
    pub const fn with_draw(draw: D) -> Self {
        Self {
            draw: Mutex::new(draw),
        }
    }
}

impl<D: Draw + Send> DayStep for ChestStep<D> {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        CHEST_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(grant_day_chests(&self.draw, day, write))
    }
}

/// The sweep step: the stale chests of the days before today, each paid on its own day.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChestSweepStep;

impl DayStep for ChestSweepStep {
    fn phase(&self) -> Phase {
        Phase::DaySteps
    }

    fn name(&self) -> &'static str {
        CHEST_SWEEP_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(sweep_and_pay(day, write))
    }
}

/// The token-bonus step: each double-XP window's bonus on the day, settled under its own source.
#[derive(Clone, Copy, Debug, Default)]
pub struct TokenBonusStep;

impl DayStep for TokenBonusStep {
    fn phase(&self) -> Phase {
        Phase::DerivedBonuses
    }

    fn name(&self) -> &'static str {
        TOKEN_BONUS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(settle_day_token_bonuses(day, write))
    }
}

/// Grants `day`'s session chests from `draw`, in the day's write.
#[expect(
    clippy::unused_async,
    reason = "SPEC-411's red stub; the green awaits its write"
)]
async fn grant_day_chests<D: Draw + Send>(
    draw: &Mutex<D>,
    day: &DayEvaluation<'_>,
    write: &mut SqliteConnection,
) -> Result<(), KernelError> {
    let _ = (draw, day, write);
    Ok(())
}

/// Resolves the stale chests before `day` and pays each, in the day's write.
#[expect(
    clippy::unused_async,
    reason = "SPEC-411's red stub; the green awaits its write"
)]
async fn sweep_and_pay(
    day: &DayEvaluation<'_>,
    write: &mut SqliteConnection,
) -> Result<(), KernelError> {
    let _ = (day, write);
    Ok(())
}

/// Settles each double-XP window's bonus on `day`, in the day's write.
#[expect(
    clippy::unused_async,
    reason = "SPEC-411's red stub; the green awaits its write"
)]
async fn settle_day_token_bonuses(
    day: &DayEvaluation<'_>,
    write: &mut SqliteConnection,
) -> Result<(), KernelError> {
    let _ = (day, write);
    Ok(())
}
