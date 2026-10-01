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

/// The state of the item on the queue, or `None` when it is no longer there.
async fn state_of(db: &Db) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT state FROM notification_queue WHERE dedupe_key = 'level-up:1'",
    )
    .fetch_optional(db.reader())
    .await
    .expect("the state is read")
}

#[tokio::test]
async fn a_row_another_flush_claimed_inside_its_claim_is_never_sent() {
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 0).await;
    let mut write = db.write().await.expect("a write");
    sqlx::query("UPDATE notification_queue SET state = 'sending', claim = ?")
        .bind((t0 + 5 * LEASE_MS).to_string())
        .execute(&mut *write)
        .await
        .expect("the row is claimed by a flush that is still inside its claim");
    write.commit().await.expect("committed");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let reached = Arc::new(Mutex::new(Vec::new()));
    let only = router(&db, &clock, Arc::new(Quick(Arc::clone(&reached))));
    let _answer = only.flush().await.expect("the flush");
    assert_eq!(
        times(&reached),
        0,
        "a claimed row is never read for sending"
    );
    assert_eq!(
        state_of(&db).await.as_deref(),
        Some("sending"),
        "its claimant still owns it"
    );
}

#[tokio::test]
async fn a_late_settle_by_a_flush_that_lost_its_claim_removes_nothing() {
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 0).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let reached = Arc::new(Mutex::new(Vec::new()));
    let slow = Slow::new(&reached, false, Pushed::Delivered);
    let running = stalled_flush(&db, &clock, &slow).await;
    clock.set(UtcMillis::from_epoch_millis(t0 + LEASE_MS + 1));
    // The second flush abandons the lapsed claim, and its recap fails, so the row stays to be named.
    let second = router(&db, &clock, Arc::new(Down));
    let _answer = second.flush().await.expect("the second flush");
    slow.release();
    running.await.expect("the first flush ended");
    // The first flush's own settle matches no claim and removes nothing; the row leaves the queue
    // only when a recap names it, and that recap reached the owner.
    assert_eq!(times(&reached), 1, "the item itself reached the owner once");
    assert_eq!(
        named(&reached).len(),
        1,
        "a recap named the row the late settle left behind: {:?}",
        named(&reached)
    );
    assert_eq!(
        state_of(&db).await,
        None,
        "the recap that named it settled it"
    );
}

#[tokio::test]
async fn a_flush_leaves_what_it_did_not_reach_held() {
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 0).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let only = router(&db, &clock, Arc::new(Down));
    let _answer = only.flush().await.expect("the flush");
    assert_eq!(
        state_of(&db).await.as_deref(),
        Some("held"),
        "a failed send holds the item again, its claim cleared"
    );
    let claim: Option<String> =
        sqlx::query_scalar("SELECT claim FROM notification_queue WHERE dedupe_key = 'level-up:1'")
            .fetch_one(db.reader())
            .await
            .expect("the claim is read");
    assert_eq!(claim, None);
}

/// The text of a second held item, held after the first.
const SECOND: &str = "synthetic second item";

/// Holds a second item on `db`'s queue, like the first and after it.
async fn hold_second(db: &Db, t0: i64) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(
        "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, tier_pending, \
         text, hold, tries, state, deferred_at, study_day, created_at) \
         VALUES ('celebration', 'level-up:2', 'bot', 'T2', 'T2', ?, 'quiet', 0, 'held', ?, ?, ?)",
    )
    .bind(SECOND)
    .bind(t0 - 60 * MINUTE_MS)
    .bind(DAY)
    .bind(t0 - 60 * MINUTE_MS)
    .execute(&mut *write)
    .await
    .expect("the second item is held");
    write.commit().await.expect("committed");
}

#[tokio::test]
async fn a_flush_ended_by_a_failed_send_gives_back_the_row_it_never_reached() {
    let t0 = noon();
    let (_scratch, db) = held_queue(t0, 0).await;
    hold_second(&db, t0).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    // The first send fails, which ends the flush before it reaches the second row it claimed.
    let failing = router(&db, &clock, Arc::new(Down));
    let _answer = failing.flush().await.expect("the failing flush");
    let second: (String, Option<String>) = sqlx::query_as(
        "SELECT state, claim FROM notification_queue WHERE dedupe_key = 'level-up:2'",
    )
    .fetch_one(db.reader())
    .await
    .expect("the second row is read");
    assert_eq!(
        second,
        ("held".to_owned(), None),
        "the row the flush never reached is held again, its claim cleared"
    );
    // A next flush inside the first one's claim takes the row back and sends it.
    clock.set(UtcMillis::from_epoch_millis(t0 + MINUTE_MS));
    let reached = Arc::new(Mutex::new(Vec::new()));
    let next = router(&db, &clock, Arc::new(Quick(Arc::clone(&reached))));
    let _answer = next.flush().await.expect("the next flush");
    let sent = reached
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .filter(|text| text.as_str() == SECOND)
        .count();
    assert_eq!(
        sent, 1,
        "the next flush sent the row the failed flush gave back"
    );
}

/// The names of the indexes and triggers a table has, from the schema.
async fn guards(db: &Db, table: &str) -> Vec<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE tbl_name = ? AND type IN ('index', 'trigger') \
         AND name NOT LIKE 'sqlite_autoindex%' ORDER BY name",
    )
    .bind(table)
    .fetch_all(db.reader())
    .await
    .expect("the schema is read")
}

/// The names `CREATE INDEX` and `CREATE TRIGGER` statements of `sql` give to objects on `table`.
fn declared(sql: &str, table: &str) -> Vec<String> {
    let mut names = Vec::new();
    let statements: Vec<&str> = sql.split(';').collect();
    for statement in statements {
        let flat = statement.split_whitespace().collect::<Vec<_>>().join(" ");
        let marker = format!(" ON {table}");
        let Some(at) = flat.find("CREATE ") else {
            continue;
        };
        let head = &flat[at..];
        if !head.contains(&marker) {
            continue;
        }
        let words: Vec<&str> = head.split(' ').collect();
        if let Some(kind) = words.iter().position(|w| *w == "INDEX" || *w == "TRIGGER")
            && let Some(name) = words.get(kind + 1)
        {
            names.push((*name).to_owned());
        }
    }
    names.sort();
    names
}

#[tokio::test]
async fn the_claim_migration_keeps_every_index_and_trigger_the_queue_had() {
    let migration = |name: &str| {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../migrations")
                .join(name),
        )
        .expect("the migration is read")
    };
    let first = migration("004101_notifications_router.sql");
    let rebuild = migration("004102_notifications_queue_claim.sql");
    let scratch = tempfile::tempdir().expect("a scratch");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens");
    // The reader finds the one index the first migration gave the deliveries, so an empty answer for
    // the queue is a measurement and not a reader that finds nothing.
    assert_eq!(
        declared(&first, "notification_deliveries"),
        vec!["notification_deliveries_scoped_key".to_owned()]
    );
    assert_eq!(
        guards(&db, "notification_deliveries").await,
        declared(&first, "notification_deliveries")
    );
    // What the queue had before the rebuild is what it has after it, and the rebuild declares each
    // one again after its rename.
    let before = declared(&first, "notification_queue");
    assert_eq!(guards(&db, "notification_queue").await, before);
    let recreated = declared(&rebuild, "notification_queue");
    assert!(
        before.iter().all(|name| recreated.contains(name)),
        "the rebuild re-creates {before:?}; it declares {recreated:?}"
    );
}
