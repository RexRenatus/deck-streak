//! The comeback's cap and the lapse (SPEC-041 A9, A10; R4 rules 1, 3 and 5): at most 3 comebacks for
//! one lapse id, at least 3 study days apart, then silence for the episode; a nudge in an open lapse
//! is withheld while a comeback in it is sent; a comeback is raised only inside a lapse, and the
//! owner switches it off with "0". Every clock is a `ManualClock`, at noon UTC outside quiet hours,
//! but for A9's two cases across the rollover hour, which run five hours west of UTC with the
//! owner's quiet window off, so that only the gap decides.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

mod support;

use std::sync::Arc;

use deck_streak_kernel::{StudyDayRule, UtcMillis, UtcOffset};
use deck_streak_notifications::{
    Decision, DedupeKey, LapseContext, Occasion, OccasionError, Reason, Router, Surface, Tier,
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

/// Five hours west of UTC, where a study day turns over at the default rollover hour, 04:00.
fn west() -> StudyDayRule {
    StudyDayRule::new(
        StudyDayRule::default().rollover_hour(),
        UtcOffset::from_minutes(-300).expect("an offset in bounds"),
    )
}

/// The instant of `hour:minute` local time, five hours west of UTC, on the calendar day `day`.
fn west_of_utc(day: i64, hour: i64, minute: i64) -> UtcMillis {
    at(day, hour + 5, minute)
}

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

    // Across the rollover hour: the gap counts the owner's study days, which the caller reads from
    // the clock through the rule, not calendar days. The owner's quiet window is off (its start
    // equals its end), since 03:00 and 05:00 are inside the default one, which withholds a nudge
    // before the gap is counted.
    let rollover = Harness::new(west_of_utc(DAY, 12, 0)).await;
    rollover.set("quiet_start_min", "0").await;
    rollover.set("quiet_end_min", "0").await;
    let router = Router::new(
        Arc::clone(&rollover.policy),
        rollover.db.clone(),
        rollover.clock.clone(),
        west(),
    )
    .with_bot(rollover.bot.clone());
    let mut across = Vec::new();
    for raised in [
        west_of_utc(DAY, 12, 0),
        west_of_utc(DAY + 3, 3, 0),
        west_of_utc(DAY + 3, 5, 0),
    ] {
        rollover.clock.set(raised);
        let on = west().study_day(raised).epoch_day();
        let occasion = rollover.occasion(
            "comeback",
            "comeback:reading",
            Surface::Bot,
            Tier::T2,
            on,
            LAPSE,
        );
        let decision = router.route(&occasion).await.expect("a decision");
        across.push((on - DAY, decision));
    }

    assert_eq!(
        across,
        [(0, SENT), (2, SPENT), (3, SENT)],
        "a comeback at noon on day 0; at 03:00 on the third calendar day, before the rollover, the \
         study day is 2 and inside the gap; at 05:00 it is 3 and past it"
    );

    // A comeback sent before the rollover hour is recorded on its study day, and the gap counts
    // from there: one at 03:00 on the calendar day after day 0 is still on study day 0, so one at
    // noon two calendar days later is on study day 3 and past the gap.
    let early = Harness::new(west_of_utc(DAY + 1, 3, 0)).await;
    early.set("quiet_start_min", "0").await;
    early.set("quiet_end_min", "0").await;
    let early_router = Router::new(
        Arc::clone(&early.policy),
        early.db.clone(),
        early.clock.clone(),
        west(),
    )
    .with_bot(early.bot.clone());
    let mut sent_early = Vec::new();
    for raised in [west_of_utc(DAY + 1, 3, 0), west_of_utc(DAY + 3, 12, 0)] {
        early.clock.set(raised);
        let on = west().study_day(raised).epoch_day();
        let occasion = early.occasion(
            "comeback",
            "comeback:reading",
            Surface::Bot,
            Tier::T2,
            on,
            LAPSE,
        );
        let decision = early_router.route(&occasion).await.expect("a decision");
        sent_early.push((on - DAY, decision));
    }

    assert_eq!(
        sent_early,
        [(0, SENT), (3, SENT)],
        "a comeback sent at 03:00 on the calendar day after day 0, before the rollover, is on study \
         day 0, so one at noon two calendar days later is 3 study days after it and past the gap"
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
