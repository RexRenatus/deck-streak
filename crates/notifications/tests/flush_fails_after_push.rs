//! A held item whose push reached the owner is never pushed again when the flush's work after the
//! push fails (SPEC-041 R16, ADR-300's amendment; #291).
//!
//! A flush settles each row it delivered in a write of its own: the row leaves the queue, and its
//! decision is recorded. When that write fails after the push answered delivered, the flush ends
//! with an error. The row it pushed must not go back to the queue for a later flush to push again:
//! it is named "may have been sent" in the next recap instead. A row the failed flush never pushed
//! goes back to the queue and reaches the owner at a later flush. The failure is a trigger that
//! aborts the settle's delete, standing in for any database error after a delivered push; it is
//! dropped before the second flush.

// An integration test is test code: its fixtures panic on a failed setup.
#![allow(clippy::expect_used)]

use std::sync::{Arc, Mutex, PoisonError};

use deck_streak_kernel::{Db, ManualClock, StudyDayRule, UtcMillis};
use deck_streak_notifications::{BotTransport, Pass, Policy, PushFuture, Pushed, Router};

/// One minute, in milliseconds.
const MINUTE_MS: i64 = 60_000;
/// One day, in milliseconds.
const DAY_MS: i64 = 86_400_000;
/// A study day near the present.
const DAY: i64 = 20_000;
/// The owner's latest message, which a held reaction answers.
const MESSAGE_ID: i64 = 4_242;

/// The settle of every delivered row fails once: its delete is aborted.
const SETTLE_FAULT: &str = "CREATE TRIGGER post_push_fault BEFORE DELETE ON notification_queue \
    BEGIN SELECT RAISE(ABORT, 'post-push fault'); END";
/// The same failure, on the third item's row alone: the item a recap line rolls up.
const RECAP_FAULT: &str = "CREATE TRIGGER post_push_fault BEFORE DELETE ON notification_queue \
    WHEN OLD.dedupe_key = 'level-up:3' BEGIN SELECT RAISE(ABORT, 'post-push fault'); END";
/// Lifts the failure before the second flush.
const LIFT: &str = "DROP TRIGGER post_push_fault";

/// A transport whose every push and reaction reaches the owner at once, logged in order.
struct Quick(Arc<Mutex<Vec<String>>>);

impl Quick {
    fn log(&self, entry: String) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(entry);
    }
}

impl BotTransport for Quick {
    fn push_message<'a>(&'a self, _pass: &'a Pass, text: &'a str) -> PushFuture<'a> {
        Box::pin(async move {
            self.log(text.to_owned());
            Pushed::Delivered
        })
    }

    fn push_reaction<'a>(
        &'a self,
        _pass: &'a Pass,
        message_id: i64,
        _emoji: &'a str,
    ) -> PushFuture<'a> {
        Box::pin(async move {
            self.log(format!("reaction:{message_id}"));
            Pushed::Delivered
        })
    }
}

/// Noon of the study day after `DAY`: the quiet window is closed.
const fn noon() -> i64 {
    (DAY + 1) * DAY_MS + 12 * 60 * MINUTE_MS
}

/// A fresh ledger holding `items` celebrations held an hour before `t0` at `tier`, keyed
/// `level-up:1` onwards, and the owner's latest message a minute before `t0`.
async fn queue(t0: i64, items: i64, tier: &str) -> (tempfile::TempDir, Db) {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let db = Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the ledger opens");
    let mut write = db.write().await.expect("a write");
    for k in 1..=items {
        sqlx::query(
            "INSERT INTO notification_queue (kind, dedupe_key, surface, tier_requested, \
             tier_pending, text, hold, tries, state, deferred_at, study_day, created_at) \
             VALUES ('celebration', ?, 'bot', ?, ?, ?, 'quiet', 0, 'held', ?, ?, ?)",
        )
        .bind(format!("level-up:{k}"))
        .bind(tier)
        .bind(tier)
        .bind(format!("held item {k}"))
        .bind(t0 - 60 * MINUTE_MS + k)
        .bind(DAY)
        .bind(t0 - 60 * MINUTE_MS + k)
        .execute(&mut *write)
        .await
        .expect("a held row");
    }
    write.commit().await.expect("the queue commits");
    deck_streak_notifications::owner_message::record(
        &db,
        MESSAGE_ID,
        UtcMillis::from_epoch_millis(t0 - MINUTE_MS),
    )
    .await
    .expect("the owner's message");
    (scratch, db)
}

async fn exec(db: &Db, sql: &'static str) {
    let mut write = db.write().await.expect("a write");
    sqlx::query(sql)
        .execute(&mut *write)
        .await
        .expect("the statement runs");
    write.commit().await.expect("it commits");
}

fn router(db: &Db, clock: &Arc<ManualClock>, reached: &Arc<Mutex<Vec<String>>>) -> Router {
    let policy = Arc::new(Policy::compiled().expect("the compiled policy"));
    Router::new(policy, db.clone(), clock.clone(), StudyDayRule::default())
        .with_bot(Arc::new(Quick(Arc::clone(reached))))
}

