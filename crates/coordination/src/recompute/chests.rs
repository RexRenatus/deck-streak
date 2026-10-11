//! The quests context's fold steps (SPEC-411 R2 to R6; ADR-425): phase 4's chest step, which
//! grants a study day's session chests at its settle and while it is current, and its sweep, which
//! pays the stale chests of the days before today; and phase 5's token-bonus step, which settles
//! each double-XP window's bonus under its own source. Each step's work is one free function, and
//! everything a step pays it writes in the day's write.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use deck_streak_analytics::constants::ANSWER_TIME_CAP_SECONDS;
use deck_streak_ingest::reader::{Card, Review, is_study_event};
use deck_streak_ingest::tier::Tier;
use deck_streak_kernel::{Hour, KernelError, PortFuture, Track};
use deck_streak_notifications::Policy;
use deck_streak_notifications::quiet::local_minute;
use deck_streak_notifications::router::in_quiet_window_on;
use deck_streak_progression::buffs::is_ascendant_day;
use deck_streak_progression::review_xp::review_xp;
use deck_streak_progression::settle::{SettleCause, SettleError, SettleRequest, settle};
use deck_streak_quests::chests::{
    ChestError, EarnedSession, SessionChestGrant, grant_session_chests_on, sweep_stale_chests_on,
};
use deck_streak_quests::draw::{Draw, DrawError, OsDraw};
use deck_streak_quests::sessions::{SESSION_GAP_MS, Session, sessions_from_reviews};
use deck_streak_quests::tokens::{ReviewXp, settle_token_bonuses_on};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Evaluation, Phase};
use crate::chests::{chest_error, pay};
use crate::skip::days::skip_days;

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

/// Grants `day`'s session chests from `draw`, in the day's write (SPEC-411 R2 to R4).
///
/// Only a day's settle and its current evaluation roll: a backfilled or revisited day rolls
/// nothing. While the day is current, a session rolls only once it ended at least
/// [`SESSION_GAP_MS`] before now, because an open session's partial base XP would cap its payout.
/// Each session is granted on its own, in start order, so a refused draw names the session it was
/// taken for: it is logged, the day's other writes stay, and the next fold grants that session.
async fn grant_day_chests<D: Draw + Send>(
    draw: &Mutex<D>,
    day: &DayEvaluation<'_>,
    write: &mut SqliteConnection,
) -> Result<(), KernelError> {
    if !matches!(
        day.evaluation,
        Evaluation::Settle { .. } | Evaluation::Current
    ) {
        return Ok(());
    }
    let facts = day.facts;
    let current = day.evaluation == Evaluation::Current;
    let reviews = facts.reviews_of(day.day);
    let tiers = tiers_by_card(&facts.data.cards);
    let sessions: Vec<EarnedSession> = sessions_from_reviews(reviews, ANSWER_TIME_CAP_SECONDS)
        .into_iter()
        .filter(|session| {
            !current || session.end.epoch_millis() + SESSION_GAP_MS <= facts.now.epoch_millis()
        })
        .map(|session| EarnedSession {
            session,
            base_xp: session_base_xp(reviews, &tiers, session),
        })
        .collect();
    if sessions.is_empty() {
        return Ok(());
    }
    let offset = facts.rule.utc_offset();
    let policy = Policy::compiled().map_err(|error| refused(&error.to_string()))?;
    let skip_day = skip_days(write).await?.contains(&day.day);
    let ascendant = is_ascendant_day(write, day.day).await?;
    let local_hour = u8::try_from(local_minute(facts.now, offset) / 60)
        .ok()
        .and_then(Hour::new)
        .ok_or_else(|| refused("a local minute of the day is not an hour of the day"))?;
    let quiet = in_quiet_window_on(write, &policy, offset, facts.now).await?;
    for earned in &sessions {
        let request = SessionChestGrant {
            study_day: day.day,
            sessions: std::slice::from_ref(earned),
            skip_day,
            ascendant,
            local_hour,
            quiet,
        };
        match grant_session_chests_on(write, &request, &mut Locked(draw), facts.now).await {
            Ok(_) => {}
            Err(ChestError::Draw(error)) => {
                tracing::warn!(
                    study_day = day.day.epoch_day(),
                    session_start = earned.session.start.epoch_millis(),
                    %error,
                    "a chest's draw failed: the session's chest waits for the next fold"
                );
                return Ok(());
            }
            Err(other) => return Err(chest_error(other)),
        }
    }
    Ok(())
}

