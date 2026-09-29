//! What the router's tests share: a migrated temporary database, a manual clock, the compiled
//! policy, a bot transport that records every push and can be told to fail, occasions of the
//! policy's kinds, and readers of the router's tables. Every value is synthetic.

// Each test file uses a part of this module.
#![allow(dead_code, clippy::expect_used)]

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_kernel::{Db, ManualClock, StudyDay, StudyDayRule, UtcMillis};
use deck_streak_notifications::{
    BotTransport, DedupeKey, LapseContext, Occasion, Pass, Policy, PushFuture, Pushed, Router,
    Surface, Tier,
};
use sqlx::Row;
use tempfile::TempDir;

/// One minute, in milliseconds.
pub const MINUTE_MS: i64 = 60_000;
/// One day, in milliseconds.
pub const DAY_MS: i64 = 86_400_000;
/// A study day far from the epoch, as its epoch day number.
pub const DAY: i64 = 20_000;

/// The instant of `hour:minute` UTC on the calendar day `day`. The default rule's offset is UTC,
/// so it is that local time too.
pub fn at(day: i64, hour: i64, minute: i64) -> UtcMillis {
    UtcMillis::from_epoch_millis(day * DAY_MS + (hour * 60 + minute) * MINUTE_MS)
}

/// The study day `day`.
pub const fn day(day: i64) -> StudyDay {
    StudyDay::from_epoch_day(day)
}

/// A bot transport that records every push it is asked for, and delivers until told to fail.
#[derive(Debug)]
pub struct Recording {
    pushes: Mutex<Vec<(String, Pushed)>>,
    /// How many more pushes deliver; below zero, every one does.
    delivers: AtomicI64,
}

impl Default for Recording {
    fn default() -> Self {
        Self {
            pushes: Mutex::default(),
            delivers: AtomicI64::new(-1),
        }
    }
}

impl Recording {
    /// Every push asked for, in order, with what it came to.
    pub fn pushes(&self) -> Vec<(String, Pushed)> {
        self.pushes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The texts of every push that was delivered, in order.
    pub fn delivered(&self) -> Vec<String> {
        self.pushes()
            .into_iter()
            .filter(|(_, pushed)| *pushed == Pushed::Delivered)
            .map(|(text, _)| text)
            .collect()
    }

    /// Makes every later push fail, or deliver again.
    pub fn fail(&self, failing: bool) {
        self.delivers
            .store(if failing { 0 } else { -1 }, Ordering::SeqCst);
    }

    /// Delivers the next `count` pushes, and fails every one after them.
    pub fn deliver_then_fail(&self, count: i64) {
        self.delivers.store(count, Ordering::SeqCst);
    }
}

impl BotTransport for Recording {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            let left = self
                .delivers
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                    Some(if left > 0 { left - 1 } else { left })
                })
                .unwrap_or(-1);
            let pushed = if left == 0 {
                Pushed::Failed
            } else {
                Pushed::Delivered
            };
            self.pushes
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push((text.to_owned(), pushed));
            pushed
        })
    }
}

/// A decision as the ledger holds it: its key, kind, surface, arm, reason, and the tiers asked
/// for and rendered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
    pub key: String,
    pub kind: String,
    pub surface: String,
    pub arm: String,
    pub reason: Option<String>,
    pub requested: String,
    pub rendered: String,
}

/// A queue row as the ledger holds it: its key, state, hold, tries and first deferral time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Queued {
    pub key: String,
    pub state: String,
    pub hold: String,
    pub tries: i64,
    pub deferred_at: i64,
}

/// A router over a temporary database on a manual clock.
pub struct Harness {
    pub clock: Arc<ManualClock>,
    pub bot: Arc<Recording>,
    pub policy: Arc<Policy>,
    pub db: Db,
    pub router: Router,
    _directory: TempDir,
}

