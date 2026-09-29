//! The ladder's renders on the one router (SPEC-084 A5, A7 to A9): each tier makes the transport
//! calls of the golden `celebration_tier` in its order, a streak-break day caps every celebration at
//! T1, a reaction is attempted only on a fresh owner message, and a refused reaction holds later
//! reactions without holding the other tiers. The router runs over a scripted transport on a
//! manual clock.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use std::time::Duration;

use deck_streak_kernel::{Clock, UtcMillis};
use deck_streak_notifications::ladder::{self, DICE_EMOJI, REACTION_EMOJI};
use deck_streak_notifications::occasion::StreakFacts;
use deck_streak_notifications::{
    Decision, DedupeKey, Hold, LapseContext, Occasion, Pushed, Surface, Tier,
};
use serde_json::Value;
use support::ladder::{
    Scripted, assert_calls, clear_owner_message, port_calls, queue_rows, script, scripted,
    seed_owner_message, seed_sent, tier,
};
use support::{DAY, Harness, MINUTE_MS, at, day};

/// The owner's message id every case seeds.
const MESSAGE_ID: i64 = 7;

/// The streak's facts on a day it broke: its last study day is that day, it is one day long, and
/// it was longer before.
const fn broke_on(on: i64) -> StreakFacts {
    StreakFacts {
        last_study_day: Some(day(on)),
        current: 1,
        longest: 2,
    }
}

/// A celebration of `event` with `rarity`, keyed `key`, on `DAY`, carrying `facts`.
fn celebration(
    harness: &Harness,
    key: &str,
    event: &str,
    rarity: Option<&str>,
    facts: Option<StreakFacts>,
) -> Occasion {
    let occasion = Occasion::new(
        harness.policy.kind("celebration").expect("a declared kind"),
        DedupeKey::new(key).expect("a key of the grammar"),
        Surface::Bot,
        Tier::T2,
        format!("synthetic {key}"),
        day(DAY),
        LapseContext::NoLapse,
    )
    .expect("an occasion the kind admits")
    .with_event(event, rarity);
    match facts {
        Some(facts) => occasion.with_streak(facts),
        None => occasion,
    }
}

/// Opens the reaction breaker: a T1 whose reaction is refused, then the calls forgotten.
async fn open_reaction_breaker(harness: &Harness, bot: &Scripted) {
    let now = harness.clock.now().epoch_millis();
    seed_owner_message(harness, MESSAGE_ID, now).await;
    bot.script("push_reaction", [Pushed::Failed]);
    let _opener = harness
        .router
        .route(&celebration(
            harness,
            "opener",
            "badge",
            None,
            Some(broke_on(DAY)),
        ))
        .await
        .expect("a decision");
    bot.clear();
}

/// The decision a golden case's output says the router makes, with the queue row it leaves.
fn wanted(output: &Value) -> (Decision, Option<(String, i64)>) {
    let latched = output["latched"].as_array().expect("a latched list");
    match latched.first() {
        Some(row) => {
            let hold = if row[2].as_str() == Some("send") {
                Hold::Send
            } else {
                Hold::Quiet
            };
            (
                Decision::Deferred {
                    surface: Surface::Bot,
                    hold,
                },
                Some((
                    tier(row[1].as_u64().expect("a tier")).as_str().to_owned(),
                    row[3].as_i64().unwrap_or(0),
                )),
            )
        }
        None => (
            Decision::Sent {
                surface: Surface::Bot,
                tier: tier(output["rendered"].as_u64().expect("a tier")),
            },
            None,
        ),
    }
}