/// Resolves the stale chests before `day` and pays each on its own study day and track, in the
/// day's write (SPEC-411 R5). Only the current day sweeps.
async fn sweep_and_pay(
    day: &DayEvaluation<'_>,
    write: &mut SqliteConnection,
) -> Result<(), KernelError> {
    if day.evaluation != Evaluation::Current {
        return Ok(());
    }
    for swept in sweep_stale_chests_on(write, day.day)
        .await
        .map_err(chest_error)?
    {
        if let Some(payout) = swept.payout {
            pay(write, payout, day.facts.now).await?;
        }
    }
    Ok(())
}

/// Settles each double-XP window's bonus on `day` under its own `2x:<id>` source, in the day's
/// write (SPEC-411 R6): replaced while the day is current, and only ever raised once it is closed.
async fn settle_day_token_bonuses(
    day: &DayEvaluation<'_>,
    write: &mut SqliteConnection,
) -> Result<(), KernelError> {
    let facts = day.facts;
    let tiers = tiers_by_card(&facts.data.cards);
    let reviews: Vec<ReviewXp> = facts
        .reviews_of(day.day)
        .iter()
        .map(|review| ReviewXp {
            review: *review,
            base_xp: base_xp(review, &tiers),
        })
        .collect();
    let settlement = settle_token_bonuses_on(write, day.day, facts.rule, &reviews, facts.now)
        .await
        .map_err(chest_error)?;
    for bonus in settlement.bonuses {
        let source = bonus.source();
        let amount = u32::try_from(bonus.xp)
            .map_err(|_| refused("a token's bonus is not a settle amount"))?;
        let request = SettleRequest {
            study_day: bonus.study_day,
            source: &source,
            track: bonus.track,
            amount,
            closed: day.evaluation != Evaluation::Current,
        };
        match settle(write, &request, SettleCause::Recompute, facts.now).await {
            Ok(_) => {}
            Err(SettleError::Database(error)) => return Err(KernelError::Database(error)),
            Err(SettleError::NotDerived) => {
                return Err(refused(
                    "a token's bonus source is outside the derived registry",
                ));
            }
        }
    }
    Ok(())
}

/// The draw a step holds, taken one draw at a time through its lock, so no guard is held across
/// an await.
struct Locked<'a, D>(&'a Mutex<D>);

impl<D: Draw> Draw for Locked<'_, D> {
    fn draw(&mut self) -> Result<f64, DrawError> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).draw()
    }
}

/// Each card's law tier, by id: the tier reaches only a law-track card's rate, as the XP step
/// reads it (SPEC-072 R4).
fn tiers_by_card(cards: &[Card]) -> HashMap<i64, Option<Tier>> {
    cards
        .iter()
        .map(|card| {
            let tier = (card.track == Track::Law).then_some(card.tier).flatten();
            (card.id, tier)
        })
        .collect()
}

/// `review`'s XP at the base rate.
fn base_xp(review: &Review, tiers: &HashMap<i64, Option<Tier>>) -> i64 {
    i64::from(review_xp(
        review,
        tiers.get(&review.card_id).copied().flatten(),
    ))
}

/// A session's review XP at the base rate: its study reviews' XP, which caps a Common or Rare
/// payout (SPEC-081 R4).
fn session_base_xp(
    reviews: &[Review],
    tiers: &HashMap<i64, Option<Tier>>,
    session: Session,
) -> i64 {
    let (start, end) = (session.start.epoch_millis(), session.end.epoch_millis());
    reviews
        .iter()
        .filter(|review| {
            is_study_event(review.kind, review.ease) && (start..=end).contains(&review.id)
        })
        .map(|review| base_xp(review, tiers))
        .sum()
}

/// A refusal that names its rule, as the other steps spell one.
fn refused(reason: &str) -> KernelError {
    KernelError::Database(sqlx::Error::Protocol(reason.to_owned()))
}