impl Harness {
    /// The router with the recording bot joined, its clock at `start`.
    pub async fn new(start: UtcMillis) -> Self {
        let mut harness = Self::without_bot(start).await;
        harness.router = Router::new(
            Arc::clone(&harness.policy),
            harness.db.clone(),
            harness.clock.clone(),
            StudyDayRule::default(),
        )
        .with_bot(harness.bot.clone());
        harness
    }

    /// The router with no bot transport, its clock at `start`.
    pub async fn without_bot(start: UtcMillis) -> Self {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let db = Db::open(&directory.path().join("deck_streak.db"))
            .await
            .expect("the database opens");
        let clock = Arc::new(ManualClock::new(start));
        let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
        let router = Router::new(
            Arc::clone(&policy),
            db.clone(),
            clock.clone(),
            StudyDayRule::default(),
        );
        Self {
            clock,
            bot: Arc::new(Recording::default()),
            policy,
            db,
            router,
            _directory: directory,
        }
    }

    /// An occasion of the declared `kind`, keyed `key`, raised on `origin` for `tier` on the study
    /// day `on`, in `lapse`; its text names its key.
    pub fn occasion(
        &self,
        kind: &str,
        key: &str,
        origin: Surface,
        tier: Tier,
        on: i64,
        lapse: LapseContext,
    ) -> Occasion {
        Occasion::new(
            self.policy.kind(kind).expect("a declared kind"),
            DedupeKey::new(key).expect("a key of the grammar"),
            origin,
            tier,
            format!("synthetic {key}"),
            day(on),
            lapse,
        )
        .expect("an occasion the kind admits")
    }

    /// A celebration at T2, raised on `origin`, keyed `key`, on the study day `DAY`.
    pub fn celebration(&self, key: &str, origin: Surface) -> Occasion {
        self.occasion(
            "celebration",
            key,
            origin,
            Tier::T2,
            DAY,
            LapseContext::NoLapse,
        )
    }

    /// Sets the owner's `key` to `value`.
    pub async fn set(&self, key: &str, value: &str) {
        sqlx::query(
            "INSERT INTO notification_settings (key, value, created_at) VALUES (?, ?, 0) \
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(self.db.reader())
        .await
        .expect("a setting is written");
    }

    /// Every decision, in the order recorded.
    pub async fn decisions(&self) -> Vec<Recorded> {
        sqlx::query(
            "SELECT dedupe_key, kind, surface, arm, reason, tier_requested, tier_rendered \
             FROM notification_decisions ORDER BY id",
        )
        .fetch_all(self.db.reader())
        .await
        .expect("the decisions are read")
        .into_iter()
        .map(|row| Recorded {
            key: row.get(0),
            kind: row.get(1),
            surface: row.get(2),
            arm: row.get(3),
            reason: row.get(4),
            requested: row.get(5),
            rendered: row.get(6),
        })
        .collect()
    }

    /// The last decision recorded.
    pub async fn last_decision(&self) -> Recorded {
        self.decisions().await.pop().expect("at least one decision")
    }

    /// Every row of the queue, in the order held.
    pub async fn queue(&self) -> Vec<Queued> {
        sqlx::query(
            "SELECT dedupe_key, state, hold, tries, deferred_at FROM notification_queue ORDER BY id",
        )
        .fetch_all(self.db.reader())
        .await
        .expect("the queue is read")
        .into_iter()
        .map(|row| Queued {
            key: row.get(0),
            state: row.get(1),
            hold: row.get(2),
            tries: row.get(3),
            deferred_at: row.get(4),
        })
        .collect()
    }

    /// How many deliveries claim a key.
    pub async fn deliveries(&self) -> i64 {
        sqlx::query("SELECT COUNT(*) FROM notification_deliveries")
            .fetch_one(self.db.reader())
            .await
            .expect("the deliveries are counted")
            .get(0)
    }

    /// The texts of the in-app feed, in the order appended.
    pub async fn feed(&self) -> Vec<String> {
        sqlx::query("SELECT text FROM in_app_feed ORDER BY id")
            .fetch_all(self.db.reader())
            .await
            .expect("the feed is read")
            .into_iter()
            .map(|row| row.get(0))
            .collect()
    }
}