#[tokio::test]
async fn each_tier_makes_the_transport_calls_of_the_parity_golden_in_order() {
    let mut cases = Vec::new();
    let examined = golden::each_case("celebration_tier", |case| {
        cases.push((case.input.clone(), case.output.clone()));
    });
    assert!(examined.count > 0);
    for (input, output) in cases {
        let start = at(DAY, 12, 0);
        let (harness, bot) = scripted(start).await;
        let event = input["event_type"].as_str().expect("an event");
        let rarity = input["rarity"].as_str();
        if let Some(intensity) = input["intensity"].as_str() {
            // the owner's intensity, under the key the predecessor's store and the v9 import use
            harness.set("celebration_intensity", intensity).await;
        }
        let at_or_above_5 = input["at_or_above_5"].as_u64().expect("a count");
        let at_or_above_4 = input["at_or_above_4"].as_u64().expect("a count");
        for used in 0..at_or_above_4 {
            let spent = if used < at_or_above_5 {
                Tier::T5
            } else {
                Tier::T4
            };
            seed_sent(&harness, &format!("spent-{used}"), spent, DAY).await;
        }
        if input["reaction_open"].as_bool() == Some(true) {
            open_reaction_breaker(&harness, &bot).await;
        }
        match input["owner"]["age_minutes"].as_i64() {
            Some(age) => {
                seed_owner_message(&harness, MESSAGE_ID, start.epoch_millis() - age * MINUTE_MS)
                    .await;
            }
            None => clear_owner_message(&harness).await,
        }
        let fail: Vec<String> = serde_json::from_value(input["fail"].clone()).expect("a list");
        let rendered = output["rendered"].as_u64().expect("a tier");
        let calls = output["calls"].as_array().expect("a call list");
        let expected = port_calls(calls, &fail, |_| rendered);
        script(&bot, &expected);
        let broke = input["broke"].as_bool() == Some(true);
        let key = format!("case-{event}");
        let occasion = celebration(&harness, &key, event, rarity, broke.then(|| broke_on(DAY)));
        let decision = harness.router.route(&occasion).await.expect("a decision");
        let context = format!("{input}");
        assert_calls(&bot, &expected, &context);
        let (decision_wanted, row_wanted) = wanted(&output);
        assert_eq!(decision, decision_wanted, "{context}: the decision");
        let held: Vec<(String, i64)> = queue_rows(&harness)
            .await
            .into_iter()
            .filter(|row| row.0 == key)
            .map(|row| (row.2, row.4))
            .collect();
        assert_eq!(
            held,
            row_wanted.into_iter().collect::<Vec<_>>(),
            "{context}: the held row's tier and failed sends"
        );
    }
}

#[tokio::test]
async fn on_a_streak_break_day_every_celebration_renders_at_most_t1() {
    let policy = deck_streak_notifications::Policy::compiled().expect("the policy parses");
    let examined = golden::each_case("outcome_cap", |case| {
        let broke = case.input["broke_today"].as_bool().expect("a flag");
        assert_eq!(
            ladder::outcome_cap(&policy, broke),
            tier(case.output.as_u64().expect("a tier")),
            "the cap of a day the streak broke: {broke}"
        );
    });
    assert!(examined.count > 0);

    let mut names = Vec::new();
    let constants = golden::each_case("ladder.constants", |case| {
        let name = case.input["name"].as_str().expect("a constant's name");
        if name.ends_with("_RARITY_TIER") || name.ends_with("_EVENT_TIER") {
            let map = case.output.as_object().expect("a tier map");
            names.push((
                name.ends_with("_RARITY_TIER"),
                map.keys().cloned().collect::<Vec<_>>(),
            ));
        }
    });
    assert!(constants.count > 0);
    let start = at(DAY, 12, 0);
    let (harness, bot) = scripted(start).await;
    seed_owner_message(&harness, MESSAGE_ID, start.epoch_millis()).await;
    let mut routed = 0;
    for (rarities, keys) in names {
        for name in keys {
            let (event, rarity) = if rarities {
                ("badge", Some(name.as_str()))
            } else {
                (name.as_str(), None)
            };
            let key = format!(
                "broke-{}-{}",
                event.replace('_', "-"),
                name.replace('_', "-")
            );
            let before = bot.calls().len();
            let decision = harness
                .router
                .route(&celebration(
                    &harness,
                    &key,
                    event,
                    rarity,
                    Some(broke_on(DAY)),
                ))
                .await
                .expect("a decision");
            assert_eq!(
                decision,
                Decision::Sent {
                    surface: Surface::Bot,
                    tier: Tier::T1
                },
                "{event} with rarity {rarity:?} on a streak-break day renders at T1"
            );
            let made: Vec<&str> = bot.calls()[before..].iter().map(|call| call.name).collect();
            assert_eq!(
                made,
                ["push_reaction"],
                "{event}, {rarity:?}: a reaction only"
            );
            routed += 1;
        }
    }
    assert!(routed > 4, "every rarity and every event was routed");
}

