//! Phase 3's step (SPEC-076 R17 to R20, R26): both tracks' streaks, the freeze ledger, the habit
//! strength and the governor, evaluated for every study day the fold visits, and the relight's XP
//! granted on the fold's own connection so that the day's base carries it in the same recompute.
//!
//! Every rule is streaks' or progression's; the step only chooses which, for the day the fold
//! evaluates, and stores what they answer.

use std::collections::BTreeSet;

use deck_streak_ingest::reader::is_study_event;
use deck_streak_kernel::{Db, KernelError, PortFuture, StudyDay, Track, UtcMillis};
use deck_streak_progression::grant::{GrantRequest, GrantScope, GrantSource};
use deck_streak_progression::ledger::grant_on;
use deck_streak_progression::xp::XpAmount;
use deck_streak_streaks::governor::{assess, silence_walk};
use deck_streak_streaks::lapse::anchor_beyond_the_walk;
use deck_streak_streaks::relight::relight;
use deck_streak_streaks::streak::StreakState;
use deck_streak_streaks::{replay, store, strength};
use sqlx::SqliteConnection;

use super::{DayEvaluation, DayStep, Evaluation, Phase};
use crate::skip::days::skip_days;

/// The name the fold's report gives this step.
pub const STREAKS_STEP: &str = "streaks.streaks_and_governor";

/// The relight's due list (SPEC-076 R27): the study days whose relight grant committed and whose
/// celebration the router has not yet decided. The step writes a day in the same write as its grant,
/// so a day is due exactly when that write commits, and the list outlives a restart between the
/// commit and the route.
#[derive(Clone, Debug, Default)]
pub struct RelightDue;

impl RelightDue {
    /// Every day still due, oldest first.
    ///
    /// # Errors
    ///
    /// [`KernelError`] when the due days cannot be read.
    pub async fn pending(&self, db: &Db) -> Result<Vec<StudyDay>, KernelError> {
        let mut read = db.reader().acquire().await?;
        store::relight_due(&mut read).await
    }

    /// Marks `day` routed: the router has decided its celebration, so it is no longer due.
    ///
    /// # Errors
    ///
    /// [`KernelError`] when the mark cannot be written.
    pub async fn routed(&self, db: &Db, day: StudyDay) -> Result<(), KernelError> {
        let mut write = db.write().await?;
        store::clear_relight_due(&mut write, day).await?;
        write.commit().await?;
        Ok(())
    }
}

/// The streaks and governor step.
#[derive(Clone, Debug, Default)]
pub struct StreaksStep;

impl StreaksStep {
    /// A step, and the handle its caller routes the relights from.
    #[must_use]
    pub fn new() -> (Self, RelightDue) {
        (Self, RelightDue)
    }
}

/// The study days of each track, and of both, as the facts' window holds them.
struct StudyDays {
    language: BTreeSet<StudyDay>,
    law: BTreeSet<StudyDay>,
    any: BTreeSet<StudyDay>,
}

impl StudyDays {
    fn of(facts: &super::RecomputeFacts<'_>) -> Self {
        let mut days = Self {
            language: BTreeSet::new(),
            law: BTreeSet::new(),
            any: BTreeSet::new(),
        };
        let tracks: std::collections::HashMap<i64, Track> = facts
            .data
            .cards
            .iter()
            .map(|card| (card.id, card.track))
            .collect();
        for review in &facts.data.reviews {
            if !is_study_event(review.kind, review.ease) {
                continue;
            }
            let day = facts
                .rule
                .study_day(UtcMillis::from_epoch_millis(review.id));
            days.any.insert(day);
            match tracks.get(&review.card_id) {
                Some(Track::Law) => days.law.insert(day),
                _ => days.language.insert(day),
            };
        }
        days
    }
}

impl StreaksStep {
    async fn streaks(
        day: StudyDay,
        evaluation: Evaluation,
        days: &StudyDays,
        facts: &super::RecomputeFacts<'_>,
        write: &mut SqliteConnection,
    ) -> Result<(), KernelError> {
        let skips = skip_days(write).await?;
        let (language, events) = replay::language(&days.language, &skips, day);
        store::insert_events(write, &events, facts.now).await?;
        if day == facts.today {
            let law = replay::law(&days.law, &skips, day);
            let outside = store::external_freezes(write).await?;
            let held = i64::from(language.freezes)
                .saturating_add(outside)
                .clamp(0, 3);
            let language = StreakState {
                freezes: u32::try_from(held).unwrap_or(0),
                ..language
            };
            store::upsert_state(write, "language", &language, facts.now).await?;
            store::upsert_state(write, "law", &law, facts.now).await?;
        }
        let first_run = matches!(evaluation, Evaluation::Backfill { .. });
        if strength::persists(day, facts.today, first_run) {
            let held = strength::fold(&days.any, day);
            if let Some((_, value)) = held.last() {
                store::put_strength(write, day, *value, facts.now).await?;
            }
        }
        Ok(())
    }

    async fn relight(
        day: StudyDay,
        facts: &super::RecomputeFacts<'_>,
        lapse_open: bool,
        write: &mut SqliteConnection,
    ) -> Result<(), KernelError> {
        if !lapse_open {
            return Ok(());
        }
        let reviews = facts
            .reviews_of(day)
            .iter()
            .filter(|review| is_study_event(review.kind, review.ease))
            .count();
        let counted = u32::try_from(reviews).unwrap_or(u32::MAX);
        // The grant port answers a replay as already granted, so the rule is asked with nothing
        // granted and the caller routes the celebration on every qualifying recompute (R19, R27).
        let Some(due) = relight(day, Some(counted), 0) else {
            return Ok(());
        };
        let source = GrantSource::new(&due.source)
            .map_err(|refused| KernelError::Database(sqlx::Error::Protocol(refused.to_string())))?;
        let request = GrantRequest {
            study_day: day,
            source,
            track: Track::Language,
            amount: XpAmount::new(due.amount),
            scope: GrantScope::Once,
        };
        let _ = grant_on(write, &request, facts.now).await?;
        // The day is due in the grant's own write: a write that rolls back leaves no day due, and
        // one that commits leaves it due until the router decides it (R27).
        store::put_relight_due(write, day, facts.now).await
    }

    async fn govern(
        day: StudyDay,
        days: &StudyDays,
        stored: store::GovernorRow,
        facts: &super::RecomputeFacts<'_>,
        write: &mut SqliteConnection,
    ) -> Result<(), KernelError> {
        let skips = skip_days(write).await?;
        let anchor = anchor_beyond_the_walk(day, &days.any, &skips, stored.lapse_since);
        let value = strength::fold(&days.any, day)
            .last()
            .map_or(0.0, |(_, value)| *value);
        let verdict = assess(value, silence_walk(day, &days.any, &skips).silent_days);
        store::put_governor(write, anchor, verdict.standby, facts.now).await
    }
}

impl DayStep for StreaksStep {
    fn phase(&self) -> Phase {
        Phase::StreaksAndGovernor
    }

    fn name(&self) -> &'static str {
        STREAKS_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            let facts = day.facts;
            let days = StudyDays::of(facts);
            let stored = store::governor(write).await?;
            Self::streaks(day.day, day.evaluation, &days, facts, write).await?;
            if day.evaluation.runs_today_only_rules() {
                Self::relight(day.day, facts, stored.lapse_since.is_some(), write).await?;
            }
            if matches!(day.evaluation, Evaluation::Settle { .. }) {
                Self::govern(day.day, &days, stored, facts, write).await?;
            }
            Ok(())
        })
    }
}
