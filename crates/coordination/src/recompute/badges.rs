//! The badge step (SPEC-073 R4, R5, R8; ADR-303): phase 7 of the fold awards each study badge the
//! evaluated day earns, and the offers raise each award's celebration until the router answers.
//!
//! A closing day at its settle and the current day are evaluated (ADR-071): the day's badge context
//! is built from its rollup (the score it closed with, or the live score, and its recorded card
//! state), the language streak, the window's reviews and the lifetime, and every met study
//! condition is awarded through progression's award port in the day's own write, its mark unset.
//! An award is never celebrated from inside that write: the router opens its own, so the fold's
//! offers hand every unmarked badge to it between the writes, and set the mark in a write of its
//! own, only once the router has answered and only while the row is still unmarked.

use deck_streak_analytics::rollup::{self, StoredDay, recent_totals};
use deck_streak_kernel::{Courses, Db, KernelError, PortFuture, StudyDay, UtcMillis};
use deck_streak_progression::badges::award::{Award, AwardError, NewBadge, award, unmarked};
use deck_streak_progression::badges::catalog::catalog;
use deck_streak_progression::badges::conditions::{PERFECT_WEEK_DAYS, Snapshot, conditions};
use deck_streak_streaks::store;
use deck_streak_streaks::streak::comeback_view;
use sqlx::SqliteConnection;

use super::{Celebrate, Celebration, DayEvaluation, DayStep, Evaluation, Phase};
use crate::progression::badge_context::{DayState, badge_context};

/// The badge step's name, as the fold's report and log name it.
pub const BADGES_STEP: &str = "progression.badges";

/// The ladder's event a badge celebration names.
pub const BADGE_EVENT: &str = "badge";

/// The study badges whose condition reads the card snapshot: a day with no recorded card state
/// judges none of them, rather than judging them on a snapshot it never had.
pub const SNAPSHOT_KEYS: [&str; 5] = [
    "inbox_zero",
    "backlog_slayer",
    "maturity_milestone",
    "forest_guardian",
    "leech_tamer",
];

/// The dedupe key of a badge's celebration: `badge:<key>:<tier>` (R4).
#[must_use]
pub fn badge_key(key: &str, tier: u32) -> String {
    format!("badge:{key}:{tier}")
}

/// The line a badge's celebration carries.
#[must_use]
pub fn badge_line(emoji: &str, name: &str) -> String {
    format!("{emoji} Badge earned: {name}")
}

/// Phase 7's badge step, over the owner's courses.
#[derive(Debug)]
pub struct BadgesStep {
    courses: Courses,
}

impl BadgesStep {
    /// The step, awarding against `courses`.
    #[must_use]
    pub const fn new(courses: Courses) -> Self {
        Self { courses }
    }
}

impl DayStep for BadgesStep {
    fn phase(&self) -> Phase {
        Phase::Awards
    }

    fn name(&self) -> &'static str {
        BADGES_STEP
    }

    fn evaluate<'a>(
        &'a self,
        day: &'a DayEvaluation<'a>,
        write: &'a mut SqliteConnection,
    ) -> PortFuture<'a, ()> {
        Box::pin(async move {
            if !day.evaluation.runs_today_only_rules() {
                return Ok(());
            }
            let earned = earned(day, write).await?;
            let catalog = catalog(&self.courses);
            for key in earned {
                let Some(badge) = catalog.iter().find(|badge| badge.key == key) else {
                    return Err(refused(format!("the catalog has no badge {key}")));
                };
                let new = NewBadge {
                    key,
                    tier: badge.tier,
                    name: &badge.name,
                    emoji: &badge.emoji,
                    study_day: day.day,
                    at: day.facts.now,
                };
                match award(write, &self.courses, &new).await {
                    Ok(Award::Awarded | Award::AlreadyAwarded) => {}
                    Err(AwardError::Database(error)) => return Err(error),
                    Err(AwardError::UnknownKey) => {
                        return Err(refused(format!("the award port refused the key {key}")));
                    }
                }
            }
            Ok(())
        })
    }
}

