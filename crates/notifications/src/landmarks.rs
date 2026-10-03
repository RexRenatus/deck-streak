//! The historical landmarks of the owner's review log (SPEC-102 R1 to R4, ADR-318): the
//! anniversaries of the first study day and every 25th earned study day, the rule that selects
//! the ones dated today, and their texts. It ports the predecessor's `landmarks.py`
//! (`compute_landmarks`, `due_today`, `render_landmark`, `_ordinal_label`, at `27ee2bc`).
//!
//! Everything here is a pure function of integers. The study days arrive already read, by
//! ingest's own read of the whole scoped log, because notifications has no edge to ingest.

use deck_streak_kernel::StudyDay;

/// Every this many earned study days is a landmark.
pub const LANDMARK_DAY_STEP: usize = 0;
/// The event name of an anniversary landmark.
pub const ANNIVERSARY_EVENT_TYPE: &str = "";
/// The event name of an earned-study-day landmark.
pub const STUDY_DAY_EVENT_TYPE: &str = "";
/// The anniversary's text.
pub const ANNIVERSARY_TEMPLATE: &str = "";
/// The anniversary's text when the streak is not current.
pub const ANNIVERSARY_GAP_TEMPLATE: &str = "";
/// The earned-study-day landmark's text.
pub const STUDY_DAY_TEMPLATE: &str = "";

/// One historical event; it carries no text derived from the collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Landmark {
    /// The dedupe key, `landmark:anniv:N` or `landmark:day:N`.
    pub key: String,
    /// The event name, [`ANNIVERSARY_EVENT_TYPE`] or [`STUDY_DAY_EVENT_TYPE`].
    pub event: &'static str,
    /// The anniversary's or the study day's ordinal.
    pub ordinal: u32,
    /// The study day the event is dated.
    pub day: StudyDay,
}

/// The landmarks that have landed by `today`, oldest first.
#[must_use]
pub fn compute_landmarks(study_days: &[StudyDay], today: StudyDay) -> Vec<Landmark> {
    let _ = (study_days, today);
    Vec::new()
}

/// The landmarks dated exactly `today`.
#[must_use]
pub fn due_today(landmarks: &[Landmark], today: StudyDay) -> Vec<&Landmark> {
    let _ = (landmarks, today);
    Vec::new()
}

/// The text of `landmark`; `gap_honest` drops the anniversary's congratulation of a streak.
#[must_use]
pub fn render_landmark(landmark: &Landmark, gap_honest: bool) -> String {
    let _ = (landmark, gap_honest);
    String::new()
}
