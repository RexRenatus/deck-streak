//! The law drill use cases (SPEC-110 R7, R9): the one answer both surfaces call, and the hourly
//! post-back that records each graded drill and pays it once.

use deck_streak_kernel::{Db, StudyDay, StudyDayRule, Track, UtcMillis};
use deck_streak_progression::grant::{
    GrantAnswer, GrantPort, GrantRequest, GrantScope, GrantSource,
};
use deck_streak_progression::xp::XpAmount;
pub use deck_streak_vault::drill_notes::{AnswerOutcome, DrillNotes};
pub use deck_streak_vault::drill_store::Surface;
use deck_streak_vault::drill_store::{GradeRow, record_grade};
use deck_streak_vault::drills::drill_key;
pub use deck_streak_vault::drills::{DrillMeta, DrillView};
pub use deck_streak_vault::{RealFs, VaultError, VaultFs};

use crate::runner::{Done, Fire, Reason, Work};

/// Answers the Active drill `id` with `text` from `surface` at `at` (R7): the one use case the
/// bot's `/drills` answer and `POST /api/drills/{id}/answer` both call.
///
/// # Errors
///
/// As [`DrillNotes::answer`].
pub async fn answer<F: VaultFs>(
    notes: &DrillNotes<F>,
    db: &Db,
    id: &str,
    text: &str,
    surface: Surface,
    rule: StudyDayRule,
    at: UtcMillis,
) -> Result<AnswerOutcome, VaultError> {
    let day = rule.study_day(at);
    notes
        .answer(db, id, text, surface, day, &stamp(rule, at), at)
        .await
}

/// The local `YYYY-MM-DD HH:MM` the answer heading names: `at` at the rule's offset, as the
/// predecessor's local clock reads it.
fn stamp(rule: StudyDayRule, at: UtcMillis) -> String {
    let minutes = at.epoch_millis().div_euclid(60_000) + i64::from(rule.utc_offset().minutes());
    let date = StudyDay::from_epoch_day(minutes.div_euclid(1_440));
    let of_day = minutes.rem_euclid(1_440);
    format!("{date} {:02}:{:02}", of_day / 60, of_day % 60)
}

/// The `drill_postback` job's work (R9).
pub struct DrillPostbackWork<'a, F: VaultFs, G> {
    notes: Result<DrillNotes<F>, VaultError>,
    db: &'a Db,
    grants: &'a G,
}

impl<'a, F: VaultFs, G: GrantPort> DrillPostbackWork<'a, F, G> {
    /// The work over `notes` (the open's result: a missing root is the job's error), recording
    /// grades in `db` and paying through `grants`.
    #[must_use]
    pub const fn new(notes: Result<DrillNotes<F>, VaultError>, db: &'a Db, grants: &'a G) -> Self {
        Self { notes, db, grants }
    }
}

impl<F: VaultFs + Send + Sync, G: GrantPort> Work for DrillPostbackWork<'_, F, G> {
    async fn perform(&self, fire: &Fire) -> Result<Done, Reason> {
        let notes = self.notes.as_ref().map_err(|error| {
            tracing::error!(%error, "the drill post-back cannot open the vault");
            Reason::new("drill_vault_root_unreadable")
        })?;
        let graded = notes.graded().map_err(|error| {
            tracing::error!(%error, "the drill post-back cannot list the graded drills");
            Reason::new("drill_graded_unreadable")
        })?;
        let mut paid: i64 = 0;
        for drill in &graded {
            let row = GradeRow {
                drill_id: drill.drill_id.clone(),
                drill_type: drill.kind.clone(),
                subject: drill.subject.clone(),
                xp: drill.xp,
                study_day: fire.study_day,
            };
            self.record(&row, fire).await?;
            let request = GrantRequest {
                study_day: fire.study_day,
                source: GrantSource::new(&drill_key(&drill.drill_id)).map_err(|refusal| {
                    tracing::error!(%refusal, "a drill's key is refused");
                    Reason::new("drill_key_refused")
                })?,
                track: Track::Law,
                amount: XpAmount::new(u32::try_from(drill.xp).unwrap_or_default()),
                scope: GrantScope::Once,
            };
            let answer = self
                .grants
                .grant(&request, fire.started_at)
                .await
                .map_err(|error| {
                    tracing::error!(%error, "a drill's grant failed");
                    Reason::new("drill_grant_failed")
                })?;
            if matches!(answer, GrantAnswer::Granted(_)) {
                paid += 1;
            }
        }
        tracing::info!(graded = graded.len(), paid, "the drill post-back polled");
        Ok(Done::Done)
    }
}

impl<F: VaultFs, G> DrillPostbackWork<'_, F, G> {
    /// Records `row` in its own write: idempotent, so a re-poll changes nothing.
    async fn record(&self, row: &GradeRow, fire: &Fire) -> Result<(), Reason> {
        let failed = |error: &dyn std::fmt::Display| {
            tracing::error!(%error, "a drill's grade could not be recorded");
            Reason::new("drill_grade_unwritable")
        };
        let mut write = self.db.write().await.map_err(|error| failed(&error))?;
        record_grade(&mut write, row, fire.started_at)
            .await
            .map_err(|error| failed(&error))?;
        write.commit().await.map_err(|error| failed(&error))
    }
}
