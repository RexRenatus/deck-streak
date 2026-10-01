//! A held item reaches the owner at most once, and is never lost without a name (SPEC-041 R16,
//! ADR-300; #291).
//!
//! The lease keeps two flushers off one queue, but it lapses after ten minutes while its holder may
//! still be sending, and a flusher can die between a push that reached the owner and the settle of
//! its row. So a second flush must neither resend a row another flush took, nor lose it without a
//! name: a claimed row whose claim has lapsed is abandoned by name, "may have been sent", and never
//! pushed again. Every clock is a `ManualClock`; a flusher's stall and its death are the transport
//! that blocks and the task that is aborted.

// An integration test is test code: its fixtures panic on a failed setup, and the log capture is
// the one a test of this workspace makes.
#![allow(clippy::expect_used)]

use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_kernel::{Db, ManualClock, StudyDayRule, UtcMillis};
use deck_streak_notifications::{BotTransport, Pass, Policy, PushFuture, Pushed, Router};

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

/// One minute, in milliseconds.
const MINUTE_MS: i64 = 60_000;
/// One day, in milliseconds.
const DAY_MS: i64 = 86_400_000;
/// A study day near the present.
const DAY: i64 = 20_000;
/// How long a flush's lease lasts.
const LEASE_MS: i64 = 10 * MINUTE_MS;
/// The text of the one held item.
const ITEM: &str = "synthetic item";

/// A transport whose first push blocks until released. `reach_first` says whether that push has
/// already reached the owner when it blocks (a send whose answer is slow) or not yet; `answer` is
/// what it answers once released.
struct Slow {
    reached: Arc<Mutex<Vec<String>>>,
    pushes: AtomicUsize,
    reach_first: bool,
    answer: Pushed,
    entered: AtomicBool,
    released: AtomicBool,
}

impl Slow {
    fn new(reached: &Arc<Mutex<Vec<String>>>, reach_first: bool, answer: Pushed) -> Arc<Self> {
        Arc::new(Self {
            reached: Arc::clone(reached),
            pushes: AtomicUsize::new(0),
            reach_first,
            answer,
            entered: AtomicBool::new(false),
            released: AtomicBool::new(false),
        })
    }

    async fn stalled(&self) {
        while !self.entered.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    }

    fn release(&self) {
        self.released.store(true, Ordering::SeqCst);
    }
}

/// What reached the owner, in order.
fn arrive(reached: &Arc<Mutex<Vec<String>>>, text: &str) {
    reached
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(text.to_owned());
}

impl BotTransport for Slow {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            if self.pushes.fetch_add(1, Ordering::SeqCst) == 0 {
                if self.reach_first {
                    arrive(&self.reached, text);
                }
                self.entered.store(true, Ordering::SeqCst);
                while !self.released.load(Ordering::SeqCst) {
                    tokio::task::yield_now().await;
                }
                if !self.reach_first && self.answer == Pushed::Delivered {
                    arrive(&self.reached, text);
                }
                return self.answer;
            }
            arrive(&self.reached, text);
            Pushed::Delivered
        })
    }
}

/// A transport that delivers every push.
struct Quick(Arc<Mutex<Vec<String>>>);

impl BotTransport for Quick {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            arrive(&self.0, text);
            Pushed::Delivered
        })
    }
}

/// A transport whose every push fails.
struct Down;

impl BotTransport for Down {
    fn push_message<'a>(&'a self, _pass: &'a Pass, _text: &'a str) -> PushFuture<'a> {
        Box::pin(async { Pushed::Failed })
    }
}

/// A queue holding one item, held an hour before `t0`, with `tries` failed sends behind it.
async fn held_queue(t0: i64, tries: i64) -> (tempfile::TempDir, Db) {
    let scratch = tempfile::tempdir().expect("a scratch");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, tier_pending, \
         text, hold, tries, state, deferred_at, study_day, created_at) \
         VALUES ('celebration', 'level-up:1', 'bot', 'T2', 'T2', ?, 'quiet', ?, 'held', ?, ?, ?)",
    )
    .bind(ITEM)
    .bind(tries)
    .bind(t0 - 60 * MINUTE_MS)
    .bind(DAY)
    .bind(t0 - 60 * MINUTE_MS)
    .execute(&mut *write)
    .await
    .expect("the item is held");
    write.commit().await.expect("committed");
    (scratch, db)
}

