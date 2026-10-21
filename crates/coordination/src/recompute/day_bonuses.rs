//! Phase 5's step (SPEC-072 R15 to R17, R19, R21): a day's consistency and Ascendant bonuses,
//! settled after every base source of the day, so a later recompute that raises a base source of a
//! closed day settles them again under the raise-only rule.

use std::collections::BTreeSet;

use deck_streak_analytics::rollup::stored;
use deck_streak_kernel::{KernelError, PortFuture, StudyDay, Track};
use deck_streak_progression::buffs::is_ascendant_day;
use deck_streak_progression::consistency::{day_base_xp, day_bonuses, on_pace_run};
use deck_streak_progression::economy_config::xp;
use deck_streak_progression::settle::{day_rows, settled_amount};
use sqlx::SqliteConnection;

use super::xp::settle_source;
use super::{DayEvaluation, DayStep, Evaluation, Phase};
use crate::skip::days::skip_epoch_days;

/// The name the fold's report gives this step.
pub const DAY_BONUSES_STEP: &str = "progression.derived_bonuses";

/// Progression's derived-bonuses step.
#[derive(Clone, Copy, Debug, Default)]
pub struct DayBonusesStep;

impl DayStep for DayBonusesStep {
    fn phase(&self) -> Phase {
        Phase::DerivedBonuses
    }

    fn name(&self) -> &'static str {
        DAY_BONUSES_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let facts = day.facts;
            if facts.reviews_of(day.day).is_empty() {
                return Ok(());
            }
            let closed = !matches!(day.evaluation, Evaluation::Current);
            let rows = day_rows(write, day.day).await?;
            let base = day_base_xp(
                rows.iter()
                    .map(|(source, amount)| (source.as_str(), *amount)),
            );
            let run = on_pace_run(
                &recent_scores(write, facts.study_days(), day.day).await?,
                &skip_epoch_days(write).await?,
                day.day.epoch_day(),
            );
            let buff = is_ascendant_day(write, day.day).await?;
            let language = settled_amount(write, day.day, "reviews", Track::Language)
                .await?
                .unwrap_or(0);
            let law = settled_amount(write, day.day, "reviews_law", Track::Law)
                .await?
                .unwrap_or(0);
            let earned = day_bonuses(base, run, buff, i64::from(language), i64::from(law));
            settle_source(
                write,
                day.day,
                "consistency",
                Track::Language,
                earned.consistency,
                closed,
                facts.now,
            )
            .await?;
            if let Some(amount) = earned.ascendant {
                settle_source(
                    write,
                    day.day,
                    "ascendant",
                    Track::Language,
                    amount,
                    closed,
                    facts.now,
                )
                .await?;
            }
            Ok(())
        })
    }
}

/// The scores of the newest rollups on or before `day`, as many as the run's window reads, back to
/// the first study day the fold's window holds.
async fn recent_scores(
    write: &mut SqliteConnection,
    study_days: BTreeSet<StudyDay>,
    day: StudyDay,
) -> Result<Vec<(i64, i64)>, KernelError> {
    let window = xp().window_days;
    let first = study_days
        .first()
        .map_or(day.epoch_day(), |first| first.epoch_day());
    let mut scores = Vec::new();
    let mut number = day.epoch_day();
    while scores.len() != window && number >= first {
        if let Some(rolled) = stored(write, StudyDay::from_epoch_day(number)).await? {
            scores.push((number, rolled.score.total));
        }
        number -= 1;
    }
    Ok(scores)
}
