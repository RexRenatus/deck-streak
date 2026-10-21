//! The week's count (SPEC-084 A6): celebrations delivered and still held at or above a tier since
//! the week's Monday, an abandoned hold releasing its slot, equal to the golden of the
//! predecessor's `celebrations_at_or_above`; and the router's budget spends a held one.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use deck_streak_notifications::{Decision, DedupeKey, LapseContext, Occasion, Surface, Tier};
use support::ladder::{HeldSeed, scripted, seed_held, seed_sent, tier};
use support::{DAY, Harness, at, day};

/// The Monday that starts `DAY`'s week: `DAY` is a Friday, since the epoch's first day was a
/// Thursday.
const MONDAY: i64 = DAY - 4;

/// A ceremony on `DAY`, keyed `next-trophy`, which asks for a T5.
fn next_trophy(harness: &Harness) -> Occasion {
    Occasion::new(
        harness.policy.kind("celebration").expect("a declared kind"),
        DedupeKey::new("next-trophy").expect("a key"),
        Surface::Bot,
        Tier::T2,
        "synthetic next-trophy",
        day(DAY),
        LapseContext::NoLapse,
    )
    .expect("an occasion")
    .with_event("ceremony", None)
}

#[tokio::test]
async fn the_week_counts_delivered_and_held_celebrations_as_the_parity_golden_does() {
    let mut cases = Vec::new();
    let examined = golden::each_case("celebration_week_count", |case| {
        cases.push((case.input.clone(), case.output.clone()));
    });
    assert!(examined.count > 0);
    for (input, output) in cases {
        let (harness, _bot) = scripted(at(DAY, 12, 0)).await;
        let rows = input["rows"].as_array().expect("a row list");
        for (at_row, row) in rows.iter().enumerate() {
            let key = format!("week-{at_row}");
            let on = row["day"].as_i64().expect("a day");
            let rank = tier(row["tier"].as_u64().expect("a tier"));
            let abandoned = row["abandoned"].as_bool() == Some(true);
            if row["held"].as_bool() == Some(true) {
                let seed = HeldSeed {
                    key: &key,
                    tier: rank,
                    hold: "quiet",
                    tries: 0,
                    state: if abandoned { "abandoned" } else { "held" },
                    deferred_at: 0,
                    study_day: on,
                };
                seed_held(&harness, &seed).await;
            } else {
                seed_sent(&harness, &key, rank, on).await;
            }
        }
        let since = input["since_day"].as_i64().expect("a day");
        let min = tier(input["min_tier"].as_u64().expect("a tier"));
        assert_eq!(
            harness
                .router
                .celebrations_at_or_above(day(since), min)
                .await
                .expect("the week is counted"),
            output.as_i64().expect("a count"),
            "{input}"
        );
    }

    // The router spends a held T5: the standard week allows one, so the next T5 steps to T4.
    let (harness, _bot) = scripted(at(DAY, 12, 0)).await;
    let seed = HeldSeed {
        key: "held-trophy",
        tier: Tier::T5,
        hold: "quiet",
        tries: 0,
        state: "held",
        deferred_at: 0,
        study_day: DAY,
    };
    seed_held(&harness, &seed).await;
    assert_eq!(
        harness
            .router
            .route(&next_trophy(&harness))
            .await
            .expect("a decision"),
        Decision::Sent {
            surface: Surface::Bot,
            tier: Tier::T4
        },
        "a held T5 spends the week's one T5"
    );

    // The router counts from the Monday of the occasion's week: a T5 delivered the Sunday before
    // spends nothing, and one delivered on the Monday spends the week's one T5.
    for (on, rendered, why) in [
        (
            MONDAY - 1,
            Tier::T5,
            "a T5 of the week before spends nothing",
        ),
        (
            MONDAY,
            Tier::T4,
            "a T5 on the week's Monday spends its one T5",
        ),
    ] {
        let (harness, _bot) = scripted(at(DAY, 12, 0)).await;
        seed_sent(&harness, "last-trophy", Tier::T5, on).await;
        assert_eq!(
            harness
                .router
                .route(&next_trophy(&harness))
                .await
                .expect("a decision"),
            Decision::Sent {
                surface: Surface::Bot,
                tier: rendered
            },
            "{why}"
        );
    }
}