/// What two flushes, a minute apart, left behind: every push in order, and the queue's rows
/// between the flushes.
struct Run {
    pushes: Vec<String>,
    between: Vec<(String, String)>,
}

impl Run {
    /// Deliveries of item `k`: its own full render, plus every recap line that rolls it up.
    fn deliveries(&self, k: i64) -> usize {
        let own = format!("held item {k}");
        let rolled = format!("\u{2022} level-up:{k}");
        self.pushes
            .iter()
            .map(|text| usize::from(*text == own) + text.lines().filter(|l| *l == rolled).count())
            .sum()
    }

    /// How many reactions reached the owner.
    fn reactions(&self) -> usize {
        let reaction = format!("reaction:{MESSAGE_ID}");
        self.pushes.iter().filter(|text| **text == reaction).count()
    }

    /// Whether a recap line named item `k` as "may have been sent".
    fn named(&self, k: i64) -> bool {
        let line = format!("\u{2022} level-up:{k} (may have been sent, unseen)");
        self.pushes
            .iter()
            .any(|text| text.lines().any(|l| l == line))
    }
}

/// Holds `items` celebrations at `tier`, plants `fault` (if any) for the first flush, lifts it,
/// and flushes again a minute later.
async fn run(fault: Option<&'static str>, items: i64, tier: &str) -> Run {
    let t0 = noon();
    let (_scratch, db) = queue(t0, items, tier).await;
    if let Some(trigger) = fault {
        exec(&db, trigger).await;
    }
    let reached: Arc<Mutex<Vec<String>>> = Arc::default();
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(t0)));
    let first = router(&db, &clock, &reached).flush().await;
    println!("first flush answered {first:?}");
    if fault.is_some() {
        exec(&db, LIFT).await;
    }
    let between: Vec<(String, String)> =
        sqlx::query_as("SELECT dedupe_key, state FROM notification_queue ORDER BY id")
            .fetch_all(db.reader())
            .await
            .expect("the queue reads");
    println!("rows between the flushes: {between:?}");
    clock.set(UtcMillis::from_epoch_millis(t0 + MINUTE_MS));
    let second = router(&db, &clock, &reached).flush().await;
    println!("second flush answered {second:?}");
    let pushes = reached
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    println!("every push: {pushes:?}");
    Run { pushes, between }
}

#[tokio::test]
async fn control_no_fault_a_full_render_reaches_once() {
    let run = run(None, 1, "T2").await;
    let n = run.deliveries(1);
    assert_eq!(n, 1, "control: the held item reached the owner {n} times");
}

#[tokio::test]
async fn control_no_fault_a_rolled_item_reaches_once() {
    let run = run(None, 3, "T2").await;
    let n = run.deliveries(3);
    assert_eq!(n, 1, "control: the rolled item reached the owner {n} times");
}

#[tokio::test]
async fn control_no_fault_a_held_reaction_reacts_once() {
    let run = run(None, 1, "T1").await;
    let n = run.reactions();
    assert_eq!(
        n, 1,
        "control: the held reaction reached the owner {n} times"
    );
}

#[tokio::test]
async fn a_settle_that_fails_after_a_full_render_is_not_followed_by_a_second_push() {
    let run = run(Some(SETTLE_FAULT), 1, "T2").await;
    let n = run.deliveries(1);
    assert_eq!(n, 1, "the held item reached the owner {n} times");
    assert!(
        run.named(1),
        "the item whose settle failed is named: {:?}",
        run.pushes
    );
}

#[tokio::test]
async fn a_settle_that_fails_after_a_recap_is_not_followed_by_a_second_delivery() {
    let run = run(Some(RECAP_FAULT), 3, "T2").await;
    let n = run.deliveries(3);
    assert_eq!(n, 1, "the rolled item reached the owner {n} times");
    assert!(
        run.named(3),
        "the rolled item whose settle failed is named: {:?}",
        run.pushes
    );
}

#[tokio::test]
async fn a_settle_that_fails_after_a_reaction_is_not_followed_by_a_second_reaction() {
    let run = run(Some(SETTLE_FAULT), 1, "T1").await;
    let n = run.reactions();
    assert_eq!(n, 1, "the held reaction reached the owner {n} times");
    assert!(
        run.named(1),
        "the reaction whose settle failed is named: {:?}",
        run.pushes
    );
}

#[tokio::test]
async fn a_row_the_failed_flush_never_pushed_reaches_the_owner_at_a_later_flush() {
    let run = run(Some(SETTLE_FAULT), 2, "T2").await;
    let pushed = run.deliveries(1);
    assert_eq!(
        pushed, 1,
        "the pushed item reached the owner {pushed} times"
    );
    assert_eq!(
        run.between,
        [
            ("level-up:1".to_owned(), "abandoned".to_owned()),
            ("level-up:2".to_owned(), "held".to_owned()),
        ],
        "the pushed row is named and only the unpushed one goes back to the queue"
    );
    let unpushed = run.deliveries(2);
    assert_eq!(
        unpushed, 1,
        "the unpushed item reached the owner {unpushed} times"
    );
    assert!(run.named(1), "the pushed item is named: {:?}", run.pushes);
}
