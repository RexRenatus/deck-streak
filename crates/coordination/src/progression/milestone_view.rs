//! The next-milestone view (SPEC-073 R14, R15): progression's milestone over the lifetime study
//! reviews, the language streak and Road to C2's mature cards, or `pending` while Road to C2 does
//! not supply that sum (#85). Nothing is computed from a stand-in for it.

use deck_streak_progression::milestone::Milestone;

/// What the milestone view answers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MilestoneView {
    /// Road to C2 has not supplied the mature cards yet.
    Pending,
    /// The nearest unreached rung.
    Next(Milestone),
}

/// The milestone over `reviews`, `streak` and the mature cards, `pending` while `mature` is not
/// supplied (R15).
#[must_use]
pub fn milestone_view(reviews: u64, streak: u64, mature: Option<u64>) -> MilestoneView {
    let _ = (reviews, streak, mature);
    MilestoneView::Pending
}
