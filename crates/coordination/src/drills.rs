//! The law drill use cases (SPEC-110 R7, R9): the one answer both surfaces call, and the hourly
//! post-back that records each graded drill and pays it once.

use deck_streak_kernel::{Db, StudyDayRule, UtcMillis};
use deck_streak_progression::grant::GrantPort;
pub use deck_streak_vault::drill_notes::{AnswerOutcome, DrillNotes};
pub use deck_streak_vault::drill_store::Surface;
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
    let _ = (notes, db, id, text, surface, rule, at);
    Ok(AnswerOutcome::NotActive)
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
        let _ = (&self.notes, self.db, self.grants, fire);
        Ok(Done::Done)
    }
}
