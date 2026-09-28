//! Deferral and the flush (SPEC-041 A4, A5, A11, A12; R7, R8): a celebration raised in quiet hours
//! waits for the first flush after the window, a hold past its age is abandoned by name, a failed
//! send is held and retried twice before it is abandoned, a flush renders two in full and rolls the
//! rest into one line, and the queue never holds more than 20. Every clock is a `ManualClock`.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

mod support;

use std::time::Duration;

use deck_streak_notifications::{Decision, Flushed, Hold, Reason, Surface};
use support::{DAY, Harness, Queued, at};

/// The deferral every quiet-hours celebration gets.
const QUIET: Decision = Decision::Deferred {
    surface: Surface::Bot,
    hold: Hold::Quiet,
};

#[tokio::test]
async fn a_celebration_raised_in_quiet_hours_is_delivered_when_the_window_ends() {
    let harness = Harness::new(at(DAY, 23, 30)).await;

    let decision = harness
        .router
        .route(&harness.celebration("level-up:8", Surface::Bot))
        .await
        .expect("a decision");
    harness.clock.set(at(DAY + 1, 7, 29));
    let at_7_29 = harness.router.flush().await.expect("a flush");
    let before = harness.bot.pushes();
    harness.clock.set(at(DAY + 1, 7, 30));
    let at_7_30 = harness.router.flush().await.expect("a flush");

    assert_eq!(decision, QUIET);
    assert_eq!(at_7_29, Flushed::QuietHours);
    assert!(before.is_empty(), "nothing is sent before 07:30");
    assert_eq!(at_7_30, Flushed::Ran { sends: 1 });
    assert_eq!(harness.bot.delivered(), ["synthetic level-up:8"]);
    assert!(harness.queue().await.is_empty(), "the queue let it go");
    let arms: Vec<(String, Option<String>, String)> = harness
        .decisions()
        .await
        .into_iter()
        .map(|decision| (decision.arm, decision.reason, decision.rendered))
        .collect();
    assert_eq!(
        arms,
        [
            (
                "defer".to_owned(),
                Some("quiet".to_owned()),
                "T0".to_owned()
            ),
            ("send".to_owned(), None, "T2".to_owned())
        ]
    );
}

#[tokio::test]
async fn a_deferred_celebration_older_than_720_minutes_is_abandoned_by_name() {
    let harness = Harness::new(at(DAY, 23, 0)).await;
    let decision = harness
        .router
        .route(&harness.celebration("badge:night-owl", Surface::Bot))
        .await
        .expect("a decision");

    harness.clock.set(at(DAY + 1, 11, 1));
    let flushed = harness.router.flush().await.expect("a flush");

    assert_eq!(decision, QUIET);
    assert_eq!(flushed, Flushed::Ran { sends: 1 });
    assert_eq!(
        harness.bot.delivered(),
        [
            "\u{1f319} <b>1 held celebration(s)</b> (quiet hours)\n\u{2022} badge:night-owl (expired, unseen)"
        ],
        "721 minutes after it was raised, it is named in the recap, never sent"
    );
    assert_eq!(
        harness.last_decision().await.kind,
        "celebration:withheld",
        "its abandonment is recorded"
    );
    assert_eq!(
        harness.last_decision().await.reason.as_deref(),
        Some("quiet_hours")
    );
    assert!(
        harness.queue().await.is_empty(),
        "named, it leaves the queue"
    );
}

#[tokio::test]
async fn a_deferred_celebration_of_exactly_720_minutes_is_delivered() {
    let harness = Harness::new(at(DAY, 23, 0)).await;
    let _decision = harness
        .router
        .route(&harness.celebration("badge:on-time", Surface::Bot))
        .await
        .expect("a decision");

    harness.clock.set(at(DAY + 1, 11, 0));
    let flushed = harness.router.flush().await.expect("a flush");

    assert_eq!(flushed, Flushed::Ran { sends: 1 });
    assert_eq!(harness.bot.delivered(), ["synthetic badge:on-time"]);
}

