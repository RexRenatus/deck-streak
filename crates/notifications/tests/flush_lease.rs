//! The flush lease (SPEC-041 amendment, #291): two flushers over one held queue never send one
//! item twice, because a flush takes a lease before its first send and a flush that finds an
//! unlapsed one answers `Busy` and sends nothing. The lease lapses ten minutes after it was
//! taken, and is released when the flush ends. Every clock is a `ManualClock`.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

mod support;

use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_kernel::Db;
use deck_streak_notifications::{BotTransport, Flushed, Pass, PushFuture, Pushed, Surface};
use support::{DAY, Harness, MINUTE_MS, at};

/// The setting the lease is held in.
const FLUSH_LEASE_SETTING: &str = "flush_lease";

/// What the lease reads, as the setting's value, or `None` when no lease is held.
async fn lease(db: &Db) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT value FROM notification_settings WHERE key = ?")
        .bind(FLUSH_LEASE_SETTING)
        .fetch_optional(db.reader())
        .await
        .expect("the lease is read")
}

/// A bot that reads the lease at the moment of each send.
struct Probe {
    db: Db,
    seen: Mutex<Vec<Option<String>>>,
}

impl BotTransport for Probe {
    fn push_message<'a>(&'a self, _pass: &'a Pass, _text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            let held = lease(&self.db).await;
            self.seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(held);
            Pushed::Delivered
        })
    }
}

/// A harness with one celebration held from 23:30 and its clock at 10:00 the next day.
async fn held_at_ten() -> Harness {
    let harness = Harness::new(at(DAY, 23, 30)).await;
    let _decision = harness
        .router
        .route(&harness.celebration("level-up:8", Surface::Bot))
        .await
        .expect("a decision");
    harness.clock.set(at(DAY + 1, 10, 0));
    harness
}

#[tokio::test]
async fn a_flush_finds_an_unlapsed_lease_and_sends_nothing() {
    let harness = held_at_ten().await;
    let now = at(DAY + 1, 10, 0).epoch_millis();
    harness
        .set(FLUSH_LEASE_SETTING, &(now + 1).to_string())
        .await;

    let flushed = harness.router.flush().await.expect("a flush");

    assert_eq!(flushed, Flushed::Busy);
    assert!(
        harness.bot.pushes().is_empty(),
        "a busy flush sends nothing"
    );
    assert_eq!(
        harness.queue().await.len(),
        1,
        "the hold waits for the next flush"
    );
}

#[tokio::test]
async fn a_lease_that_lapsed_this_instant_is_taken_over() {
    let harness = held_at_ten().await;
    let now = at(DAY + 1, 10, 0).epoch_millis();
    harness.set(FLUSH_LEASE_SETTING, &now.to_string()).await;

    let flushed = harness.router.flush().await.expect("a flush");

    assert_eq!(flushed, Flushed::Ran { sends: 1 });
    assert_eq!(harness.bot.delivered(), ["synthetic level-up:8"]);
    assert_eq!(lease(&harness.db).await, None, "the takeover was released");
}

#[tokio::test]
async fn the_lease_holds_for_ten_minutes_and_is_released_when_the_flush_ends() {
    let harness = held_at_ten().await;
    let probe = Arc::new(Probe {
        db: harness.db.clone(),
        seen: Mutex::default(),
    });
    let router = deck_streak_notifications::Router::new(
        Arc::clone(&harness.policy),
        harness.db.clone(),
        harness.clock.clone(),
        deck_streak_kernel::StudyDayRule::default(),
    )
    .with_bot(probe.clone());
    let now = at(DAY + 1, 10, 0).epoch_millis();

    let flushed = router.flush().await.expect("a flush");

    assert_eq!(flushed, Flushed::Ran { sends: 1 });
    let seen = probe
        .seen
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert_eq!(
        seen,
        [Some((now + 10 * MINUTE_MS).to_string())],
        "the lease read ten minutes ahead at the send"
    );
    assert_eq!(lease(&harness.db).await, None, "the flush released it");
}
