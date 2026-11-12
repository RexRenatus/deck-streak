//! What the scheduled flush job answers for each thing the router's flush can come to (SPEC-041
//! amendment, #291): a ran flush, a closed window and a lease held elsewhere are done; a breaker
//! that is open is a send attempted with nothing delivered; a router with no bot is a named
//! refusal. Every clock is a `ManualClock`, every value is synthetic.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_coordination::held_flush::HeldFlushWork;
use deck_streak_coordination::jobs::{FireDate, HELD_FLUSH};
use deck_streak_coordination::runner::{Done, Fire, Reason, Work};
use deck_streak_kernel::{Clock, Db, ManualClock, StudyDayRule, UtcMillis};
use deck_streak_notifications::{BotTransport, Pass, Policy, PushFuture, Pushed, Router};

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
const DAY: i64 = 20_000;

/// A bot whose every push fails.
#[derive(Default)]
struct Failing {
    tried: Mutex<i64>,
}

impl BotTransport for Failing {
    fn push_message<'a>(&'a self, _pass: &'a Pass, _text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            *self.tried.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            Pushed::Failed
        })
    }
}

struct World {
    db: Db,
    clock: Arc<ManualClock>,
    router: Router,
    _scratch: tempfile::TempDir,
}

impl World {
    async fn new(bot: Option<Arc<Failing>>, minute_of_day: i64) -> Self {
        let scratch = tempfile::tempdir().expect("a scratch");
        let db = Db::open(&scratch.path().join("deckstreak.db"))
            .await
            .expect("the database");
        let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
            (DAY + 1) * DAY_MS + minute_of_day * MINUTE_MS,
        )));
        let router = Router::new(
            Arc::new(Policy::compiled().expect("the compiled policy parses")),
            db.clone(),
            clock.clone(),
            StudyDayRule::default(),
        );
        let router = match bot {
            Some(bot) => router.with_bot(bot),
            None => router,
        };
        let mut write = db.write().await.expect("a write");
        let held_at = (DAY + 1) * DAY_MS + minute_of_day * MINUTE_MS - 60 * MINUTE_MS;
        sqlx::query(
            "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, \
             tier_pending, text, hold, tries, state, deferred_at, study_day, created_at) \
             VALUES ('celebration', 'level-up:9', 'bot', 'T2', 'T2', 'synthetic level-up', \
             'quiet', 0, 'held', ?, ?, ?)",
        )
        .bind(held_at)
        .bind(DAY)
        .bind(held_at)
        .execute(&mut *write)
        .await
        .expect("a held row");
        write.commit().await.expect("committed");
        Self {
            db,
            clock,
            router,
            _scratch: scratch,
        }
    }

    async fn answer(&self) -> Result<Done, Reason> {
        let at = self.clock.now();
        let rule = StudyDayRule::default();
        let fire = Fire {
            job: HELD_FLUSH,
            fire_date: FireDate::of(at, rule.utc_offset()),
            scheduled_at: at,
            started_at: at,
            study_day: rule.study_day(at),
            sync: None,
        };
        HeldFlushWork::new(&self.router).perform(&fire).await
    }
}

#[tokio::test]
async fn a_flush_that_ran_and_one_inside_the_window_are_done() {
    let open = World::new(Some(Arc::default()), 10 * 60).await;
    let closed = World::new(Some(Arc::default()), 2 * 60).await;

    assert_eq!(open.answer().await, Ok(Done::Done));
    assert_eq!(closed.answer().await, Ok(Done::Done));
}

#[tokio::test]
async fn a_flush_whose_lease_is_held_elsewhere_is_done() {
    let world = World::new(Some(Arc::default()), 10 * 60).await;
    let lapses = world.clock.now().epoch_millis() + MINUTE_MS;
    sqlx::query(
        "INSERT INTO notification_settings (key, value, created_at) VALUES ('flush_lease', ?, 0)",
    )
    .bind(lapses.to_string())
    .execute(world.db.reader())
    .await
    .expect("a lease");

    assert_eq!(world.answer().await, Ok(Done::Done));
}

#[tokio::test]
async fn an_open_breaker_is_a_send_attempted_and_nothing_delivered() {
    let bot = Arc::new(Failing::default());
    let world = World::new(Some(bot.clone()), 10 * 60).await;

    let first = world.answer().await;
    let second = world.answer().await;

    assert_eq!(first, Ok(Done::Done), "the failed send is retried later");
    assert_eq!(second, Ok(Done::NotDelivered), "the breaker waits");
    assert_eq!(
        *bot.tried.lock().unwrap_or_else(PoisonError::into_inner),
        1,
        "the open breaker made no attempt"
    );
}

#[tokio::test]
async fn a_router_with_no_bot_is_a_named_refusal() {
    let world = World::new(None, 10 * 60).await;

    assert_eq!(world.answer().await, Err(Reason::new("no_notifier")));
}
