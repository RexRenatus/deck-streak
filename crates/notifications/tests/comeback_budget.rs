//! The comeback's cap and the lapse (SPEC-041 A9, A10; R4 rules 1, 3 and 5): at most 3 comebacks for
//! one lapse id, at least 3 study days apart, then silence for the episode; a nudge in an open lapse
//! is withheld while a comeback in it is sent; a comeback is raised only inside a lapse, and the
//! owner switches it off with "0". Every clock is a `ManualClock`, at noon, outside quiet hours.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

mod support;

use deck_streak_notifications::{
    Decision, DedupeKey, LapseContext, Occasion, OccasionError, Reason, Surface, Tier,
};
use support::{DAY, Harness, at, day};

/// The lapse the tests open: anchored three study days before `DAY`.
const LAPSE: LapseContext = LapseContext::Open(day(DAY - 3));

/// What a sent comeback decides.
const SENT: Decision = Decision::Sent {
    surface: Surface::Bot,
    tier: Tier::T2,
};

/// What a comeback past its cap or inside its gap decides.
const SPENT: Decision = Decision::Withheld {
    surface: Surface::Bot,
    reason: Reason::BudgetSpent,
};

/// Routes a comeback on the study day `on` in `lapse`.
async fn comeback(harness: &Harness, on: i64, lapse: LapseContext) -> Decision {
    harness.clock.set(at(on, 12, 0));
    let occasion = harness.occasion(
        "comeback",
        "comeback:reading",
        Surface::Bot,
        Tier::T2,
        on,
        lapse,
    );
    harness.router.route(&occasion).await.expect("a decision")
}

#[tokio::test]
async fn a_comeback_past_the_cap_or_inside_the_gap_is_withheld() {
    let harness = Harness::new(at(DAY, 12, 0)).await;

    let decisions = [
        comeback(&harness, DAY, LAPSE).await,
        comeback(&harness, DAY + 1, LAPSE).await,
        comeback(&harness, DAY + 2, LAPSE).await,
        comeback(&harness, DAY + 3, LAPSE).await,
        comeback(&harness, DAY + 6, LAPSE).await,
        comeback(&harness, DAY + 9, LAPSE).await,
        comeback(&harness, DAY + 20, LAPSE).await,
    ];
    let next_lapse = comeback(&harness, DAY + 21, LapseContext::Open(day(DAY + 18))).await;

    assert_eq!(
        decisions,
        [SENT, SPENT, SPENT, SENT, SENT, SPENT, SPENT],
        "sent on the lapse's day 0, 3 and 6; inside the gap on 1 and 2; past the cap on 9 and 20"
    );
    assert_eq!(next_lapse, SENT, "a new lapse id is a new episode");
    assert_eq!(
        harness.bot.delivered().len(),
        4,
        "three comebacks for the first lapse, one for the next"
    );
    let reasons: Vec<Option<String>> = harness
        .decisions()
        .await
        .into_iter()
        .map(|decision| decision.reason)
        .collect();
    assert_eq!(
        reasons.iter().flatten().collect::<Vec<_>>(),
        ["budget_spent"; 4],
        "every withhold is the budget's"
    );
}

#[tokio::test]
async fn a_nudge_in_a_lapse_is_withheld_and_a_comeback_is_not() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let nudge = harness.occasion(
        "habit",
        "habit:check-in",
        Surface::Bot,
        Tier::T2,
        DAY,
        LAPSE,
    );
    let celebration = harness.occasion(
        "celebration",
        "badge:return",
        Surface::Bot,
        Tier::T2,
        DAY,
        LAPSE,
    );

    let nudged = harness.router.route(&nudge).await.expect("a decision");
    let returned = comeback(&harness, DAY, LAPSE).await;
    let celebrated = harness
        .router
        .route(&celebration)
        .await
        .expect("a decision");

    assert_eq!(
        (nudged, returned, celebrated),
        (
            Decision::Withheld {
                surface: Surface::Bot,
                reason: Reason::Lapse
            },
            SENT,
            SENT
        ),
        "a lapse suppresses nudges, not its comeback and not a celebration"
    );
    let kinds: Vec<String> = harness
        .decisions()
        .await
        .into_iter()
        .map(|decision| decision.kind)
        .collect();
    assert_eq!(kinds, ["habit:withheld", "comeback", "celebration"]);
}

#[tokio::test]
async fn a_comeback_switched_off_at_zero_is_withheld_as_disabled() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    harness.set("comeback_enabled", "0").await;

    let decision = comeback(&harness, DAY, LAPSE).await;

    assert_eq!(
        decision,
        Decision::Withheld {
            surface: Surface::Bot,
            reason: Reason::NudgesDisabled
        }
    );
    assert!(harness.bot.pushes().is_empty());
}

#[tokio::test]
async fn a_comeback_is_raised_only_inside_a_lapse() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let outside = Occasion::new(
        harness.policy.kind("comeback").expect("the comeback kind"),
        DedupeKey::new("comeback:reading").expect("a key"),
        Surface::Bot,
        Tier::T2,
        "synthetic",
        day(DAY),
        LapseContext::NoLapse,
    );

    assert_eq!(outside, Err(OccasionError::ComebackWithoutLapse));
}