#[tokio::test]
async fn a_failed_send_is_held_and_retried_twice() {
    let harness = Harness::new(at(DAY, 12, 0)).await;
    harness.bot.fail(true);
    let text = "synthetic quest:weekly";

    let decision = harness
        .router
        .route(&harness.celebration("quest:weekly", Surface::Bot))
        .await
        .expect("a decision");
    let held = harness.queue().await;
    harness.clock.advance(Duration::from_secs(30));
    let breaker = harness.router.flush().await.expect("a flush");
    harness.clock.set(at(DAY, 12, 1));
    let first_retry = harness.router.flush().await.expect("a flush");
    let relatched = harness.queue().await;
    harness.clock.set(at(DAY, 12, 2));
    let second_retry = harness.router.flush().await.expect("a flush");
    let abandoned = harness.queue().await;
    harness.bot.fail(false);
    harness.clock.set(at(DAY, 12, 3));
    let recap = harness.router.flush().await.expect("a flush");

    let first_deferral = at(DAY, 12, 0).epoch_millis();
    let row = |state: &str, tries: i64| Queued {
        key: "quest:weekly".to_owned(),
        state: state.to_owned(),
        hold: "send".to_owned(),
        tries,
        deferred_at: first_deferral,
    };
    assert_eq!(
        decision,
        Decision::Deferred {
            surface: Surface::Bot,
            hold: Hold::Send
        }
    );
    assert_eq!(held, [row("held", 1)], "held with its first deferral time");
    assert_eq!(
        breaker,
        Flushed::BreakerOpen,
        "the breaker holds for 60 seconds"
    );
    assert_eq!(first_retry, Flushed::Ran { sends: 0 });
    assert_eq!(relatched, [row("held", 2)], "its first deferral time kept");
    assert_eq!(second_retry, Flushed::Ran { sends: 0 });
    assert_eq!(abandoned, [row("abandoned", 3)], "after its second retry");
    let attempts = harness
        .bot
        .pushes()
        .iter()
        .filter(|(pushed, _)| pushed == text)
        .count();
    assert_eq!(attempts, 3, "the send and two retries");
    assert_eq!(recap, Flushed::Ran { sends: 1 });
    assert_eq!(
        harness.bot.delivered(),
        [
            "\u{1f4e1} <b>1 held celebration(s)</b> (send failures)\n\u{2022} quest:weekly (gave up retrying, unseen)"
        ],
        "abandoned by name"
    );
    let last = harness.last_decision().await;
    assert_eq!(
        (last.kind.as_str(), last.reason.as_deref()),
        ("celebration:withheld", Some(Reason::NoNotifier.as_str()))
    );
    assert!(harness.queue().await.is_empty());
}

#[tokio::test]
async fn a_flush_renders_two_and_rolls_up_the_rest() {
    let harness = Harness::new(at(DAY, 23, 30)).await;
    let keys: Vec<String> = (1..=25).map(|n| format!("chest:{n:02}")).collect();
    let mut most_held = 0;
    for key in &keys {
        let decision = harness
            .router
            .route(&harness.celebration(key, Surface::Bot))
            .await
            .expect("a decision");
        assert_eq!(decision, QUIET);
        let held = harness
            .queue()
            .await
            .iter()
            .filter(|row| row.state == "held")
            .count();
        most_held = most_held.max(held);
    }

    harness.clock.set(at(DAY + 1, 8, 0));
    let flushed = harness.router.flush().await.expect("a flush");

    assert_eq!(most_held, 20, "the queue never holds more than 20");
    assert_eq!(flushed, Flushed::Ran { sends: 3 });
    let delivered = harness.bot.delivered();
    assert_eq!(
        delivered[..2],
        ["synthetic chest:01", "synthetic chest:02"],
        "two in full, oldest first"
    );
    let rolled: Vec<String> = (3..=20).map(|n| format!("\u{2022} chest:{n:02}")).collect();
    let dropped: Vec<String> = (21..=25)
        .map(|n| format!("\u{2022} chest:{n:02} (expired, unseen)"))
        .collect();
    let recap = format!(
        "\u{1f319} <b>23 held celebration(s)</b> (quiet hours)\n{}\n{}",
        rolled.join("\n"),
        dropped.join("\n")
    );
    assert_eq!(delivered[2..], [recap], "one line names every other one");
    assert!(harness.queue().await.is_empty());
}

