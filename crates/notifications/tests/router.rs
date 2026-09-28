//! The router's decision in the policy's order (SPEC-041 A6, A7; R1, R4, R5, R6): the kind's switch,
//! the claim of a key across both surfaces, the quiet window by class, the transport, the tier
//! rendered, and the surface each occasion goes to. Every clock is a `ManualClock`; the default
//! rule reads local time as UTC, so `at(day, 23, 30)` is 23:30 local.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

mod support;

use std::sync::Arc;
use std::time::Duration;

use deck_streak_kernel::{StudyDayRule, UtcOffset};
use deck_streak_notifications::{Decision, Hold, LapseContext, Reason, Router, Surface, Tier};
use support::{DAY, Harness, Recorded, at};

/// A decision the ledger should hold.
fn recorded(key: &str, kind: &str, surface: &str, arm: &str, reason: Option<&str>) -> Recorded {
    Recorded {
        key: key.to_owned(),
        kind: kind.to_owned(),
        surface: surface.to_owned(),
        arm: arm.to_owned(),
        reason: reason.map(str::to_owned),
        requested: "T2".to_owned(),
        rendered: if arm == "send" { "T2" } else { "T0" }.to_owned(),
    }
}

#[tokio::test]
async fn a_nudge_in_quiet_hours_is_withheld_and_recorded() {
    let harness = Harness::new(at(DAY, 23, 30)).await;
    let nudge = harness.occasion(
        "habit",
        "habit:check-in",
        Surface::Bot,
        Tier::T2,
        DAY,
        LapseContext::NoLapse,
    );

    let decision = harness
        .router
        .route(&nudge)
        .await
        .expect("the router decides");

    assert_eq!(
        decision,
        Decision::Withheld {
            surface: Surface::Bot,
            reason: Reason::QuietHours
        }
    );
    assert_eq!(
        harness.decisions().await,
        [recorded(
            "habit:check-in",
            "habit:withheld",
            "bot",
            "withhold",
            Some("quiet_hours")
        )],
        "recorded under its kind with :withheld, so it never stands for a delivery"
    );
    assert!(harness.bot.pushes().is_empty(), "nothing was sent");
    assert_eq!(harness.deliveries().await, 0, "the key stays unclaimed");
}

#[tokio::test]
async fn the_same_event_from_both_surfaces_is_delivered_once() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let from_the_bot = harness.celebration("level-up:7", Surface::Bot);
    let from_the_app = harness.celebration("level-up:7", Surface::MiniApp);

    let first = harness
        .router
        .route(&from_the_bot)
        .await
        .expect("a decision");
    let second = harness
        .router
        .route(&from_the_app)
        .await
        .expect("a decision");

    assert_eq!(
        (first, second),
        (
            Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T2
            },
            Decision::Withheld {
                surface: Surface::MiniApp,
                reason: Reason::AlreadyRecorded
            }
        )
    );
    assert_eq!(
        harness.decisions().await,
        [
            recorded("level-up:7", "celebration", "bot", "send", None),
            recorded(
                "level-up:7",
                "celebration:withheld",
                "mini-app",
                "withhold",
                Some("already_recorded")
            ),
        ]
    );
    assert_eq!(harness.bot.delivered(), ["synthetic level-up:7"]);
    assert!(harness.feed().await.is_empty(), "the Mini App got nothing");
    assert_eq!(harness.deliveries().await, 1, "one delivery holds the key");
}

#[tokio::test]
async fn an_event_raised_in_the_mini_app_first_is_not_sent_to_the_bot() {
    let harness = Harness::new(at(DAY, 12, 0)).await;

    let first = harness
        .router
        .route(&harness.celebration("badge:first", Surface::MiniApp))
        .await
        .expect("a decision");
    let second = harness
        .router
        .route(&harness.celebration("badge:first", Surface::Bot))
        .await
        .expect("a decision");

    assert_eq!(
        (first, second),
        (
            Decision::Sent {
                surface: Surface::MiniApp,
                tier: Tier::T2
            },
            Decision::Withheld {
                surface: Surface::Bot,
                reason: Reason::AlreadyRecorded
            }
        )
    );
    assert_eq!(harness.feed().await, ["synthetic badge:first"]);
    assert!(harness.bot.pushes().is_empty(), "the bot sent nothing");
}