#[tokio::test]
async fn a_reaction_is_attempted_only_on_a_message_within_its_age() {
    let policy = deck_streak_notifications::Policy::compiled().expect("the policy parses");
    let mut cases = Vec::new();
    let examined = golden::each_case("reaction_freshness", |case| {
        cases.push((case.input.clone(), case.output.clone()));
    });
    assert!(examined.count > 0);
    for (input, output) in cases {
        let start = at(DAY, 12, 0);
        let (harness, bot) = scripted(start).await;
        let open = input["reaction_open"].as_bool() == Some(true);
        if open {
            open_reaction_breaker(&harness, &bot).await;
        }
        let age = input["owner"]["age_minutes"].as_i64();
        match age {
            Some(age) => {
                let arrived = start.epoch_millis() - age * MINUTE_MS;
                seed_owner_message(&harness, MESSAGE_ID, arrived).await;
                if !open {
                    assert_eq!(
                        ladder::reaction_fresh(
                            &policy,
                            UtcMillis::from_epoch_millis(arrived),
                            start
                        ),
                        !output["calls"].as_array().expect("calls").is_empty(),
                        "a message {age} minute(s) old"
                    );
                }
            }
            None => clear_owner_message(&harness).await,
        }
        let fail: Vec<String> = serde_json::from_value(input["fail"].clone()).expect("a list");
        let calls = output["calls"].as_array().expect("a call list");
        let expected = port_calls(calls, &fail, |_| 1);
        assert!(
            expected
                .iter()
                .all(|call| call.emoji.as_deref() == Some(REACTION_EMOJI)),
            "the golden's reaction is the router's emoji"
        );
        script(&bot, &expected);
        let decision = harness
            .router
            .route(&celebration(
                &harness,
                "fresh",
                "badge",
                None,
                Some(broke_on(DAY)),
            ))
            .await
            .expect("a decision");
        assert_calls(&bot, &expected, &format!("{input}"));
        let wanted = match output["outcome"].as_bool() {
            Some(true) => Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T1,
            },
            Some(false) => Decision::Deferred {
                surface: Surface::Bot,
                hold: Hold::Send,
            },
            None => Decision::Deferred {
                surface: Surface::Bot,
                hold: if open { Hold::Send } else { Hold::Quiet },
            },
        };
        assert_eq!(decision, wanted, "{input}: the decision");
    }
}

#[tokio::test]
async fn a_refused_reaction_holds_later_reactions_but_not_other_tiers() {
    let start = at(DAY, 12, 0);
    let (harness, bot) = scripted(start).await;
    seed_owner_message(&harness, MESSAGE_ID, start.epoch_millis()).await;
    bot.script("push_reaction", [Pushed::Failed]);
    let held = Decision::Deferred {
        surface: Surface::Bot,
        hold: Hold::Send,
    };
    let refused = harness
        .router
        .route(&celebration(
            &harness,
            "first",
            "badge",
            None,
            Some(broke_on(DAY)),
        ))
        .await
        .expect("a decision");
    assert_eq!(refused, held, "a refused reaction holds its celebration");
    let second = harness
        .router
        .route(&celebration(
            &harness,
            "second",
            "badge",
            None,
            Some(broke_on(DAY)),
        ))
        .await
        .expect("a decision");
    assert_eq!(second, held, "a reaction inside the cooldown is held");
    let names: Vec<&str> = bot.calls().iter().map(|call| call.name).collect();
    assert_eq!(names, ["push_reaction"], "no attempt inside the cooldown");

    let line = harness
        .router
        .route(&celebration(&harness, "line", "badge", None, None))
        .await
        .expect("a decision");
    assert_eq!(
        line,
        Decision::Sent {
            surface: Surface::Bot,
            tier: Tier::T2
        },
        "a T2 still sends while the reaction breaker is open"
    );
    let dice = harness
        .router
        .route(&celebration(&harness, "dice", "queue_zero", None, None))
        .await
        .expect("a decision");
    assert_eq!(
        dice,
        Decision::Sent {
            surface: Surface::Bot,
            tier: Tier::T4
        },
        "a T4 still sends while the reaction breaker is open"
    );
    let names: Vec<&str> = bot.calls().iter().map(|call| call.name).collect();
    assert_eq!(
        names,
        ["push_reaction", "push_message", "push_dice", "push_message"],
        "the other tiers make their calls"
    );
    assert_eq!(
        bot.calls()[2].emoji.as_deref(),
        Some(DICE_EMOJI),
        "the dice's emoji"
    );

    harness.clock.advance(Duration::from_millis(60_000));
    let third = harness
        .router
        .route(&celebration(
            &harness,
            "third",
            "badge",
            None,
            Some(broke_on(DAY)),
        ))
        .await
        .expect("a decision");
    assert_eq!(
        third,
        Decision::Sent {
            surface: Surface::Bot,
            tier: Tier::T1
        },
        "after the cooldown a reaction is attempted again"
    );
}