#[tokio::test]
async fn a_failed_recap_holds_its_rolled_celebration_again() {
    let harness = Harness::new(at(DAY, 23, 30)).await;
    for key in ["gem:a", "gem:b", "gem:c"] {
        let _decision = harness
            .router
            .route(&harness.celebration(key, Surface::Bot))
            .await
            .expect("a decision");
    }
    harness.bot.deliver_then_fail(2);

    harness.clock.set(at(DAY + 1, 8, 0));
    let first = harness.router.flush().await.expect("a flush");
    let after_first = harness.queue().await;
    harness.bot.fail(false);
    harness.clock.set(at(DAY + 1, 8, 5));
    let second = harness.router.flush().await.expect("a flush");

    assert_eq!(
        first,
        Flushed::Ran { sends: 2 },
        "two in full, and the recap failed"
    );
    assert_eq!(
        after_first,
        [Queued {
            key: "gem:c".to_owned(),
            state: "held".to_owned(),
            hold: "send".to_owned(),
            tries: 1,
            deferred_at: at(DAY, 23, 30).epoch_millis(),
        }],
        "the rolled one is held again, with its first deferral time"
    );
    assert_eq!(second, Flushed::Ran { sends: 1 });
    assert_eq!(
        harness.bot.delivered().last().map(String::as_str),
        Some("synthetic gem:c"),
        "delivered in full by the next flush"
    );
    assert!(harness.queue().await.is_empty());
}

#[tokio::test]
async fn an_abandoned_celebration_waits_until_a_recap_names_it() {
    let harness = Harness::new(at(DAY, 23, 0)).await;
    let _decision = harness
        .router
        .route(&harness.celebration("badge:unseen", Surface::Bot))
        .await
        .expect("a decision");
    harness.bot.fail(true);

    harness.clock.set(at(DAY + 1, 11, 30));
    let failed = harness.router.flush().await.expect("a flush");
    let waiting = harness.queue().await;
    harness.bot.fail(false);
    harness.clock.set(at(DAY + 1, 11, 35));
    let named = harness.router.flush().await.expect("a flush");

    assert_eq!(failed, Flushed::Ran { sends: 0 }, "the recap failed");
    assert_eq!(
        waiting
            .iter()
            .map(|row| row.state.as_str())
            .collect::<Vec<_>>(),
        ["abandoned"],
        "abandoned, and not yet named"
    );
    assert_eq!(named, Flushed::Ran { sends: 1 });
    assert_eq!(
        harness.bot.delivered(),
        [
            "\u{1f319} <b>1 held celebration(s)</b> (quiet hours)\n\u{2022} badge:unseen (expired, unseen)"
        ]
    );
    assert!(harness.queue().await.is_empty());
}

#[tokio::test]
async fn a_flush_without_a_bot_transport_leaves_the_queue() {
    let harness = Harness::without_bot(at(DAY, 23, 30)).await;
    let decision = harness
        .router
        .route(&harness.celebration("record:best-day", Surface::Bot))
        .await
        .expect("a decision");

    harness.clock.set(at(DAY + 1, 8, 0));
    let flushed = harness.router.flush().await.expect("a flush");

    assert_eq!(decision, QUIET);
    assert_eq!(flushed, Flushed::NoNotifier);
    assert_eq!(harness.queue().await.len(), 1, "still held");
}

#[tokio::test]
async fn a_mini_app_celebration_held_in_quiet_hours_is_flushed_to_the_feed() {
    let harness = Harness::new(at(DAY, 23, 30)).await;
    let decision = harness
        .router
        .route(&harness.celebration("badge:late", Surface::MiniApp))
        .await
        .expect("a decision");

    harness.clock.set(at(DAY + 1, 8, 0));
    let flushed = harness.router.flush().await.expect("a flush");

    assert_eq!(
        decision,
        Decision::Deferred {
            surface: Surface::MiniApp,
            hold: Hold::Quiet
        }
    );
    assert_eq!(flushed, Flushed::Ran { sends: 1 });
    assert_eq!(harness.feed().await, ["synthetic badge:late"]);
    assert!(harness.bot.pushes().is_empty(), "the bot sent nothing");
}