fn router(db: &Db, clock: &Arc<ManualClock>, bot: Arc<dyn BotTransport>) -> Arc<Router> {
    let policy = Arc::new(Policy::compiled().expect("the compiled policy parses"));
    Arc::new(Router::new(policy, db.clone(), clock.clone(), StudyDayRule::default()).with_bot(bot))
}

/// How many times the item itself reached the owner.
fn times(reached: &Arc<Mutex<Vec<String>>>) -> usize {
    reached
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .filter(|text| text.as_str() == ITEM)
        .count()
}

/// The lines of every recap that reached the owner and name the item.
fn named(reached: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    reached
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .flat_map(|text| text.lines().map(str::to_owned).collect::<Vec<_>>())
        .filter(|line| line.contains("level-up:1"))
        .collect()
}

/// The noon of the study day after `DAY`.
const fn noon() -> i64 {
    (DAY + 1) * DAY_MS + 12 * 60 * MINUTE_MS
}

/// Starts a flush that stalls in its first push, and returns once it has.
async fn stalled_flush(
    db: &Db,
    clock: &Arc<ManualClock>,
    slow: &Arc<Slow>,
) -> tokio::task::JoinHandle<()> {
    let first = router(db, clock, slow.clone());
    let running = tokio::spawn(async move {
        let _flushed = first.flush().await.expect("the first flush");
    });
    slow.stalled().await;
    running
}

#[tokio::test]
async fn a_flush_that_outlives_its_lease_is_never_doubled_by_a_second_flush() {
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 0).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let reached = Arc::new(Mutex::new(Vec::new()));
    let slow = Slow::new(&reached, false, Pushed::Delivered);
    let running = stalled_flush(&db, &clock, &slow).await;
    clock.set(UtcMillis::from_epoch_millis(t0 + LEASE_MS + 1));
    let second = router(&db, &clock, Arc::new(Quick(Arc::clone(&reached))));
    let _answer = second.flush().await.expect("the second flush");
    slow.release();
    running.await.expect("the first flush ended");
    assert_eq!(
        times(&reached),
        1,
        "the held item reached the owner {} times",
        times(&reached)
    );
}

#[tokio::test]
async fn a_flush_inside_its_lease_leaves_the_item_to_the_first() {
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 0).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let reached = Arc::new(Mutex::new(Vec::new()));
    let slow = Slow::new(&reached, false, Pushed::Delivered);
    let running = stalled_flush(&db, &clock, &slow).await;
    clock.set(UtcMillis::from_epoch_millis(t0 + LEASE_MS - 1));
    let second = router(&db, &clock, Arc::new(Quick(Arc::clone(&reached))));
    let answer = second.flush().await.expect("the second flush");
    slow.release();
    running.await.expect("the first flush ended");
    assert_eq!(answer, deck_streak_notifications::Flushed::Busy);
    assert_eq!(times(&reached), 1, "the item reaches the owner once");
}

#[tokio::test]
async fn a_flush_that_dies_after_its_send_reached_is_never_resent() {
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 0).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let reached = Arc::new(Mutex::new(Vec::new()));
    let slow = Slow::new(&reached, true, Pushed::Delivered);
    let running = stalled_flush(&db, &clock, &slow).await;
    running.abort();
    let _died = running.await;
    clock.set(UtcMillis::from_epoch_millis(t0 + 2 * 60 * MINUTE_MS));
    let later = router(&db, &clock, Arc::new(Quick(Arc::clone(&reached))));
    let _answer = later.flush().await.expect("the later flush");
    assert_eq!(
        times(&reached),
        1,
        "the held item reached the owner {} times",
        times(&reached)
    );
    let lines = named(&reached);
    assert!(
        lines.iter().any(|line| line.contains("may have been sent")),
        "the item whose fate is unknown is named: {lines:?}"
    );
}

