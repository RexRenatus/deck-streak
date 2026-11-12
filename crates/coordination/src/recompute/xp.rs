//! Phase 2's step (SPEC-072 R4, R5, R11, R12, R19, R20): a day's review XP and daily bonuses,
//! settled into progression's `xp_settlement` through `deck_streak_progression::settle`, and the
//! Ascendant buff armed for the day.
//!
//! The reviews settle for every evaluation of a day, closed unless the day is the current one. The
//! bonuses and the arming run where the predecessor's today-only rules do: at a day's settle and
//! while it is current. Every rule is progression's; the step only chooses which, for the day the
//! fold evaluates, and reads what the day's own rollup holds.

use std::collections::HashMap;

use deck_streak_analytics::rollup::stored;
use deck_streak_analytics::score::raw_streak;
use deck_streak_ingest::reader::Card;
use deck_streak_ingest::tier::Tier;
use deck_streak_kernel::{KernelError, PortFuture, StudyDay, Track, UtcMillis};
use deck_streak_progression::bonus::{DayFacts, daily_bonuses};
use deck_streak_progression::buffs::{arm_ascendant, is_ascendant_day};
use deck_streak_progression::consistency::ascendant_arms;
use deck_streak_progression::review_xp::review_xp;
use deck_streak_progression::settle::{
    SettleCause, SettleError, SettleRequest, settle, settled_amount,
};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Evaluation, Phase};
use crate::skip::days::skip_days;

/// The name the fold's report gives this step.
pub const XP_STEP: &str = "progression.base_xp";

/// The daily bonuses' sources, in the order they are settled.
const BONUS_SOURCES: [&str; 5] = [
    "studied",
    "backlog_zero",
    "streak",
    "score90",
    "graduations",
];

/// Progression's base-XP step.
#[derive(Clone, Copy, Debug, Default)]
pub struct XpStep;

/// Settles `source` on `track` for `day`, as the fold's recompute. A row that does not exist is
/// not written for an amount of nothing.
pub(super) async fn settle_source(
    write: &mut SqliteConnection,
    day: StudyDay,
    source: &'static str,
    track: Track,
    amount: u32,
    closed: bool,
    at: UtcMillis,
) -> Result<(), KernelError> {
    if amount == 0 && settled_amount(write, day, source, track).await?.is_none() {
        return Ok(());
    }
    let request = SettleRequest {
        study_day: day,
        source,
        track,
        amount,
        closed,
    };
    match settle(write, &request, SettleCause::Recompute, at).await {
        Ok(_) => Ok(()),
        Err(SettleError::Database(error)) => Err(KernelError::Database(error)),
        Err(SettleError::NotDerived) => Err(KernelError::Database(sqlx::Error::Protocol(
            "a step settled a source outside the derived registry".to_owned(),
        ))),
    }
}

/// Each card's track and tier, by id. The tier reaches only a law-track card's rate (R4).
fn cards_by_id(cards: &[Card]) -> HashMap<i64, (Track, Option<Tier>)> {
    cards
        .iter()
        .map(|card| {
            let tier = (card.track == Track::Law).then_some(card.tier).flatten();
            (card.id, (card.track, tier))
        })
        .collect()
}

impl DayStep for XpStep {
    fn phase(&self) -> Phase {
        Phase::BaseXp
    }

    fn name(&self) -> &'static str {
        XP_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let facts = day.facts;
            let closed = !matches!(day.evaluation, Evaluation::Current);
            let cards = cards_by_id(&facts.data.cards);
            let (mut language, mut law) = (0_u32, 0_u32);
            for review in facts.reviews_of(day.day) {
                let (track, tier) = cards
                    .get(&review.card_id)
                    .copied()
                    .unwrap_or((Track::Language, None));
                let earned = review_xp(review, tier);
                if track == Track::Law {
                    law = law.saturating_add(earned);
                } else {
                    language = language.saturating_add(earned);
                }
            }
            settle_source(
                write,
                day.day,
                "reviews",
                Track::Language,
                language,
                closed,
                facts.now,
            )
            .await?;
            settle_source(
                write,
                day.day,
                "reviews_law",
                Track::Law,
                law,
                closed,
                facts.now,
            )
            .await?;
            if day.evaluation.runs_today_only_rules() {
                settle_bonuses(day, write, closed).await?;
                arm(day, write).await?;
            }
            Ok(())
        })
    }
}

/// Settles the day's daily bonuses (R11, R12).
async fn settle_bonuses(
    day: &DayEvaluation<'_>,
    write: &mut SqliteConnection,
    closed: bool,
) -> Result<(), KernelError> {
    let facts = day.facts;
    let Some(rolled) = stored(write, day.day).await? else {
        return Ok(());
    };
    let backlog_zero = rolled
        .card_state
        .is_some_and(|state| state.backlog == 0 && state.due_today == 0);
    // The current day's live score, and a closed day's the score that decided it at its close.
    let score_total = if closed {
        rolled.score_at_close.unwrap_or(0)
    } else {
        rolled.score.total
    };
    let earned = daily_bonuses(&DayFacts {
        studied: !facts.reviews_of(day.day).is_empty(),
        backlog_zero,
        streak_days: raw_streak(&facts.study_days(), day.day),
        score_total,
        graduations: rolled.metrics.graduations,
    });
    for source in BONUS_SOURCES {
        let amount = earned
            .iter()
            .find(|(name, _)| *name == source)
            .map_or(0, |(_, amount)| *amount);
        settle_source(
            write,
            day.day,
            source,
            Track::Language,
            amount,
            closed,
            facts.now,
        )
        .await?;
    }
    Ok(())
}

/// Arms the Ascendant buff for the day when the day before earned it (R20).
async fn arm(day: &DayEvaluation<'_>, write: &mut SqliteConnection) -> Result<(), KernelError> {
    let before = StudyDay::from_epoch_day(day.day.epoch_day() - 1);
    let reviews = stored(write, before)
        .await?
        .map(|rolled| rolled.metrics.reviews);
    let backlog_zero = settled_amount(write, before, "backlog_zero", Track::Language)
        .await?
        .unwrap_or(0);
    let held = is_ascendant_day(write, day.day).await?;
    // A skip day is armed by nothing: the day before earned no buff for a day that was not studied.
    let skip = skip_days(write).await?.contains(&day.day);
    if ascendant_arms(held, skip, reviews, i64::from(backlog_zero)) {
        arm_ascendant(write, day.day, day.facts.now).await?;
    }
    Ok(())
}