#[tokio::test]
async fn a_kind_switched_off_is_withheld_and_one_switched_on_is_sent() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let nudge = |key: &str| {
        harness.occasion(
            "habit",
            key,
            Surface::Bot,
            Tier::T2,
            DAY,
            LapseContext::NoLapse,
        )
    };

    harness.set("habit_enabled", "0").await;
    let off = harness
        .router
        .route(&nudge("habit:a"))
        .await
        .expect("a decision");
    harness.set("habit_enabled", "1").await;
    let on = harness
        .router
        .route(&nudge("habit:b"))
        .await
        .expect("a decision");

    assert_eq!(
        (off, on),
        (
            Decision::Withheld {
                surface: Surface::Bot,
                reason: Reason::NudgesDisabled
            },
            Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T2
            }
        )
    );
    assert_eq!(harness.bot.delivered(), ["synthetic habit:b"]);
}

#[tokio::test]
async fn an_alert_is_sent_in_quiet_hours_and_a_digest_is_withheld() {
    let harness = Harness::new(at(DAY, 23, 30)).await;
    let alert = harness.occasion(
        "alert",
        "alert:sync-stopped",
        Surface::Bot,
        Tier::T2,
        DAY,
        LapseContext::NoLapse,
    );
    let digest = harness.occasion(
        "digest",
        "digest:daily",
        Surface::Bot,
        Tier::T2,
        DAY,
        LapseContext::NoLapse,
    );

    let alerted = harness.router.route(&alert).await.expect("a decision");
    let digested = harness.router.route(&digest).await.expect("a decision");

    assert_eq!(
        (alerted, digested),
        (
            Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T2
            },
            Decision::Withheld {
                surface: Surface::Bot,
                reason: Reason::QuietHours
            }
        )
    );
    assert_eq!(harness.bot.delivered(), ["synthetic alert:sync-stopped"]);
}

#[tokio::test]
async fn a_bot_occasion_with_no_transport_is_withheld_and_its_key_released() {
    let harness = Harness::without_bot(at(DAY, 12, 0)).await;

    let decision = harness
        .router
        .route(&harness.celebration("record:streak", Surface::Bot))
        .await
        .expect("a decision");

    assert_eq!(
        decision,
        Decision::Withheld {
            surface: Surface::Bot,
            reason: Reason::NoNotifier
        }
    );
    assert_eq!(harness.deliveries().await, 0, "the claim was released");
    assert!(harness.queue().await.is_empty(), "nothing was held");
}

#[tokio::test]
async fn a_failed_nudge_is_withheld_its_key_released_and_the_breaker_waits_60_seconds() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let nudge = |key: &str| {
        harness.occasion(
            "habit",
            key,
            Surface::Bot,
            Tier::T2,
            DAY,
            LapseContext::NoLapse,
        )
    };
    harness.bot.fail(true);

    let failed = harness
        .router
        .route(&nudge("habit:a"))
        .await
        .expect("a decision");
    harness.bot.fail(false);
    harness.clock.advance(Duration::from_millis(59_999));
    let inside = harness
        .router
        .route(&nudge("habit:b"))
        .await
        .expect("a decision");
    harness.clock.advance(Duration::from_millis(1));
    let after = harness
        .router
        .route(&nudge("habit:a"))
        .await
        .expect("a decision");

    let withheld = Decision::Withheld {
        surface: Surface::Bot,
        reason: Reason::NoNotifier,
    };
    assert_eq!((failed, inside), (withheld, withheld));
    assert_eq!(
        after,
        Decision::Sent {
            surface: Surface::Bot,
            tier: Tier::T2
        },
        "the released key is sent once the breaker closes"
    );
    assert_eq!(
        harness.bot.pushes().len(),
        2,
        "no push was tried while the breaker was open"
    );
    assert_eq!(harness.deliveries().await, 1);
}

#[tokio::test]
async fn a_per_study_day_key_is_delivered_once_each_study_day() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let nudge = |on: i64| {
        harness.occasion(
            "habit",
            "habit:daily",
            Surface::Bot,
            Tier::T2,
            on,
            LapseContext::NoLapse,
        )
    };

    let first = harness.router.route(&nudge(DAY)).await.expect("a decision");
    let again = harness.router.route(&nudge(DAY)).await.expect("a decision");
    let next = harness
        .router
        .route(&nudge(DAY + 1))
        .await
        .expect("a decision");

    let sent = Decision::Sent {
        surface: Surface::Bot,
        tier: Tier::T2,
    };
    assert_eq!(
        (first, again, next),
        (
            sent,
            Decision::Withheld {
                surface: Surface::Bot,
                reason: Reason::AlreadyRecorded
            },
            sent
        )
    );
}