/// The warnings the router logs, one line per event, its fields spelled `name=value`.
#[derive(Clone, Default)]
struct Warnings(Arc<Mutex<Vec<String>>>);

impl Warnings {
    fn logged(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Spells an event's fields into one line.
struct Spelled(String);

impl tracing::field::Visit for Spelled {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let _spelled = write!(self.0, "{}={value:?} ", field.name());
    }
}

impl tracing::Subscriber for Warnings {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        if *event.metadata().level() == tracing::Level::WARN
            && event
                .metadata()
                .target()
                .starts_with("deck_streak_notifications")
        {
            let mut line = Spelled(String::new());
            event.record(&mut line);
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(line.0);
        }
    }

    fn enter(&self, _span: &tracing::span::Id) {}

    fn exit(&self, _span: &tracing::span::Id) {}
}

#[tokio::test]
async fn a_lapsed_claim_is_abandoned_by_name_in_the_log() {
    let warnings = Warnings::default();
    let _logging = log_capture::hold_capture(warnings.clone());
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 0).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let reached = Arc::new(Mutex::new(Vec::new()));
    let slow = Slow::new(&reached, true, Pushed::Delivered);
    let running = stalled_flush(&db, &clock, &slow).await;
    running.abort();
    let _died = running.await;
    clock.set(UtcMillis::from_epoch_millis(t0 + 2 * LEASE_MS));
    let later = router(&db, &clock, Arc::new(Quick(Arc::clone(&reached))));
    let _answer = later.flush().await.expect("the later flush");

    let claimant = (t0 + LEASE_MS).to_string();
    let lines: Vec<String> = warnings
        .logged()
        .into_iter()
        .filter(|line| line.contains("level-up:1"))
        .collect();
    assert_eq!(lines.len(), 1, "one line names the item: {lines:?}");
    assert!(
        lines[0].contains("may have been sent"),
        "it says why: {lines:?}"
    );
    assert!(
        lines[0].contains(&claimant),
        "it names the claimant {claimant}: {lines:?}"
    );
}

#[tokio::test]
async fn a_failed_send_that_spent_its_retries_is_abandoned_by_name_in_the_log() {
    let warnings = Warnings::default();
    let _logging = log_capture::hold_capture(warnings.clone());
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 99).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let only = router(&db, &clock, Arc::new(Down));
    let _answer = only.flush().await.expect("the flush");

    let claimant = (t0 + LEASE_MS).to_string();
    let lines: Vec<String> = warnings
        .logged()
        .into_iter()
        .filter(|line| line.contains("level-up:1"))
        .collect();
    assert_eq!(lines.len(), 1, "one line names the item: {lines:?}");
    assert!(lines[0].contains("send failed"), "it says why: {lines:?}");
    assert!(
        lines[0].contains(&claimant),
        "it names the claimant {claimant}: {lines:?}"
    );
}

/// What the lease reads, as the setting's value, or `None` when no lease is held.
async fn lease(db: &Db) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT value FROM notification_settings WHERE key = 'flush_lease'",
    )
    .fetch_optional(db.reader())
    .await
    .expect("the lease is read")
}

#[tokio::test]
async fn a_flush_that_ends_after_its_lease_lapsed_leaves_the_next_holders_lease() {
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 0).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let reached = Arc::new(Mutex::new(Vec::new()));
    let first = Slow::new(&reached, false, Pushed::Delivered);
    let running = stalled_flush(&db, &clock, &first).await;
    clock.set(UtcMillis::from_epoch_millis(t0 + LEASE_MS + 1));
    let next = Slow::new(&reached, false, Pushed::Delivered);
    let taking = stalled_flush(&db, &clock, &next).await;
    first.release();
    running.await.expect("the first flush ended");
    assert_eq!(
        lease(&db).await,
        Some((t0 + 2 * LEASE_MS + 1).to_string()),
        "the first flush released the lease of the flush that took over"
    );
    next.release();
    taking.await.expect("the second flush ended");
    assert_eq!(lease(&db).await, None, "the second flush released its own");
}
