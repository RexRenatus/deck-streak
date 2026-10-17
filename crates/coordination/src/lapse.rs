//! The open lapse, read from the window a caller read (SPEC-049 R15; ADR-088).
//!
//! It counts the qualifying reviews of each study day the window holds (ingest's
//! [`is_study_event`], on the days SPEC-020's study-day rule gave [`RecomputeFacts`]), takes the
//! skip set from its caller, who reads it where it reads the record (ADR-321 D12), and returns the open lapse's id as the
//! kernel's [`StudyDay`], which an occasion's lapse context carries as it is (SPEC-041 R3). It
//! opens no collection: the window is its caller's, and it holds no rule of its own, the rule is
//! streaks' [`open_lapse`].
//!
//! It lies outside `readings/`, so SPEC-047's census (its A10) stays green. Its callers are
//! SPEC-076's streak step and SPEC-049's remainder.

use std::collections::{BTreeMap, BTreeSet};

use deck_streak_ingest::reader::is_study_event;
use deck_streak_kernel::StudyDay;
use deck_streak_streaks::lapse::{self, LAPSE_AFTER_SILENT_DAYS};

use crate::recompute::RecomputeFacts;

/// The lapse open on the facts' current study day, by its id, if any.
///
/// Every day the window holds a review on has a row, with a count of zero when none of that day's
/// rows is a study event, so a manual or rescheduling entry never closes a run.
#[must_use]
pub fn open_lapse(facts: &RecomputeFacts<'_>, skips: &BTreeSet<StudyDay>) -> Option<StudyDay> {
    let review_counts: BTreeMap<StudyDay, u32> = facts
        .data
        .reviews
        .iter()
        .map(|review| {
            (
                facts
                    .rule
                    .study_day(deck_streak_kernel::UtcMillis::from_epoch_millis(review.id)),
                review,
            )
        })
        .fold(BTreeMap::new(), |mut rows, (day, review)| {
            let studied = u32::from(is_study_event(review.kind, review.ease));
            let count: &mut u32 = rows.entry(day).or_insert(0);
            *count = count.saturating_add(studied);
            rows
        });
    lapse::open_lapse(facts.today, &review_counts, skips, LAPSE_AFTER_SILENT_DAYS)
}