#[tokio::test]
async fn a_celebration_is_delivered_once_ever() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let on = |day: i64| {
        harness.occasion(
            "celebration",
            "season:final",
            Surface::Bot,
            Tier::T2,
            day,
            LapseContext::NoLapse,
        )
    };

    let first = harness.router.route(&on(DAY)).await.expect("a decision");
    let later = harness
        .router
        .route(&on(DAY + 30))
        .await
        .expect("a decision");

    assert_eq!(
        later,
        Decision::Withheld {
            surface: Surface::Bot,
            reason: Reason::AlreadyRecorded
        },
        "after {first:?}"
    );
}

#[tokio::test]
async fn a_celebration_renders_as_a_line_or_as_nothing_at_t0() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let asking = |key: &str, tier: Tier| {
        harness.occasion(
            "celebration",
            key,
            Surface::Bot,
            tier,
            DAY,
            LapseContext::NoLapse,
        )
    };

    let silent = harness
        .router
        .route(&asking("quiet:one", Tier::T0))
        .await
        .expect("a decision");
    let reaction = harness
        .router
        .route(&asking("small:one", Tier::T1))
        .await
        .expect("a decision");
    let trophy = harness
        .router
        .route(&asking("big:one", Tier::T5))
        .await
        .expect("a decision");

    assert_eq!(
        (silent, reaction, trophy),
        (
            Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T0
            },
            Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T2
            },
            Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T2
            }
        )
    );
    assert_eq!(
        harness.bot.delivered(),
        ["synthetic small:one", "synthetic big:one"],
        "T0 sends nothing"
    );
    let tiers: Vec<(String, String)> = harness
        .decisions()
        .await
        .into_iter()
        .map(|decision| (decision.requested, decision.rendered))
        .collect();
    assert_eq!(
        tiers,
        [
            ("T0".to_owned(), "T0".to_owned()),
            ("T1".to_owned(), "T2".to_owned()),
            ("T5".to_owned(), "T2".to_owned())
        ]
    );
}

#[tokio::test]
async fn a_nudge_raised_in_the_mini_app_goes_to_the_bot() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    let nudge = harness.occasion(
        "habit",
        "habit:from-app",
        Surface::MiniApp,
        Tier::T2,
        DAY,
        LapseContext::NoLapse,
    );

    let decision = harness.router.route(&nudge).await.expect("a decision");

    assert_eq!(
        decision,
        Decision::Sent {
            surface: Surface::Bot,
            tier: Tier::T2
        }
    );
    assert!(harness.feed().await.is_empty());
}

#[tokio::test]
async fn the_quiet_window_follows_the_owners_settings() {
    let harness = Harness::new(at(DAY, 21, 0)).await;
    let nudge = |key: &str| {
        harness.occasion(
            "habit",
            key,
            Surface::Bot,
            Tier::T2,
            DAY,
            LapseContext::NoLapse,
        )
    };

    let before = harness
        .router
        .route(&nudge("habit:a"))
        .await
        .expect("a decision");
    harness.set("quiet_start_min", "1200").await;
    harness.set("quiet_end_min", "360").await;
    let inside = harness
        .router
        .route(&nudge("habit:b"))
        .await
        .expect("a decision");
    harness.set("quiet_start_min", "not-a-number").await;
    let unreadable = harness
        .router
        .route(&nudge("habit:c"))
        .await
        .expect("a decision");

    assert_eq!(
        (before, inside, unreadable),
        (
            Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T2
            },
            Decision::Withheld {
                surface: Surface::Bot,
                reason: Reason::QuietHours
            },
            Decision::Sent {
                surface: Surface::Bot,
                tier: Tier::T2
            }
        ),
        "21:00 is outside 23:00 to 07:30, inside the owner's 20:00 to 06:00, and outside once the \
         start is unreadable and falls back to 23:00"
    );
}

