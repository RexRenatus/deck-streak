//! Phase 1's step (SPEC-071 R5 to R14, R17, R18): analytics rolls a day up and scores it.
//!
//! For each day the fold hands it, the step rolls up the day's metrics, its per-course rows and its
//! fingerprint when the evaluation re-rolls it, and scores it:
//!
//! - the current day with its live card snapshot, recorded as its card state;
//! - a settling day with its end-of-day card snapshot when it is the most recently closed day, taken
//!   at its own collection day number and recorded as its card state, over the rollups before it;
//!   that total is kept as the score the day closed with;
//! - every other day in the predecessor's historical form: no card state, and the current day's
//!   baseline.
//!
//! Every rule is analytics' own (`deck_streak_analytics`); the step only chooses which, for the day
//! the fold evaluates.

use std::collections::BTreeSet;

use deck_streak_analytics::metrics::{DailyMetrics, daily_metrics, language_metrics};
use deck_streak_analytics::rollup::{
    RolledDay, fingerprint, recent_volumes, record_card_state, record_close, record_score, roll_up,
    stored,
};
use deck_streak_analytics::score::{Score, ScoreState, baseline_window, compute_score, raw_streak};
use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_analytics::snapshot::{CardSnapshot, CardState, card_snapshot};
use deck_streak_ingest::calendar::collection_day_number;
use deck_streak_kernel::{KernelError, PortFuture, StudyDay};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Evaluation, Phase, RecomputeFacts};

/// The name the fold's report gives this step.
pub const ANALYTICS_STEP: &str = "analytics.rollup_and_score";

/// Analytics' step: the rollup and the score.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnalyticsStep {
    settings: AnalyticsSettings,
}

impl AnalyticsStep {
    /// The step, counting leeches by `settings`.
    #[must_use]
    pub const fn new(settings: AnalyticsSettings) -> Self {
        Self { settings }
    }
}

impl DayStep for AnalyticsStep {
    fn phase(&self) -> Phase {
        Phase::RollupAndScore
    }

    fn name(&self) -> &'static str {
        ANALYTICS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let facts = day.facts;
            let streak = raw_streak(&facts.study_days(), day.day);
            match day.evaluation {
                Evaluation::Current => {
                    let metrics = self.metrics(facts, day.day);
                    let snapshot = self.snapshot(facts, day.day);
                    let baseline = baseline_window(&recent_volumes(write, day.day).await?, day.day);
                    let state = Some(ScoreState::from(&snapshot));
                    let score = compute_score(&metrics, state, streak, baseline);
                    roll(write, facts, &metrics, &score).await?;
                    record_card_state(write, day.day, &CardState::from(&snapshot), facts.now)
                        .await?;
                }
                Evaluation::Settle { end_of_day } => {
                    // Scored as the current day it was: over the rollups before it, and with its
                    // end-of-day card state when it is the day that just closed.
                    let metrics = self.metrics(facts, day.day);
                    let snapshot = end_of_day.then(|| self.snapshot(facts, day.day));
                    let baseline = baseline_window(&recent_volumes(write, day.day).await?, day.day);
                    let state = snapshot.as_ref().map(ScoreState::from);
                    let score = compute_score(&metrics, state, streak, baseline);
                    roll(write, facts, &metrics, &score).await?;
                    if let Some(snapshot) = snapshot {
                        record_card_state(write, day.day, &CardState::from(&snapshot), facts.now)
                            .await?;
                    }
                    record_close(write, day.day, score.total).await?;
                }
                Evaluation::Backfill { .. } | Evaluation::Revisit { .. } => {
                    // The predecessor's historical form: no card state, the current day's baseline.
                    let baseline =
                        baseline_window(&recent_volumes(write, facts.today).await?, facts.today);
                    let kept = if day.evaluation.rerolls() {
                        None
                    } else {
                        stored(write, day.day).await?.map(|row| row.metrics)
                    };
                    match kept {
                        Some(metrics) => {
                            let score = compute_score(&metrics, None, streak, baseline);
                            record_score(write, day.day, &score).await?;
                        }
                        None => {
                            let metrics = self.metrics(facts, day.day);
                            let score = compute_score(&metrics, None, streak, baseline);
                            roll(write, facts, &metrics, &score).await?;
                        }
                    }
                }
            }
            Ok(())
        })
    }
}

impl AnalyticsStep {
    /// The metrics of `day` over the window's reviews of that day.
    fn metrics(self, facts: &RecomputeFacts<'_>, day: StudyDay) -> DailyMetrics {
        let _ = self;
        daily_metrics(facts.reviews_of(day), facts.rule, day, facts.card_decks())
    }

    /// The card snapshot at `day`'s own collection day number (R7, R8).
    fn snapshot(self, facts: &RecomputeFacts<'_>, day: StudyDay) -> CardSnapshot {
        let number = collection_day_number(facts.rule, facts.data.created_at, day);
        card_snapshot(
            &facts.data.cards,
            self.settings.leech_threshold.get(),
            number,
        )
    }
}

/// Rolls `metrics`'s day up with `score`: its per-course rows and its fingerprint with it (R11,
/// R18).
async fn roll(
    write: &mut SqliteConnection,
    facts: &RecomputeFacts<'_>,
    metrics: &DailyMetrics,
    score: &Score,
) -> Result<bool, KernelError> {
    let reviews = facts.reviews_of(metrics.day);
    let languages = language_metrics(
        reviews,
        facts.rule,
        facts.card_courses(),
        &BTreeSet::from([metrics.day]),
    );
    let print = fingerprint(reviews, facts.courses_digest);
    roll_up(
        write,
        &RolledDay {
            metrics,
            languages: &languages,
            fingerprint: &print,
            score,
        },
        facts.now,
    )
    .await
}
