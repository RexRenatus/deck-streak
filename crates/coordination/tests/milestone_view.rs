//! The next-milestone view waits for Road to C2 (SPEC-073 A19; R14, R15): with no mature-card sum
//! supplied it answers `pending` and computes nothing from a stand-in, and once the sum is supplied
//! it answers progression's milestone over the three inputs.

use deck_streak_coordination::progression::milestone_view::{MilestoneView, milestone_view};
use deck_streak_progression::milestone::next_milestone;

#[test]
fn the_milestone_is_pending_until_road_to_c2_supplies_the_mature_cards() {
    let totals = [(0, 0), (99, 6), (1_000, 30), (60_000, 2_000)];
    for (reviews, streak) in totals {
        assert_eq!(
            milestone_view(reviews, streak, None),
            MilestoneView::Pending,
            "{reviews} reviews and a {streak}-day streak with no mature cards supplied"
        );
        for mature in [0, 100, 6_000] {
            assert_eq!(
                milestone_view(reviews, streak, Some(mature)),
                MilestoneView::Next(next_milestone(reviews, streak, mature)),
                "{reviews} reviews, a {streak}-day streak and {mature} mature cards"
            );
        }
    }
}