#[tokio::test]
async fn the_quiet_window_is_read_at_the_configured_offset() {
    let harness = Harness::new(at(DAY, 21, 30)).await;
    let two_hours_east = StudyDayRule::new(
        StudyDayRule::default().rollover_hour(),
        UtcOffset::from_minutes(120).expect("an offset in bounds"),
    );
    let router = Router::new(
        Arc::clone(&harness.policy),
        harness.db.clone(),
        harness.clock.clone(),
        two_hours_east,
    )
    .with_bot(harness.bot.clone());
    let nudge = harness.occasion(
        "habit",
        "habit:east",
        Surface::Bot,
        Tier::T2,
        DAY,
        LapseContext::NoLapse,
    );

    let decision = router.route(&nudge).await.expect("a decision");

    assert_eq!(
        decision,
        Decision::Withheld {
            surface: Surface::Bot,
            reason: Reason::QuietHours
        },
        "21:30 UTC is 23:30 two hours east"
    );
}

#[tokio::test]
async fn a_celebration_is_deferred_while_the_breaker_is_open() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    harness.bot.fail(true);
    let failed = harness
        .router
        .route(&harness.celebration("quest:all", Surface::Bot))
        .await
        .expect("a decision");
    harness.bot.fail(false);

    let held = harness
        .router
        .route(&harness.celebration("quest:one", Surface::Bot))
        .await
        .expect("a decision");

    let deferred = Decision::Deferred {
        surface: Surface::Bot,
        hold: Hold::Send,
    };
    assert_eq!((failed, held), (deferred, deferred));
    assert_eq!(harness.bot.pushes().len(), 1, "the breaker made no attempt");
    let tries: Vec<i64> = harness
        .queue()
        .await
        .into_iter()
        .map(|row| row.tries)
        .collect();
    assert_eq!(
        tries,
        [1, 0],
        "the failed send is one try, the breaker's hold none"
    );
}

#[test]
fn a_dedupe_key_is_an_opaque_token_that_holds_no_calendar_date() {
    use deck_streak_notifications::{DedupeKey, OccasionError};

    let longest = "k".repeat(128);
    let too_long = "k".repeat(129);
    let tokens = [
        "level-up:7",
        "0",
        "a.b_c-d:e",
        "day:20000",
        "2025",
        "1.2.3",
        "12.34",
        "t-19-99",
        "2025-13-01",
        "2025-00-10",
        "2025-12-32",
        "2025-10-00",
        "1899-01-01",
        "2100-01-01",
        "2025-1-15",
        longest.as_str(),
    ];
    let dates = [
        "2025-01-15",
        "20250115",
        "review:2025-1231",
        "199912-31",
        "1.2.25",
        "level:12.1.2025",
        "1.12.99",
        "12.1.09",
    ];
    let not_tokens = [
        "",
        "Level-up",
        "-lead",
        "a b",
        "a/b",
        "caf\u{e9}",
        too_long.as_str(),
    ];

    let judged: Vec<(&str, Result<(), OccasionError>)> = tokens
        .iter()
        .chain(&dates)
        .chain(&not_tokens)
        .map(|text| {
            (
                *text,
                DedupeKey::new(text).map(|key| assert_eq!(key.as_str(), *text)),
            )
        })
        .collect();

    let expected: Vec<(&str, Result<(), OccasionError>)> = tokens
        .iter()
        .map(|text| (*text, Ok(())))
        .chain(
            dates
                .iter()
                .map(|text| (*text, Err(OccasionError::KeyHoldsDate))),
        )
        .chain(
            not_tokens
                .iter()
                .map(|text| (*text, Err(OccasionError::KeyNotAToken))),
        )
        .collect();
    assert_eq!(judged, expected);
}

#[tokio::test]
async fn the_router_shows_its_rule_and_whether_a_bot_is_joined() {
    let with = Harness::new(at(DAY, 12, 0)).await;
    let without = Harness::without_bot(at(DAY, 12, 0)).await;

    let shown = (
        format!("{:?}", with.router),
        format!("{:?}", without.router),
    );

    assert!(
        shown.0.starts_with("Router { rule: StudyDayRule"),
        "{}",
        shown.0
    );
    assert!(shown.0.ends_with("bot: true, .. }"), "{}", shown.0);
    assert!(shown.1.ends_with("bot: false, .. }"), "{}", shown.1);
}