/// The score `day` is judged with: the score it closed with at its settle (its end-of-day state),
/// the live score while it is current (SPEC-073 R5, A11).
#[must_use]
pub fn judged_score(stored: &StoredDay, evaluation: Evaluation) -> i64 {
    match evaluation {
        Evaluation::Settle { .. } => stored.score_at_close.unwrap_or(stored.score.total),
        _ => stored.score.total,
    }
}

/// The study badges `day` earns, in the catalog's condition order, with every badge that reads
/// the card snapshot left out on a day that recorded none.
async fn earned(
    day: &DayEvaluation<'_>,
    write: &mut SqliteConnection,
) -> Result<Vec<&'static str>, KernelError> {
    let stored = rollup::stored(write, day.day).await?;
    let score_total = stored
        .as_ref()
        .map_or(0, |stored| judged_score(stored, day.evaluation));
    let card_state = stored.as_ref().and_then(|stored| stored.card_state);
    let snapshot = card_state.map_or_else(Snapshot::default, |state| Snapshot {
        mature_count: state.mature_count,
        leech_active: state.leech_active,
        backlog: state.backlog,
        due_today: state.due_today,
    });
    let streak = store::state(write, "language").await?;
    let week = i64::try_from(PERFECT_WEEK_DAYS).unwrap_or(i64::MAX);
    let rollups: Vec<(StudyDay, i64)> = recent_totals(write, day.day, week)
        .await?
        .into_iter()
        .map(|totals| (totals.day, totals.score))
        .collect();
    let facts = day.facts;
    let context = badge_context(&DayState {
        reviews: &facts.data.reviews,
        rule: facts.rule,
        day: day.day,
        card_decks: facts.card_decks(),
        snapshot,
        streak_current: streak.as_ref().map_or(0, |state| u64::from(state.current)),
        comeback_armed: streak.as_ref().is_some_and(comeback_view),
        lifetime: facts.lifetime_through(day.day),
        score_total,
        rollups: &rollups,
    });
    Ok(conditions(&context)
        .into_iter()
        .filter(|(key, met)| *met && (card_state.is_some() || !SNAPSHOT_KEYS.contains(key)))
        .map(|(key, _)| key)
        .collect())
}

/// Offers every badge whose mark is unset to `celebrate`, oldest first, and marks each one it
/// answered at `now`, in a write of its own (ADR-303). An offer the router did not answer leaves
/// the badge owed for the next offers.
///
/// # Errors
///
/// [`KernelError`] when the owed badges cannot be read, or a mark cannot be written.
pub async fn offer_badges(
    celebrate: &dyn Celebrate,
    db: &Db,
    now: UtcMillis,
    today: StudyDay,
) -> Result<(), KernelError> {
    let owed = {
        let mut read = db.reader().acquire().await?;
        unmarked(&mut read).await?
    };
    for badge in owed {
        let celebration = Celebration {
            event: BADGE_EVENT,
            key: badge_key(&badge.key, badge.tier),
            text: badge_line(&badge.emoji, &badge.name),
            study_day: today,
        };
        // The mark is set only once the router has answered (ADR-303).
        if let Err(error) = celebrate.celebrate(&celebration).await {
            tracing::warn!(key = %celebration.key, %error, "the router did not answer; the badge stays owed");
            continue;
        }
        let at = now.epoch_millis();
        let tier = i64::from(badge.tier);
        let mut write = db.write().await?;
        sqlx::query!(
            "UPDATE badges_earned SET celebrated_at = ?1 \
             WHERE badge_key = ?2 AND tier = ?3 AND celebrated_at IS NULL",
            at,
            badge.key,
            tier
        )
        .execute(&mut *write)
        .await?;
        write.commit().await?;
    }
    Ok(())
}

/// A refusal the step cannot recover from, as the database error kind the cycle already reports.
fn refused(reason: impl std::fmt::Display) -> KernelError {
    KernelError::Database(sqlx::Error::Protocol(format!("{reason}")))
}
